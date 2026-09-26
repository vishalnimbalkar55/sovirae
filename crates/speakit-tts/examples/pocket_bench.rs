//! Installs Pocket TTS English through the verified downloader, then
//! measures cold and warm synthesis and writes WAVs for listening.
//!
//! cargo build --release -p speakit-pocket-worker
//! cargo run --release -p speakit-tts --example pocket_bench -- <models-dir> <out-dir> [threads]

use std::path::PathBuf;
use std::time::Instant;

use speakit_models::{Cancel, Store};
use speakit_tts::pocket::{PocketConfig, PocketEngine, PocketVoice};
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
    let out = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| ".".into()));
    let threads: usize = args.get(3).and_then(|t| t.parse().ok()).unwrap_or(4);
    std::fs::create_dir_all(&out).unwrap();

    let model = speakit_models::find("pocket-tts-en").expect("catalog entry");
    let artifact = &model.artifacts[0].id;
    let store = Store::new(root);
    let t = Instant::now();
    let mut last = 0u64;
    let installed = store
        .install(model, artifact, &Cancel::default(), &mut |p| {
            if p.done_bytes >= last + (64 << 20) || p.done_bytes == p.total_bytes {
                last = p.done_bytes;
                eprintln!("  {:>4} / {} MB", p.done_bytes >> 20, p.total_bytes >> 20);
            }
        })
        .expect("install");
    println!("installed {} in {:.1?}", model.name, t.elapsed());

    let worker_bin = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("speakit-pocket-worker");
    let voices = model
        .voices
        .iter()
        .map(|v| PocketVoice { id: v.id.clone(), label: v.label.clone(), language: v.language.clone(), gender: v.gender.clone(), file: installed.voice_file(&v.id) })
        .collect();
    let engine = PocketEngine::new(PocketConfig {
        model_id: model.id.clone(),
        model_name: model.name.clone(),
        voice_prefix: model.voice_prefix.clone(),
        model_file: installed.model_file.clone(),
        tokenizer_file: installed.support_file(&model.files[0]),
        worker_bin,
        voices,
        threads,
        options: model.options.clone(),
        model_rate: model.sample_rate,
    });
    assert_eq!(engine.voices().unwrap().len(), model.voices.len(), "every voice installed");

    let first = &model.voices[0].id;
    let male = model.voices.iter().find(|v| v.gender.as_deref() == Some("male")).unwrap();
    for voice in [first, &male.id] {
        let id = format!("{}:{voice}", model.voice_prefix);
        for (i, text) in CORPUS.iter().enumerate() {
            let t = Instant::now();
            let pcm = engine.synthesize(text, &id, 24_000, &CancelToken::default()).expect("synthesize");
            let wall = t.elapsed().as_secs_f64();
            let audio = pcm.samples.len() as f64 / 24_000.0;
            let peak = pcm.samples.iter().fold(0f32, |m, s| m.max(s.abs()));
            let cold = if i == 0 && voice == first { "  (includes worker start + model load)" } else { "" };
            println!("{id} #{i}: {audio:.2}s audio in {wall:.2}s  RTF {:.3}  peak {peak:.2}{cold}", wall / audio);
            let spec = hound::WavSpec { channels: 1, sample_rate: 24_000, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
            let mut w = hound::WavWriter::create(out.join(format!("pocket-en-{voice}-{i}.wav")), spec).unwrap();
            for s in &pcm.samples {
                w.write_sample(*s).unwrap();
            }
            w.finalize().unwrap();
        }
    }
    let cancel = CancelToken::default();
    cancel.cancel();
    assert!(engine.synthesize("Cancelled.", &format!("{}:{first}", model.voice_prefix), 24_000, &cancel).is_err());
    println!("cancellation ok");
}
