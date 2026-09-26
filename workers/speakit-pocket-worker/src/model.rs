//! Pocket TTS inference: a port of Kyutai's reference implementation
//! (github.com/kyutai-labs/pocket-tts, `pocket_tts/models` and `modules`)
//! restricted to what reading with a predefined voice needs. Voice cloning
//! (the Mimi encoder) is not included.
//!
//! Pipeline for one chunk of text: the voice's precomputed key/value cache
//! primes the FlowLM transformer, the text tokens are appended, then the
//! model emits one 32-dim latent per 80 ms frame until its end-of-speech
//! head fires. Each latent goes through the flow head (one LSD step) and
//! the streaming Mimi decoder turns latents into 24 kHz audio.

use candle_core::safetensors::BufferedSafetensors;
use candle_core::{DType, Device, IndexOp, Module, Result, Tensor, D};
use candle_nn::LayerNorm;

/// Mimi runs its transformer at 200 Hz: 16 steps per latent frame.
const MIMI_STEPS_PER_FRAME: usize = 16;
const TOKENS_PER_SECOND_ESTIMATE: f64 = 3.0;
const GEN_SECONDS_PADDING: f64 = 2.0;
const FRAME_RATE: f64 = 12.5;
/// EOS is ignored on the first frames: before speech starts, the EOS logit
/// of some voices can cross the threshold.
const MIN_FRAMES_BEFORE_EOS: usize = 6;
const MIMI_CONTEXT: usize = 250;

pub struct Weights {
    st: BufferedSafetensors,
    dev: Device,
}

impl Weights {
    pub fn read(path: &std::path::Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(Self { st: BufferedSafetensors::new(bytes)?, dev: Device::Cpu })
    }

    /// Loads a tensor as f32 (the released files store bf16).
    fn get(&self, name: &str) -> Result<Tensor> {
        self.st.load(name, &self.dev)?.to_dtype(DType::F32)
    }

    fn has(&self, name: &str) -> bool {
        self.st.get(name).is_ok()
    }

    fn names(&self) -> Vec<String> {
        self.st.tensors().into_iter().map(|(n, _)| n).collect()
    }
}

struct Linear {
    w: Tensor,
    b: Option<Tensor>,
}

impl Linear {
    fn load(w: &Weights, prefix: &str, bias: bool) -> Result<Self> {
        Ok(Self {
            w: w.get(&format!("{prefix}.weight"))?,
            b: if bias { Some(w.get(&format!("{prefix}.bias"))?) } else { None },
        })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let mut dims = x.dims().to_vec();
        let last = *dims.last().unwrap();
        let y = x.reshape(((), last))?.matmul(&self.w.t()?)?;
        let y = match &self.b {
            Some(b) => y.broadcast_add(b)?,
            None => y,
        };
        *dims.last_mut().unwrap() = self.w.dim(0)?;
        y.reshape(dims)
    }
}

fn layer_norm(w: &Weights, prefix: &str, eps: f64) -> Result<LayerNorm> {
    Ok(LayerNorm::new(w.get(&format!("{prefix}.weight"))?, w.get(&format!("{prefix}.bias"))?, eps))
}

fn layer_norm_plain(dim: usize, eps: f64) -> Result<LayerNorm> {
    Ok(LayerNorm::new(Tensor::ones(dim, DType::F32, &Device::Cpu)?, Tensor::zeros(dim, DType::F32, &Device::Cpu)?, eps))
}

/// The flow head's RMSNorm: scales by the *unbiased* variance and does not
/// remove the mean, as in `pocket_tts/modules/mlp.py`.
fn rms_norm(x: &Tensor, alpha: &Tensor, eps: f64) -> Result<Tensor> {
    let n = x.dim(D::Minus1)?;
    let mean = x.mean_keepdim(D::Minus1)?;
    let var = (x.broadcast_sub(&mean)?.sqr()?.sum_keepdim(D::Minus1)? / (n as f64 - 1.0))?;
    x.broadcast_mul(&(var + eps)?.sqrt()?.recip()?)?.broadcast_mul(alpha)
}

/// Rotary position embedding on interleaved pairs.
struct Rope {
    freqs: Vec<f32>,
}

impl Rope {
    fn new(head_dim: usize, max_period: f32) -> Self {
        let freqs = (0..head_dim / 2)
            .map(|i| (i as f32 * (-(max_period.ln()) * 2.0 / head_dim as f32)).exp())
            .collect();
        Self { freqs }
    }

    fn apply(&self, q: &Tensor, k: &Tensor, offset: usize) -> Result<(Tensor, Tensor)> {
        let t = q.dim(2)?;
        let half = self.freqs.len();
        let mut cos = Vec::with_capacity(t * half);
        let mut sin = Vec::with_capacity(t * half);
        for i in 0..t {
            let ts = (offset + i) as f32;
            for f in &self.freqs {
                let a = f * ts;
                cos.push(a.cos());
                sin.push(a.sin());
            }
        }
        let cos = Tensor::from_vec(cos, (t, half), &Device::Cpu)?;
        let sin = Tensor::from_vec(sin, (t, half), &Device::Cpu)?;
        Ok((candle_nn::rotary_emb::rope_i(q, &cos, &sin)?, candle_nn::rotary_emb::rope_i(k, &cos, &sin)?))
    }
}

/// Keys and values of one attention layer, `[1, heads, len, head_dim]`,
/// holding absolute positions `offset - len .. offset`.
#[derive(Clone)]
pub struct KvCache {
    k: Option<Tensor>,
    v: Option<Tensor>,
    offset: usize,
}

impl KvCache {
    fn empty() -> Self {
        Self { k: None, v: None, offset: 0 }
    }
}

struct Attention {
    in_proj: Linear,
    out_proj: Linear,
    heads: usize,
    context: Option<usize>,
}

impl Attention {
    fn forward(&self, x: &Tensor, cache: &mut KvCache, rope: &Rope) -> Result<Tensor> {
        let (b, t, e) = x.dims3()?;
        let d = e / self.heads;
        let qkv = self.in_proj.forward(x)?.reshape((b, t, 3, self.heads, d))?;
        let part = |i: usize| qkv.i((.., .., i))?.transpose(1, 2)?.contiguous();
        let (q, k) = rope.apply(&part(0)?, &part(1)?, cache.offset)?;
        let v = part(2)?;
        let k = match &cache.k {
            Some(prev) => Tensor::cat(&[prev, &k], 2)?,
            None => k,
        };
        let v = match &cache.v {
            Some(prev) => Tensor::cat(&[prev, &v], 2)?,
            None => v,
        };
        let s = k.dim(2)?;

        let scores = (q.matmul(&k.t()?)? * (1.0 / (d as f64).sqrt()))?;
        // Key j sits at absolute position `offset + t - s + j`, query i at
        // `offset + i`: causal, and within `context` steps when limited.
        let first_key = cache.offset + t - s;
        let limited = self.context.is_some_and(|c| s > c);
        let scores = if t > 1 || limited {
            let mut mask = vec![0f32; t * s];
            for i in 0..t {
                let pq = cache.offset + i;
                for j in 0..s {
                    let pk = first_key + j;
                    let visible = pk <= pq && self.context.is_none_or(|c| pq - pk < c);
                    if !visible {
                        mask[i * s + j] = f32::NEG_INFINITY;
                    }
                }
            }
            scores.broadcast_add(&Tensor::from_vec(mask, (t, s), &Device::Cpu)?)?
        } else {
            scores
        };
        let probs = candle_nn::ops::softmax_last_dim(&scores)?;
        let y = probs.matmul(&v)?.transpose(1, 2)?.reshape((b, t, e))?;

        cache.offset += t;
        // Older keys can never be seen again once past the context window.
        let (k, v) = match self.context {
            Some(c) if s > c => (k.narrow(2, s - c, c)?, v.narrow(2, s - c, c)?),
            _ => (k, v),
        };
        cache.k = Some(k);
        cache.v = Some(v);
        self.out_proj.forward(&y)
    }
}

struct Layer {
    norm1: LayerNorm,
    norm2: LayerNorm,
    attn: Attention,
    linear1: Linear,
    linear2: Linear,
    scale1: Option<Tensor>,
    scale2: Option<Tensor>,
}

impl Layer {
    fn load(w: &Weights, p: &str, heads: usize, context: Option<usize>, layer_scale: bool) -> Result<Self> {
        let scale = |n: usize| -> Result<Option<Tensor>> {
            if layer_scale {
                Some(w.get(&format!("{p}.layer_scale_{n}.scale"))).transpose()
            } else {
                Ok(None)
            }
        };
        Ok(Self {
            norm1: layer_norm(w, &format!("{p}.norm1"), 1e-5)?,
            norm2: layer_norm(w, &format!("{p}.norm2"), 1e-5)?,
            attn: Attention {
                in_proj: Linear::load(w, &format!("{p}.self_attn.in_proj"), false)?,
                out_proj: Linear::load(w, &format!("{p}.self_attn.out_proj"), false)?,
                heads,
                context,
            },
            linear1: Linear::load(w, &format!("{p}.linear1"), false)?,
            linear2: Linear::load(w, &format!("{p}.linear2"), false)?,
            scale1: scale(1)?,
            scale2: scale(2)?,
        })
    }

    fn forward(&self, x: &Tensor, cache: &mut KvCache, rope: &Rope) -> Result<Tensor> {
        let scaled = |s: &Option<Tensor>, u: Tensor| match s {
            Some(s) => u.broadcast_mul(s),
            None => Ok(u),
        };
        let u = self.attn.forward(&self.norm1.forward(x)?, cache, rope)?;
        let x = (x + scaled(&self.scale1, u)?)?;
        let u = self.linear2.forward(&self.linear1.forward(&self.norm2.forward(&x)?)?.gelu()?)?;
        &x + scaled(&self.scale2, u)?
    }
}

fn load_layers(w: &Weights, prefix: &str, heads: usize, context: Option<usize>, layer_scale: bool) -> Result<Vec<Layer>> {
    let mut layers = Vec::new();
    while w.has(&format!("{prefix}.{}.norm1.weight", layers.len())) {
        layers.push(Layer::load(w, &format!("{prefix}.{}", layers.len()), heads, context, layer_scale)?);
    }
    if layers.is_empty() {
        candle_core::bail!("no transformer layers under {prefix}");
    }
    Ok(layers)
}

struct TimeEmbed {
    freqs: Tensor,
    l0: Linear,
    l2: Linear,
    alpha: Tensor,
}

impl TimeEmbed {
    fn forward(&self, t: f64) -> Result<Tensor> {
        let args = (&self.freqs * t)?.unsqueeze(0)?;
        let emb = Tensor::cat(&[args.cos()?, args.sin()?], 1)?;
        let h = self.l2.forward(&self.l0.forward(&emb)?.silu()?)?;
        rms_norm(&h, &self.alpha, 1e-5)
    }
}

struct ResBlock {
    in_ln: LayerNorm,
    mlp0: Linear,
    mlp2: Linear,
    ada: Linear,
}

/// `SimpleMLPAdaLN`: the flow head that turns noise into a latent,
/// conditioned on the transformer output and two times (LSD).
struct FlowNet {
    time: Vec<TimeEmbed>,
    cond_embed: Linear,
    input_proj: Linear,
    blocks: Vec<ResBlock>,
    final_norm: LayerNorm,
    final_ada: Linear,
    final_linear: Linear,
}

impl FlowNet {
    fn load(w: &Weights) -> Result<Self> {
        let p = "flow_lm.flow_net";
        let mut time = Vec::new();
        while w.has(&format!("{p}.time_embed.{}.freqs", time.len())) {
            let q = format!("{p}.time_embed.{}", time.len());
            time.push(TimeEmbed {
                freqs: w.get(&format!("{q}.freqs"))?,
                l0: Linear::load(w, &format!("{q}.mlp.0"), true)?,
                l2: Linear::load(w, &format!("{q}.mlp.2"), true)?,
                alpha: w.get(&format!("{q}.mlp.3.alpha"))?,
            });
        }
        if time.len() != 2 {
            candle_core::bail!("expected an LSD flow head with two time embeddings, found {}", time.len());
        }
        let mut blocks = Vec::new();
        while w.has(&format!("{p}.res_blocks.{}.in_ln.weight", blocks.len())) {
            let q = format!("{p}.res_blocks.{}", blocks.len());
            blocks.push(ResBlock {
                in_ln: layer_norm(w, &format!("{q}.in_ln"), 1e-6)?,
                mlp0: Linear::load(w, &format!("{q}.mlp.0"), true)?,
                mlp2: Linear::load(w, &format!("{q}.mlp.2"), true)?,
                ada: Linear::load(w, &format!("{q}.adaLN_modulation.1"), true)?,
            });
        }
        let final_linear = Linear::load(w, &format!("{p}.final_layer.linear"), true)?;
        Ok(Self {
            time,
            cond_embed: Linear::load(w, &format!("{p}.cond_embed"), true)?,
            input_proj: Linear::load(w, &format!("{p}.input_proj"), true)?,
            blocks,
            final_norm: layer_norm_plain(final_linear.w.dim(1)?, 1e-6)?,
            final_ada: Linear::load(w, &format!("{p}.final_layer.adaLN_modulation.1"), true)?,
            final_linear,
        })
    }

    /// Flow direction at start time `s` and target time `t` for `x`.
    fn forward(&self, c: &Tensor, s: f64, t: f64, x: &Tensor) -> Result<Tensor> {
        let mut x = self.input_proj.forward(x)?;
        let time = ((self.time[0].forward(s)? + self.time[1].forward(t)?)? / 2.0)?;
        let y = (self.cond_embed.forward(c)? + time)?;
        let ys = y.silu()?;
        for b in &self.blocks {
            let m = b.ada.forward(&ys)?.chunk(3, D::Minus1)?;
            let h = b.in_ln.forward(&x)?.broadcast_mul(&(&m[1] + 1.0)?)?.broadcast_add(&m[0])?;
            let h = b.mlp2.forward(&b.mlp0.forward(&h)?.silu()?)?;
            x = (&x + m[2].broadcast_mul(&h)?)?;
        }
        let m = self.final_ada.forward(&ys)?.chunk(2, D::Minus1)?;
        let x = self.final_norm.forward(&x)?.broadcast_mul(&(&m[1] + 1.0)?)?.broadcast_add(&m[0])?;
        self.final_linear.forward(&x)
    }
}

/// Precomputed voice: the FlowLM key/value cache after the voice prompt.
pub struct Voice {
    caches: Vec<KvCache>,
}

impl Voice {
    pub fn read(path: &std::path::Path, layers: usize) -> Result<Self> {
        let w = Weights::read(path)?;
        let mut caches = Vec::new();
        for i in 0..layers {
            let p = format!("transformer.layers.{i}.self_attn");
            let cache = w.st.load(&format!("{p}/cache"), &Device::Cpu)?.to_dtype(DType::F32)?;
            let offset = w.st.load(&format!("{p}/offset"), &Device::Cpu)?.to_dtype(DType::I64)?.flatten_all()?.to_vec1::<i64>()?;
            let offset = *offset.first().ok_or_else(|| candle_core::Error::Msg("empty offset".into()))? as usize;
            // [2, 1, len, heads, head_dim] → keys and values as [1, heads, offset, head_dim]
            let part = |i: usize| cache.i(i)?.narrow(1, 0, offset)?.transpose(1, 2)?.contiguous();
            caches.push(KvCache { k: Some(part(0)?), v: Some(part(1)?), offset });
        }
        if w.has(&format!("transformer.layers.{layers}.self_attn/cache")) {
            candle_core::bail!("the voice was made for a larger model");
        }
        Ok(Self { caches })
    }
}

pub struct FlowLm {
    embed: Tensor,
    input_linear: Linear,
    layers: Vec<Layer>,
    rope: Rope,
    out_norm: LayerNorm,
    out_eos: Linear,
    bos: Tensor,
    emb_mean: Tensor,
    emb_std: Tensor,
    flow: FlowNet,
}

pub struct Step {
    pub latent: Tensor,
    pub eos: bool,
}

impl FlowLm {
    pub fn load(w: &Weights) -> Result<Self> {
        let layers = load_layers(w, "flow_lm.transformer.layers", 16, None, false)?;
        let d_model = layers[0].norm1.weight().dim(0)?;
        Ok(Self {
            embed: w.get("flow_lm.conditioner.embed.weight")?,
            input_linear: Linear::load(w, "flow_lm.input_linear", false)?,
            rope: Rope::new(d_model / 16, 10_000.0),
            layers,
            out_norm: layer_norm(w, "flow_lm.out_norm", 1e-5)?,
            out_eos: Linear::load(w, "flow_lm.out_eos", true)?,
            bos: w.get("flow_lm.bos_emb")?.reshape((1, 1, ()))?,
            emb_mean: w.get("flow_lm.emb_mean")?,
            emb_std: w.get("flow_lm.emb_std")?,
            flow: FlowNet::load(w)?,
        })
    }

    pub fn num_layers(&self) -> usize {
        self.layers.len()
    }

    fn run(&self, mut x: Tensor, caches: &mut [KvCache]) -> Result<Tensor> {
        for (layer, cache) in self.layers.iter().zip(caches.iter_mut()) {
            x = layer.forward(&x, cache, &self.rope)?;
        }
        Ok(x)
    }

    /// Appends text tokens to the voice-primed cache.
    pub fn prompt(&self, tokens: &[u32], caches: &mut [KvCache]) -> Result<()> {
        if tokens.is_empty() {
            return Ok(());
        }
        let ids = Tensor::new(tokens, &Device::Cpu)?;
        let x = self.embed.index_select(&ids, 0)?.unsqueeze(0)?;
        self.run(x, caches).map(|_| ())
    }

    /// One autoregressive step. `prev` is the previous latent, or `None`
    /// for the first frame. `noise` is standard-normal, `[1, ldim]`.
    pub fn step(&self, prev: Option<&Tensor>, caches: &mut [KvCache], temp: f64, noise: &Tensor, eos_threshold: f64) -> Result<Step> {
        let input = match prev {
            Some(l) => l.reshape((1, 1, ()))?,
            None => self.bos.clone(),
        };
        let x = self.run(self.input_linear.forward(&input)?, caches)?;
        let out = self.out_norm.forward(&x)?.i((.., 0))?;
        let eos = self.out_eos.forward(&out)?.flatten_all()?.to_vec1::<f32>()?[0] as f64 > eos_threshold;
        let noise = (noise * temp.sqrt())?;
        // Lagrangian self-distillation, one step from s = 0 to t = 1.
        let latent = (&noise + self.flow.forward(&out, 0.0, 1.0, &noise)?)?;
        Ok(Step { latent, eos })
    }

    pub fn latent_dim(&self) -> usize {
        self.emb_mean.dim(0).unwrap_or(32)
    }
}

/// Upper bound on frames for a chunk of `tokens` text tokens.
pub fn max_frames(tokens: usize) -> usize {
    ((tokens as f64 / TOKENS_PER_SECOND_ESTIMATE + GEN_SECONDS_PADDING) * FRAME_RATE).ceil() as usize
}

/// Runs generation for one prepared chunk, handing each latent to `emit`.
pub fn generate(
    lm: &FlowLm,
    voice: &Voice,
    tokens: &[u32],
    frames_after_eos: usize,
    temp: f64,
    eos_threshold: f64,
    rng: &mut Rng,
    cancelled: &dyn Fn() -> bool,
    emit: &mut dyn FnMut(Tensor) -> bool,
) -> Result<()> {
    let mut caches = voice.caches.clone();
    lm.prompt(tokens, &mut caches)?;
    let mut prev: Option<Tensor> = None;
    let mut eos_step = None;
    let ldim = lm.latent_dim();
    for step in 0..max_frames(tokens.len()) {
        if cancelled() {
            break;
        }
        let noise = Tensor::from_vec(rng.normals(ldim), (1, ldim), &Device::Cpu)?;
        let s = lm.step(prev.as_ref(), &mut caches, temp, &noise, eos_threshold)?;
        if s.eos && eos_step.is_none() && step >= MIN_FRAMES_BEFORE_EOS {
            eos_step = Some(step);
        }
        if eos_step.is_some_and(|e| step >= e + frames_after_eos) {
            break;
        }
        if !emit(s.latent.clone()) {
            break;
        }
        prev = Some(s.latent);
    }
    Ok(())
}

/// A streaming 1-d convolution with left context carried between calls
/// (`StreamingConv1d`, constant padding).
struct Conv {
    w: Tensor,
    b: Tensor,
    context: usize,
}

impl Conv {
    fn load(w: &Weights, p: &str) -> Result<Self> {
        let weight = w.get(&format!("{p}.weight"))?;
        let k = weight.dim(2)?;
        Ok(Self { context: k - 1, w: weight, b: w.get(&format!("{p}.bias"))?.reshape((1, (), 1))? })
    }

    fn forward(&self, x: &Tensor, prev: &mut Option<Tensor>) -> Result<Tensor> {
        let x = if self.context > 0 {
            let p = match prev.take() {
                Some(p) => p,
                None => Tensor::zeros((1, x.dim(1)?, self.context), DType::F32, &Device::Cpu)?,
            };
            let x = Tensor::cat(&[&p, x], 2)?;
            *prev = Some(x.narrow(2, x.dim(2)? - self.context, self.context)?.contiguous()?);
            x
        } else {
            x.clone()
        };
        x.conv1d(&self.w, 0, 1, 1, 1)?.broadcast_add(&self.b)
    }
}

/// Overlap-adds the halves of a transposed convolution whose kernel is
/// twice its stride. `first`/`second` are `[1, C, T * S]`, holding what
/// each input step contributes to its own block and to the next one.
/// `partial` carries the last block into the next call.
fn overlap_add(first: Tensor, second: Tensor, partial: &mut Option<Tensor>, stride: usize) -> Result<Tensor> {
    let (_, c, n) = first.dims3()?;
    let carry = match partial.take() {
        Some(p) => p,
        None => Tensor::zeros((1, c, stride), DType::F32, &Device::Cpu)?,
    };
    *partial = Some(second.narrow(2, n - stride, stride)?.contiguous()?);
    let shifted = Tensor::cat(&[&carry, &second.narrow(2, 0, n - stride)?], 2)?;
    first + shifted
}

/// `StreamingConvTranspose1d` with kernel = 2 × stride, as a matrix
/// product plus overlap-add.
struct ConvTr {
    /// `[C_in, C_out * K]`
    w: Tensor,
    b: Tensor,
    c_out: usize,
    stride: usize,
}

impl ConvTr {
    fn load(w: &Weights, p: &str) -> Result<Self> {
        let weight = w.get(&format!("{p}.weight"))?;
        let (c_in, c_out, k) = weight.dims3()?;
        if k % 2 != 0 {
            candle_core::bail!("{p}: kernel must be twice the stride");
        }
        Ok(Self {
            w: weight.reshape((c_in, c_out * k))?,
            b: w.get(&format!("{p}.bias"))?.reshape((1, (), 1))?,
            c_out,
            stride: k / 2,
        })
    }

    fn forward(&self, x: &Tensor, partial: &mut Option<Tensor>) -> Result<Tensor> {
        let t = x.dim(2)?;
        let s = self.stride;
        // [T, C_out, 2, S]: contribution of each input step to two blocks.
        let p = x.squeeze(0)?.t()?.matmul(&self.w)?.reshape((t, self.c_out, 2, s))?;
        let half = |i: usize| p.i((.., .., i))?.permute((1, 0, 2))?.reshape((1, self.c_out, t * s));
        overlap_add(half(0)?, half(1)?, partial, s)?.broadcast_add(&self.b)
    }
}

enum Block {
    Conv(Conv),
    ConvTr(ConvTr),
    Res(Conv, Conv),
    Elu,
}

pub struct MimiState {
    upsample: Option<Tensor>,
    caches: Vec<KvCache>,
    blocks: Vec<(Option<Tensor>, Option<Tensor>)>,
}

/// The Mimi decoder: latents → 24 kHz audio, streaming.
pub struct Mimi {
    emb_mean: Tensor,
    emb_std: Tensor,
    /// `[512, ldim]` projection out of the quantizer space.
    quant: Tensor,
    /// Depthwise upsampling kernel `[C, 2 × 16]` (200 Hz from 12.5 Hz).
    up: Tensor,
    layers: Vec<Layer>,
    rope: Rope,
    blocks: Vec<Block>,
}

impl Mimi {
    pub fn load(w: &Weights, lm: &FlowLm) -> Result<Self> {
        let up = w.get("mimi.upsample.convtr.convtr.weight")?;
        let (c, _, k) = up.dims3()?;
        if k != 2 * MIMI_STEPS_PER_FRAME {
            candle_core::bail!("unexpected Mimi upsampling kernel {k}");
        }
        let mut blocks = Vec::new();
        let mut i = 0;
        loop {
            let p = format!("mimi.decoder.model.{i}");
            if w.has(&format!("{p}.conv.weight")) {
                blocks.push(Block::Conv(Conv::load(w, &format!("{p}.conv"))?));
            } else if w.has(&format!("{p}.convtr.weight")) {
                blocks.push(Block::ConvTr(ConvTr::load(w, &format!("{p}.convtr"))?));
            } else if w.has(&format!("{p}.block.1.conv.weight")) {
                blocks.push(Block::Res(Conv::load(w, &format!("{p}.block.1.conv"))?, Conv::load(w, &format!("{p}.block.3.conv"))?));
            } else if w.names().iter().any(|n| n.starts_with(&format!("mimi.decoder.model.{}.", i + 1))) {
                blocks.push(Block::Elu);
            } else {
                break;
            }
            i += 1;
        }
        let layers = load_layers(w, "mimi.decoder_transformer.transformer.layers", 8, Some(MIMI_CONTEXT), true)?;
        let d_model = layers[0].norm1.weight().dim(0)?;
        Ok(Self {
            emb_mean: lm.emb_mean.clone(),
            emb_std: lm.emb_std.clone(),
            quant: w.get("mimi.quantizer.output_proj.weight")?.squeeze(2)?,
            up: up.reshape((c, k))?,
            rope: Rope::new(d_model / 8, 10_000.0),
            layers,
            blocks,
        })
    }

    pub fn state(&self) -> MimiState {
        MimiState {
            upsample: None,
            caches: vec![KvCache::empty(); self.layers.len()],
            blocks: self.blocks.iter().map(|_| (None, None)).collect(),
        }
    }

    /// Decodes `[n, ldim]` latents into `n × 1920` samples.
    pub fn decode(&self, latents: &Tensor, st: &mut MimiState) -> Result<Vec<f32>> {
        let n = latents.dim(0)?;
        let x = latents.broadcast_mul(&self.emb_std)?.broadcast_add(&self.emb_mean)?;
        let x = self.quant.matmul(&x.t()?)?; // [512, n]
        // Depthwise transposed conv, stride 16, kernel 32.
        let s = MIMI_STEPS_PER_FRAME;
        let c = x.dim(0)?;
        let half = |i: usize| x.unsqueeze(2)?.broadcast_mul(&self.up.narrow(1, i * s, s)?.unsqueeze(1)?)?.reshape((1, c, n * s));
        let x = overlap_add(half(0)?, half(1)?, &mut st.upsample, s)?;

        let mut h = x.transpose(1, 2)?.contiguous()?;
        for (layer, cache) in self.layers.iter().zip(st.caches.iter_mut()) {
            h = layer.forward(&h, cache, &self.rope)?;
        }
        let mut x = h.transpose(1, 2)?.contiguous()?;
        for (block, (s1, s2)) in self.blocks.iter().zip(st.blocks.iter_mut()) {
            x = match block {
                Block::Conv(c) => c.forward(&x, s1)?,
                Block::ConvTr(c) => c.forward(&x, s1)?,
                Block::Res(a, b) => {
                    let v = b.forward(&a.forward(&x.elu(1.0)?, s1)?.elu(1.0)?, s2)?;
                    (&x + v)?
                }
                Block::Elu => x.elu(1.0)?,
            };
        }
        x.flatten_all()?.to_vec1::<f32>()
    }
}

/// SplitMix64 with Box–Muller normals; seeded per request.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    pub fn normals(&mut self, n: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(n + 1);
        while out.len() < n {
            let (u1, u2) = (self.uniform(), self.uniform());
            let r = (-2.0 * u1.ln()).sqrt();
            let a = 2.0 * std::f64::consts::PI * u2;
            out.push((r * a.cos()) as f32);
            out.push((r * a.sin()) as f32);
        }
        out.truncate(n);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_add_matches_a_transposed_convolution() {
        // One channel, stride 2, kernel [1, 2, 3, 4], fed in two calls.
        let w = [1f32, 2., 3., 4.];
        let x = [1f32, 10., 100.];
        let mut expected = vec![0f32; (x.len() + 1) * 2];
        for (t, xv) in x.iter().enumerate() {
            for (k, wv) in w.iter().enumerate() {
                expected[t * 2 + k] += xv * wv;
            }
        }
        let mut partial = None;
        let mut got = Vec::new();
        for part in [&x[..2], &x[2..]] {
            let n = part.len();
            let a: Vec<f32> = part.iter().flat_map(|v| [v * w[0], v * w[1]]).collect();
            let b: Vec<f32> = part.iter().flat_map(|v| [v * w[2], v * w[3]]).collect();
            let a = Tensor::from_vec(a, (1, 1, n * 2), &Device::Cpu).unwrap();
            let b = Tensor::from_vec(b, (1, 1, n * 2), &Device::Cpu).unwrap();
            got.extend(overlap_add(a, b, &mut partial, 2).unwrap().flatten_all().unwrap().to_vec1::<f32>().unwrap());
        }
        assert_eq!(got, expected[..x.len() * 2]);
    }

    #[test]
    fn normals_have_unit_variance() {
        let v = Rng::new(7).normals(20_000);
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32;
        assert!(mean.abs() < 0.03 && (var - 1.0).abs() < 0.05, "{mean} {var}");
    }

    #[test]
    fn frame_budget_follows_the_reference() {
        // ceil((12 / 3 + 2) * 12.5)
        assert_eq!(max_frames(12), 75);
    }
}
