//! Streaming WSOLA time stretcher: changes speed without changing pitch and
//! without resynthesizing speech (spec §9.3). The rate may change between
//! calls; state carries across so segment joins stay continuous.

use std::f32::consts::PI;

pub struct Stretcher {
    /// Analysis window length in samples.
    n: usize,
    /// Synthesis hop (n / 2).
    hs: usize,
    /// Search radius for the best-matching analysis frame.
    delta: usize,
    window: Vec<f32>,
    /// Buffered input; `input[0]` is absolute input sample `base`.
    input: Vec<f32>,
    base: u64,
    /// Nominal analysis position (absolute input samples).
    ana_pos: f64,
    /// Absolute input position of the previously chosen frame.
    prev: Option<u64>,
    /// Second half of the previous windowed frame, waiting for overlap.
    tail: Vec<f32>,
    rate: f64,
    finished_input: bool,
}

impl Stretcher {
    pub fn new(sample_rate: u32) -> Self {
        // ~21 ms windows, ±5 ms search: tuned for speech.
        let n = ((sample_rate as usize * 21 / 1000) & !1).max(64);
        let hs = n / 2;
        // Periodic Hann: overlapping at n/2 sums exactly to one.
        let window = (0..n).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / n as f32).cos()).collect();
        Self {
            n,
            hs,
            delta: sample_rate as usize * 5 / 1000,
            window,
            input: Vec::new(),
            base: 0,
            ana_pos: 0.0,
            prev: None,
            tail: vec![0.0; hs],
            rate: 1.0,
            finished_input: false,
        }
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(0.25, 4.0) as f64;
    }

    /// Clears all state, e.g. after a seek or stop.
    pub fn reset(&mut self) {
        self.input.clear();
        self.base = 0;
        self.ana_pos = 0.0;
        self.prev = None;
        self.tail.iter_mut().for_each(|s| *s = 0.0);
        self.finished_input = false;
    }

    pub fn push(&mut self, samples: &[f32]) {
        self.input.extend_from_slice(samples);
    }

    /// No more input will arrive; lets the tail be flushed.
    pub fn finish(&mut self) {
        self.finished_input = true;
    }

    /// Absolute input position of the next analysis frame.
    pub fn input_position(&self) -> u64 {
        self.ana_pos as u64
    }

    /// Input samples buffered but not yet consumed.
    pub fn pending_input(&self) -> usize {
        (self.base + self.input.len() as u64).saturating_sub(self.ana_pos as u64) as usize
    }

    /// Whether `process_hop` would produce output now. Between segments a
    /// remainder shorter than one frame waits for more input and does not.
    pub fn can_process(&self) -> bool {
        let end = self.base + self.input.len() as u64;
        let nominal = self.ana_pos.round() as u64;
        if nominal + (self.n + self.delta) as u64 <= end {
            return true;
        }
        self.finished_input && (nominal < end || self.tail.iter().any(|&s| s != 0.0))
    }

    /// Produces one synthesis hop (`hs` samples) into `out` if enough input
    /// is buffered. Returns the number of samples written.
    pub fn process_hop(&mut self, out: &mut Vec<f32>) -> usize {
        let end = self.base + self.input.len() as u64;
        let nominal = self.ana_pos.round() as u64;
        let need = nominal + (self.n + self.delta) as u64;
        if need > end && !self.finished_input {
            return 0;
        }
        if nominal >= end {
            // Input exhausted: emit the pending tail once.
            if self.tail.iter().any(|&s| s != 0.0) {
                out.extend_from_slice(&self.tail);
                self.tail.iter_mut().for_each(|s| *s = 0.0);
                return self.hs;
            }
            return 0;
        }

        let best = self.best_frame(nominal);
        let n = self.n;
        let hs = self.hs;
        let start = out.len();
        for i in 0..n {
            let x = self.sample(best + i as u64) * self.window[i];
            if i < hs {
                out.push(self.tail[i] + x);
            } else {
                self.tail[i - hs] = x;
            }
        }
        self.prev = Some(best);
        self.ana_pos += hs as f64 * self.rate;
        self.compact();
        out.len() - start
    }

    fn sample(&self, abs: u64) -> f32 {
        abs.checked_sub(self.base)
            .and_then(|i| self.input.get(i as usize))
            .copied()
            .unwrap_or(0.0)
    }

    /// Chooses the frame near `nominal` whose start best continues the
    /// previous frame's natural successor (cross-correlation over the overlap).
    fn best_frame(&self, nominal: u64) -> u64 {
        let Some(prev) = self.prev else { return nominal };
        let natural = prev + self.hs as u64;
        if (self.rate - 1.0).abs() < 1e-3 {
            return natural;
        }
        let lo = nominal.saturating_sub(self.delta as u64).max(self.base);
        let hi = nominal + self.delta as u64;
        let corr = |cand: u64, stride: usize| -> f32 {
            let mut acc = 0.0f32;
            let mut i = 0;
            while i < self.hs {
                acc += self.sample(natural + i as u64) * self.sample(cand + i as u64);
                i += stride;
            }
            acc
        };
        // Coarse search, then refine around the winner.
        let mut best = nominal.max(lo);
        let mut best_c = f32::MIN;
        let mut c = lo;
        while c <= hi {
            let v = corr(c, 4);
            if v > best_c {
                best_c = v;
                best = c;
            }
            c += 4;
        }
        let (rlo, rhi) = (best.saturating_sub(3).max(lo), (best + 3).min(hi));
        let mut best_c = f32::MIN;
        for c in rlo..=rhi {
            let v = corr(c, 1);
            if v > best_c {
                best_c = v;
                best = c;
            }
        }
        best
    }

    /// Drops input that no future frame or template can reference.
    fn compact(&mut self) {
        let keep_from = (self.ana_pos as u64)
            .saturating_sub(self.delta as u64)
            .min(self.prev.map_or(u64::MAX, |p| p + self.hs as u64));
        let drop = keep_from.saturating_sub(self.base) as usize;
        if drop > 4096 && drop <= self.input.len() {
            self.input.drain(..drop);
            self.base += drop as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(rate: f32, input: &[f32]) -> Vec<f32> {
        let mut s = Stretcher::new(48_000);
        s.set_rate(rate);
        s.push(input);
        s.finish();
        let mut out = Vec::new();
        while s.process_hop(&mut out) > 0 {}
        out
    }

    fn sine(len: usize) -> Vec<f32> {
        (0..len).map(|i| (i as f32 * 2.0 * PI * 220.0 / 48_000.0).sin() * 0.5).collect()
    }

    #[test]
    fn unity_rate_reconstructs_input() {
        let input = sine(48_000);
        let out = run(1.0, &input);
        // Skip the first half-window, which fades in from silence.
        let hs = Stretcher::new(48_000).hs;
        for i in hs..input.len() - hs {
            assert!((out[i] - input[i]).abs() < 1e-4, "sample {i}");
        }
    }

    #[test]
    fn output_length_scales_with_rate() {
        let input = sine(96_000);
        for rate in [0.5f32, 1.5, 2.0, 3.0] {
            let out = run(rate, &input);
            let expected = input.len() as f32 / rate;
            let ratio = out.len() as f32 / expected;
            assert!((0.95..1.05).contains(&ratio), "rate {rate}: ratio {ratio}");
        }
    }

    #[test]
    fn preserves_pitch() {
        // Count zero crossings per second: pitch must not change with rate.
        let input = sine(96_000);
        let out = run(2.0, &input);
        let zc = |s: &[f32]| s.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32 / s.len() as f32;
        let a = zc(&input[4800..90_000]);
        let b = zc(&out[4800..out.len() - 4800]);
        assert!((a - b).abs() / a < 0.03, "{a} vs {b}");
    }

    #[test]
    fn streams_incrementally() {
        let input = sine(48_000);
        let mut s = Stretcher::new(48_000);
        s.set_rate(1.5);
        let mut out = Vec::new();
        for chunk in input.chunks(1000) {
            s.push(chunk);
            while s.process_hop(&mut out) > 0 {}
        }
        s.finish();
        while s.process_hop(&mut out) > 0 {}
        let ratio = out.len() as f32 / (input.len() as f32 / 1.5);
        assert!((0.95..1.05).contains(&ratio), "{ratio}");
    }

    #[test]
    fn reports_when_a_remainder_is_waiting_for_input() {
        for rate in [1.0, 1.3] {
            let mut s = Stretcher::new(48_000);
            s.set_rate(rate);
            s.push(&sine(10_000));
            let mut out = Vec::new();
            while s.can_process() {
                assert!(s.process_hop(&mut out) > 0, "can_process agrees with process_hop");
            }
            // A remainder is buffered but cannot play until the next segment.
            assert!(s.pending_input() > 0);
            assert_eq!(s.process_hop(&mut out), 0);
            s.push(&sine(10_000));
            assert!(s.can_process());
            s.finish();
            while s.can_process() {
                assert!(s.process_hop(&mut out) > 0);
            }
            assert_eq!(s.process_hop(&mut out), 0, "everything flushed at {rate}x");
        }
    }
}
