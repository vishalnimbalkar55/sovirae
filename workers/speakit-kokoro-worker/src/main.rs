//! Isolated Kokoro-82M inference process (spec §5.1, §8.2).
//!
//! The desktop app starts this worker with a fixed model path, thread
//! count, and device, sends one JSON request per line on stdin, and reads binary frames
//! from stdout. If inference hangs or crashes, the app kills the process and
//! the UI keeps running.
//!
//! Frame: u64 request id | u32 status | u32 length | payload (little-endian)
//!   status 0: `length` f32 samples of 24 kHz mono audio
//!   status 1: `length` bytes of UTF-8 error text
//!   status 2: ready (sent once after the model loads); payload is the
//!             UTF-8 name of the device in use, e.g. `CPU` or `GPU`

mod vocab;

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor;
use serde::Deserialize;

/// Model context minus the two pad tokens.
const MAX_TOKENS: usize = 510;
const STYLE_DIM: usize = 256;

#[derive(Deserialize)]
struct Request {
    id: u64,
    phonemes: String,
    /// Absolute path to a voice style file (510 × 1 × 256 f32).
    voice: String,
    speed: f32,
}

struct Worker {
    session: Session,
    device: &'static str,
    ids_name: String,
    style_name: String,
    speed_name: String,
    vocab: HashMap<char, i64>,
    voices: HashMap<String, Vec<f32>>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let Some(model) = arg("--model").map(PathBuf::from) else {
        eprintln!("usage: speakit-kokoro-worker --model <path> [--threads <n>] [--device cpu|gpu]");
        std::process::exit(2);
    };
    let threads = arg("--threads").and_then(|t| t.parse::<usize>().ok()).unwrap_or(2).max(1);
    let gpu = arg("--device").is_some_and(|d| d == "gpu");

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut worker = match Worker::load(&model, threads, gpu) {
        Ok(w) => w,
        Err(e) => {
            write_error(&mut out, 0, &format!("The voice model could not be loaded: {e}"));
            std::process::exit(1);
        }
    };
    write_frame(&mut out, 0, 2, worker.device.as_bytes());

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
    fn load(model: &PathBuf, threads: usize, gpu: bool) -> Result<Self, String> {
        let e = |e: ort::Error<ort::session::builder::SessionBuilder>| e.to_string();
        let mut builder = Session::builder().map_err(|e| e.to_string())?;
        if gpu {
            builder = builder.with_execution_providers([gpu_provider()?]).map_err(e)?;
        }
        let session = builder
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(e)?
            .with_intra_threads(threads)
            .map_err(e)?
            .with_inter_threads(1)
            .map_err(e)?
            // No busy-waiting between requests (spec §8.2).
            .with_intra_op_spinning(false)
            .map_err(e)?
            .with_inter_op_spinning(false)
            .map_err(e)?
            .commit_from_file(model)
            .map_err(|e| e.to_string())?;

        let names: Vec<String> = session.inputs().iter().map(|i| i.name().to_string()).collect();
        let find = |keys: &[&str]| {
            names.iter().find(|n| keys.iter().any(|k| n.contains(k))).cloned()
        };
        let ids_name = find(&["input_ids", "tokens"]).ok_or("model has no token input")?;
        let style_name = find(&["style", "ref_s"]).ok_or("model has no style input")?;
        let speed_name = find(&["speed"]).ok_or("model has no speed input")?;
        Ok(Self {
            session,
            device: if gpu { "GPU" } else { "CPU" },
            ids_name,
            style_name,
            speed_name,
            vocab: vocab::VOCAB.iter().copied().collect(),
            voices: HashMap::new(),
        })
    }

    fn voice(&mut self, path: &str) -> Result<&[f32], String> {
        if !self.voices.contains_key(path) {
            let bytes = std::fs::read(path).map_err(|e| format!("voice file unreadable: {e}"))?;
            if bytes.len() % (STYLE_DIM * 4) != 0 || bytes.is_empty() {
                return Err("voice file has an unexpected size".into());
            }
            let floats = bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
            self.voices.insert(path.to_string(), floats);
        }
        Ok(&self.voices[path])
    }

    fn synthesize(&mut self, req: &Request) -> Result<Vec<f32>, String> {
        let ids: Vec<i64> = req.phonemes.chars().filter_map(|c| self.vocab.get(&c).copied()).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let speed = if req.speed.is_finite() { req.speed.clamp(0.5, 2.0) } else { 1.0 };
        let mut audio = Vec::new();
        for chunk in split_tokens(&ids, MAX_TOKENS) {
            // The style vector is indexed by the number of phoneme tokens.
            let rows = self.voice(&req.voice)?.len() / STYLE_DIM;
            let row = chunk.len().min(rows - 1);
            let style: Vec<f32> = self.voices[&req.voice][row * STYLE_DIM..(row + 1) * STYLE_DIM].to_vec();

            let mut padded = Vec::with_capacity(chunk.len() + 2);
            padded.push(0);
            padded.extend_from_slice(chunk);
            padded.push(0);
            let n = padded.len();
            let ids_t = Tensor::from_array(([1usize, n], padded)).map_err(|e| e.to_string())?;
            let style_t = Tensor::from_array(([1usize, STYLE_DIM], style)).map_err(|e| e.to_string())?;
            let speed_t = Tensor::from_array(([1usize], vec![speed])).map_err(|e| e.to_string())?;
            let outputs = self
                .session
                .run(ort::inputs![
                    self.ids_name.as_str() => ids_t,
                    self.style_name.as_str() => style_t,
                    self.speed_name.as_str() => speed_t,
                ])
                .map_err(|e| e.to_string())?;
            let (_, samples) = outputs[0].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
            audio.extend_from_slice(samples);
        }
        Ok(audio)
    }
}

/// The GPU execution provider for this platform: WebGPU, which runs on
/// Metal on macOS. Measured on Apple M4 against CoreML, which was no faster
/// than the CPU for this model (spec §7.3). Registration errors are reported
/// instead of silently running on the CPU.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn gpu_provider() -> Result<ort::ep::ExecutionProviderDispatch, String> {
    Ok(ort::ep::WebGPU::default().build().error_on_failure())
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn gpu_provider() -> Result<ort::ep::ExecutionProviderDispatch, String> {
    Err("GPU acceleration is not available on this computer".into())
}

/// Splits token IDs into model-sized chunks, preferring to cut after
/// sentence punctuation, then after a space.
fn split_tokens(ids: &[i64], max: usize) -> Vec<&[i64]> {
    const SPACE: i64 = 16;
    const STRONG: [i64; 6] = [1, 2, 3, 4, 5, 6]; // ; : , . ! ?
    let mut out = Vec::new();
    let mut rest = ids;
    while rest.len() > max {
        let window = &rest[..max];
        let cut = window
            .iter()
            .rposition(|t| STRONG.contains(t))
            .filter(|&i| i > max / 3)
            .or_else(|| window.iter().rposition(|&t| t == SPACE).filter(|&i| i > max / 3))
            .map(|i| i + 1)
            .unwrap_or(max);
        out.push(&rest[..cut]);
        rest = &rest[cut..];
    }
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocab_is_complete_and_unique() {
        let map: HashMap<char, i64> = vocab::VOCAB.iter().copied().collect();
        assert_eq!(map.len(), vocab::VOCAB.len());
        assert_eq!(map[&'$'], 0);
        assert_eq!(map[&' '], 16);
        for c in "həlˈO wˈɜɹld".chars() {
            assert!(map.contains_key(&c), "missing {c}");
        }
    }

    #[test]
    fn splits_long_token_runs() {
        let ids: Vec<i64> = (0..1200).map(|i| if i % 7 == 0 { 16 } else { 50 }).collect();
        let chunks = split_tokens(&ids, 510);
        assert!(chunks.iter().all(|c| c.len() <= 510));
        assert_eq!(chunks.iter().map(|c| c.len()).sum::<usize>(), 1200);
        assert_eq!(*chunks[0].last().unwrap(), 16, "cuts after a space");
    }
}
