//! Fetches any catalog voices missing from an installed Kokoro model, then
//! speaks one sample per language and writes WAVs for listening.
//!
//! cargo run --release -p speakit-tts --example kokoro_languages -- <models-dir> <out-dir>

use std::path::PathBuf;
use std::time::Instant;

use speakit_models::{Cancel, Store};
use speakit_tts::kokoro::{KokoroConfig, KokoroEngine, KokoroVoice};
use speakit_tts::{CancelToken, Engine};

const SAMPLES: &[(&str, &str)] = &[
    ("kokoro:af_heart", "Every voice is now available, grouped by language."),
    ("kokoro:bm_george", "Good afternoon. The train to London leaves at half past three."),
    ("kokoro:ef_dora", "Hola, ¿cómo estás? Hoy hace un día precioso."),
    ("kokoro:ff_siwis", "Bonjour, comment allez-vous aujourd'hui ?"),
    ("kokoro:hf_alpha", "नमस्ते, आप कैसे हैं?"),
    ("kokoro:if_sara", "Ciao, come stai? Oggi è una bella giornata."),
    ("kokoro:jf_alpha", "こんにちは、きょうは いい てんき ですね。"),
    ("kokoro:pf_dora", "Olá, tudo bem? Hoje está um lindo dia."),
    ("kokoro:zf_xiaoxiao", "你好，今天天气很好。"),
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let store = Store::new(PathBuf::from(&args[1]));
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out).unwrap();
    let model = speakit_models::find("kokoro-82m-v1.0").unwrap();

    let current = store.installed(model).expect("Kokoro installed");
    let missing = store.missing_voices(model).len();
    println!("{missing} catalog voices missing on disk");
    if missing > 0 {
        let t = Instant::now();
        store.install(model, &current.marker.artifact_id, &Cancel::default(), &mut |_| {}).expect("install");
        println!("fetched and verified in {:.1?}; now missing: {}", t.elapsed(), store.missing_voices(model).len());
    }
    let installed = store.installed(model).unwrap();

    let worker_bin = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("sovirae-kokoro-worker");
    let voices = model.voices.iter().map(|v| KokoroVoice {
        id: v.id.clone(), label: v.label.clone(), language: v.language.clone(), gender: v.gender.clone(),
        g2p: v.g2p.clone(), approximate_pronunciation: v.pronunciation.is_some(), file: installed.voice_file(&v.id),
    }).collect();
    let engine = KokoroEngine::new(KokoroConfig {
        model_id: model.id.clone(), model_name: model.name.clone(), voice_prefix: model.voice_prefix.clone(),
        model_file: installed.model_file.clone(), worker_bin, voices, threads: 4, gpu: false, model_rate: model.sample_rate,
    });
    let listed = engine.voices().unwrap();
    println!("engine lists {} voices", listed.len());

    for (voice, text) in SAMPLES {
        let t = Instant::now();
        let pcm = engine.synthesize(text, voice, 24_000, &CancelToken::default()).expect(voice);
        let secs = pcm.samples.len() as f64 / 24_000.0;
        println!("{voice}: {secs:.2}s audio in {:.2}s", t.elapsed().as_secs_f64());
        let mut w = hound::WavWriter::create(
            out.join(format!("{}.wav", voice.trim_start_matches("kokoro:"))),
            hound::WavSpec { channels: 1, sample_rate: 24_000, bits_per_sample: 32, sample_format: hound::SampleFormat::Float },
        ).unwrap();
        for s in &pcm.samples {
            w.write_sample(*s).unwrap();
        }
        w.finalize().unwrap();
        assert!(secs > 0.8, "{voice} produced too little audio");
    }
}
