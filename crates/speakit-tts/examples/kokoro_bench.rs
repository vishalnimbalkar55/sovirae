//! Installs Kokoro through the verified downloader, then measures cold and
//! warm synthesis on a small English corpus and writes WAVs for listening.
//!
//! cargo run --release -p speakit-tts --example kokoro_bench -- <models-dir> <fp32|q8> <out-dir> [threads] [cpu|gpu]

use std::path::PathBuf;
use std::time::Instant;

use speakit_models::{Cancel, Store};
use speakit_tts::kokoro::{KokoroConfig, KokoroEngine, KokoroVoice};
use speakit_tts::{CancelToken, Engine};

const CORPUS: &[&str] = &[
    "Local speech synthesis has improved quickly, and a compact model can now run on an ordinary laptop.",
    "Dr. Smith paid $3.50 on Jan. 5, 2024 — was it worth it?",
    "The URL is example.com, and the API returns JSON in about 120 milliseconds.",
    "\"Well,\" she said, \"I suppose we could try again tomorrow.\"",
    "Photosynthesis converts light energy into chemical energy stored in glucose.",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]);
    let artifact = args.get(2).map(String::as_str).unwrap_or("fp32");
    let out = PathBuf::from(args.get(3).cloned().unwrap_or_else(|| ".".into()));
    let threads: usize = args.get(4).and_then(|t| t.parse().ok()).unwrap_or(4);
    let gpu = args.get(5).is_some_and(|d| d == "gpu");
    std::fs::create_dir_all(&out).unwrap();

    let model = speakit_models::find("kokoro-82m-v1.0").expect("catalog entry");
    let store = Store::new(root);
    let t = Instant::now();
    let mut last = 0u64;
    let installed = store
        .install(model, artifact, &Cancel::default(), &mut |p| {
            if p.done_bytes >= last + (32 << 20) || p.done_bytes == p.total_bytes {
                last = p.done_bytes;
                eprintln!("  {:>4} / {} MB", p.done_bytes >> 20, p.total_bytes >> 20);
            }
        })
        .expect("install");
    println!("installed {} ({}) in {:.1?}", model.name, artifact, t.elapsed());

    let worker_bin = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("speakit-kokoro-worker");
    let voices = model
        .voices
        .iter()
        .map(|v| KokoroVoice { id: v.id.clone(), label: v.label.clone(), language: v.language.clone(), gender: v.gender.clone(), g2p: v.g2p.clone(), approximate_pronunciation: v.pronunciation.is_some(), file: installed.voice_file(&v.id) })
        .collect();
    let engine = KokoroEngine::new(KokoroConfig {
        model_id: model.id.clone(),
        model_name: model.name.clone(),
        voice_prefix: model.voice_prefix.clone(),
        model_file: installed.model_file.clone(),
        worker_bin,
        voices,
        threads,
        gpu,
        model_rate: model.sample_rate,
    });
    assert!(engine.has_phonemizer(), "espeak-ng not found");

    for voice in ["kokoro:af_heart", "kokoro:bm_george"] {
        for (i, text) in CORPUS.iter().enumerate() {
            let t = Instant::now();
            let pcm = engine.synthesize(text, voice, 24_000, &CancelToken::default()).expect("synthesize");
            let wall = t.elapsed().as_secs_f64();
            let audio = pcm.samples.len() as f64 / 24_000.0;
            let peak = pcm.samples.iter().fold(0f32, |m, s| m.max(s.abs()));
            println!("{voice} #{i}: {:.2}s audio in {:.2}s  RTF {:.3}  peak {:.2}{}", audio, wall, wall / audio, peak, if i == 0 && voice.ends_with("heart") { "  (includes worker start + model load)" } else { "" });
            let name = format!("{}-{}-{}-{}.wav", artifact, if gpu { "gpu" } else { "cpu" }, voice.trim_start_matches("kokoro:"), i);
            let mut w = hound::WavWriter::create(out.join(name), hound::WavSpec { channels: 1, sample_rate: 24_000, bits_per_sample: 32, sample_format: hound::SampleFormat::Float }).unwrap();
            for s in &pcm.samples {
                w.write_sample(*s).unwrap();
            }
            w.finalize().unwrap();
        }
    }
    let cancel = CancelToken::default();
    cancel.cancel();
    assert!(engine.synthesize("Cancelled.", "kokoro:af_heart", 24_000, &cancel).is_err());
    println!("cancellation ok");
}
