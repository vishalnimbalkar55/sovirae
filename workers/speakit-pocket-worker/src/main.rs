//! Isolated Pocket TTS inference process (spec §5.1, §8.2), for Kyutai's
//! Pocket TTS models with predefined voices.
//!
//! The desktop app starts this worker with a fixed model, tokenizer, thread
//! count, and per-language text options, sends one JSON request per line on
//! stdin, and reads binary frames from stdout. If inference hangs or
//! crashes, the app kills the process and the UI keeps running.
//!
//! Frame: u64 request id | u32 status | u32 length | payload (little-endian)
//!   status 0: `length` f32 samples of 24 kHz mono audio
//!   status 1: `length` bytes of UTF-8 error text
//!   status 2: ready (sent once after the model loads); payload is the
//!             UTF-8 name of the device in use (`CPU`)

mod model;
mod text;

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use candle_core::Tensor;
use serde::Deserialize;

use model::{FlowLm, Mimi, Rng, Voice, Weights};
use text::{Text, TextOptions};

#[derive(Deserialize)]
struct Request {
    id: u64,
    text: String,
    /// Absolute path to a predefined voice state (safetensors).
    voice: String,
    /// Fixed seed for reproducible output; random when absent.
    seed: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Options {
    #[serde(flatten)]
    text: TextOptions,
    temperature: f64,
    eos_threshold: f64,
    max_tokens_per_chunk: usize,
}

impl Default for Options {
    fn default() -> Self {
        // Defaults of the reference package (`default_parameters.py`).
        Self { text: TextOptions::default(), temperature: 0.3, eos_threshold: -4.0, max_tokens_per_chunk: 50 }
    }
}

struct Worker {
    lm: FlowLm,
    mimi: Mimi,
    text: Text,
    options: Options,
    voices: HashMap<String, Voice>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let (Some(model), Some(tokenizer)) = (arg("--model").map(PathBuf::from), arg("--tokenizer").map(PathBuf::from)) else {
        eprintln!("usage: speakit-pocket-worker --model <model.safetensors> --tokenizer <tokenizer.json> [--threads <n>] [--options <json>]");
        std::process::exit(2);
    };
    let threads = arg("--threads").and_then(|t| t.parse::<usize>().ok()).unwrap_or(2).max(1);
    // Both pools read these once, on first use (spec §8.2).
    std::env::set_var("RAYON_NUM_THREADS", threads.to_string());
    std::env::set_var("VECLIB_MAXIMUM_THREADS", threads.to_string());

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let options = match arg("--options").map(|o| serde_json::from_str::<Options>(&o)).transpose() {
        Ok(o) => o.unwrap_or_default(),
        Err(e) => {
            write_error(&mut out, 0, &format!("invalid model options: {e}"));
            std::process::exit(1);
        }
    };
    let mut worker = match Worker::load(&model, &tokenizer, options) {
        Ok(w) => w,
        Err(e) => {
            write_error(&mut out, 0, &format!("The voice model could not be loaded: {e}"));
            std::process::exit(1);
        }
    };
    write_frame(&mut out, 0, 2, b"CPU");

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                write_error(&mut out, 0, &format!("invalid request: {e}"));
                continue;
            }
        };
        match worker.synthesize(&req) {
            Ok(samples) => {
                let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
                write_frame(&mut out, req.id, 0, &bytes);
            }
            Err(e) => write_error(&mut out, req.id, &e),
        }
    }
}

fn write_frame(out: &mut impl Write, id: u64, status: u32, payload: &[u8]) {
    let len = if status == 0 { payload.len() / 4 } else { payload.len() } as u32;
    let mut header = Vec::with_capacity(16);
    header.extend_from_slice(&id.to_le_bytes());
    header.extend_from_slice(&status.to_le_bytes());
    header.extend_from_slice(&len.to_le_bytes());
    // stdout carries protocol bytes only; a broken pipe means the app is gone.
    if out.write_all(&header).and_then(|_| out.write_all(payload)).and_then(|_| out.flush()).is_err() {
        std::process::exit(0);
    }
}

fn write_error(out: &mut impl Write, id: u64, message: &str) {
    write_frame(out, id, 1, message.as_bytes());
}

impl Worker {
    fn load(model: &Path, tokenizer: &Path, options: Options) -> Result<Self, String> {
        let weights = Weights::read(model).map_err(|e| e.to_string())?;
        let lm = FlowLm::load(&weights).map_err(|e| e.to_string())?;
        let mimi = Mimi::load(&weights, &lm).map_err(|e| e.to_string())?;
        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer).map_err(|e| format!("tokenizer: {e}"))?;
        Ok(Self { lm, mimi, text: Text::new(tokenizer)?, options, voices: HashMap::new() })
    }

    fn voice(&mut self, path: &str) -> Result<&Voice, String> {
        if !self.voices.contains_key(path) {
            let v = Voice::read(Path::new(path), self.lm.num_layers()).map_err(|e| format!("voice file unreadable: {e}"))?;
            self.voices.insert(path.to_string(), v);
        }
        Ok(&self.voices[path])
    }

    fn synthesize(&mut self, req: &Request) -> Result<Vec<f32>, String> {
        let o = &self.options;
        let chunks = self.text.chunks(&req.text, &o.text, o.max_tokens_per_chunk)?;
        let mut prepared = Vec::new();
        for chunk in &chunks {
            if let Some((text, guess)) = text::prepare(chunk, &o.text) {
                prepared.push((self.text.encode(&text)?, guess + 2));
            }
        }
        let (temp, eos) = (o.temperature, o.eos_threshold);
        let mut rng = Rng::new(req.seed.unwrap_or_else(seed));
        self.voice(&req.voice)?;
        let voice = &self.voices[&req.voice];
        let mut audio = Vec::new();
        for (tokens, frames_after_eos) in prepared {
            audio.extend(self.chunk(voice, &tokens, frames_after_eos, temp, eos, &mut rng)?);
        }
        limit_peak(&mut audio);
        Ok(audio)
    }

    /// Generates latents on this thread while a second one decodes them,
    /// as the reference does; each chunk starts a fresh decoder state.
    fn chunk(&self, voice: &Voice, tokens: &[u32], frames_after_eos: usize, temp: f64, eos: f64, rng: &mut Rng) -> Result<Vec<f32>, String> {
        let (tx, rx) = mpsc::channel::<Tensor>();
        std::thread::scope(|s| {
            let decoder = s.spawn(move || -> candle_core::Result<Vec<f32>> {
                let mut state = self.mimi.state();
                let mut audio = Vec::new();
                while let Ok(first) = rx.recv() {
                    // Decode everything queued so far in one call.
                    let mut batch = vec![first];
                    batch.extend(rx.try_iter());
                    let latents = Tensor::cat(&batch, 0)?;
                    audio.extend(self.mimi.decode(&latents, &mut state)?);
                }
                Ok(audio)
            });
            let generated = model::generate(&self.lm, voice, tokens, frames_after_eos, temp, eos, rng, &|| false, &mut |latent| {
                tx.send(latent).is_ok()
            });
            drop(tx);
            let audio = decoder.join().map_err(|_| "the audio decoder crashed".to_string())?;
            generated.map_err(|e| e.to_string())?;
            audio.map_err(|e| e.to_string())
        })
    }
}

/// The model occasionally overshoots full scale; scale such a segment down
/// rather than let playback clip.
fn limit_peak(audio: &mut [f32]) {
    const CEILING: f32 = 0.98;
    let peak = audio.iter().fold(0f32, |m, s| m.max(s.abs()));
    if peak > CEILING {
        let gain = CEILING / peak;
        audio.iter_mut().for_each(|s| *s *= gain);
    }
}

fn seed() -> u64 {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    t.as_nanos() as u64 ^ (std::process::id() as u64) << 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_only_overshooting_segments() {
        let mut quiet = vec![0.5, -0.9];
        limit_peak(&mut quiet);
        assert_eq!(quiet, [0.5, -0.9]);
        let mut loud = vec![0.5, -1.4];
        limit_peak(&mut loud);
        assert!((loud[1] + 0.98).abs() < 1e-6 && (loud[0] - 0.35).abs() < 1e-6);
    }
}
