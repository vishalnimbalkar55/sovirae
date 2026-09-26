//! Offline windowed-sinc resampling of one synthesized segment to the
//! output device rate (e.g. Kokoro's 24 kHz to 48 kHz).

use std::f64::consts::PI;

/// Filter half-width in input samples.
const HALF_TAPS: i64 = 16;

pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to as f64 / from as f64;
    // Low-pass at the lower Nyquist, with a little margin against aliasing.
    let cutoff = ratio.min(1.0) * 0.94;
    let half = (HALF_TAPS as f64 / cutoff).ceil() as i64;
    let out_len = (input.len() as f64 * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for j in 0..out_len {
        let t = j as f64 / ratio;
        let center = t.floor() as i64;
        let mut acc = 0.0f64;
        for k in (center - half + 1)..=(center + half) {
            if k < 0 || k as usize >= input.len() {
                continue;
            }
            let x = t - k as f64;
            let w = blackman(x / half as f64);
            acc += input[k as usize] as f64 * cutoff * sinc(x * cutoff) * w;
        }
        out.push(acc as f32);
    }
    out
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

/// Blackman window over [-1, 1].
fn blackman(u: f64) -> f64 {
    if u.abs() >= 1.0 {
        return 0.0;
    }
    let p = (u + 1.0) / 2.0;
    0.42 - 0.5 * (2.0 * PI * p).cos() + 0.08 * (4.0 * PI * p).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(rate: u32, hz: f32, len: usize) -> Vec<f32> {
        (0..len).map(|i| (i as f32 * 2.0 * std::f32::consts::PI * hz / rate as f32).sin() * 0.5).collect()
    }

    #[test]
    fn upsamples_without_changing_pitch_or_level() {
        let input = tone(24_000, 1_000.0, 24_000);
        let out = resample(&input, 24_000, 48_000);
        assert_eq!(out.len(), 48_000);
        let expected = tone(48_000, 1_000.0, 48_000);
        let mid = &out[1_000..47_000];
        let err = mid.iter().zip(&expected[1_000..47_000]).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
        assert!(err < 0.01, "max error {err}");
    }

    #[test]
    fn handles_non_integer_ratios() {
        let input = tone(24_000, 440.0, 24_000);
        let out = resample(&input, 24_000, 44_100);
        assert_eq!(out.len(), 44_100);
        let peak = out[2_000..42_000].iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((peak - 0.5).abs() < 0.02, "peak {peak}");
    }
}
