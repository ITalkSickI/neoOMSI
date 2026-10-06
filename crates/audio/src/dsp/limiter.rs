//! The master limiter: a busy street sums past full scale, and cut off hard there the sound
//! crackled and squeaked. Loud moments are turned down (at once) and back up (over half a
//! second), and what still peaks is rounded off, not cut.

/// Past 0.9 a sample is bent smoothly towards 1 instead of being cut off there.
pub fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.9 {
        x
    } else {
        x.signum() * (0.9 + 0.1 * ((a - 0.9) / 0.1).tanh())
    }
}

pub struct Limiter {
    /// The gain now (1 = none).
    gain: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        Limiter { gain: 1.0 }
    }
}

impl Limiter {
    pub fn process(&mut self, out: &mut [f32], ch: usize, rate: u32) {
        let release = (-1.0 / (0.5 * rate.max(1) as f32)).exp();
        for frame in out.chunks_exact_mut(ch.max(1)) {
            for sample in frame.iter_mut() { if !sample.is_finite() { *sample = 0.0; } }
            let peak = frame.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            let want = if peak > 0.9 { 0.9 / peak } else { 1.0 };
            self.gain = if want < self.gain { want }
                else { want + (self.gain - want) * release };
            for sample in frame { *sample = soft_clip(*sample * self.gain); }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_clip_is_smooth_and_bounded() {
        assert_eq!(soft_clip(0.5), 0.5);
        assert!(soft_clip(3.0) <= 1.0 && soft_clip(-3.0) >= -1.0);
        assert!(soft_clip(0.95) > 0.9 && soft_clip(0.95) < 0.95);
    }
    #[test]
    fn dense_stereo_is_linked_bounded_and_recovers_in_sample_time() {
        let mut limiter = Limiter::default();
        let mut busy = vec![0.0; 4800 * 2];
        for f in busy.chunks_exact_mut(2) { f.copy_from_slice(&[100.0, 50.0]); }
        limiter.process(&mut busy, 2, 48000);
        assert!(busy.iter().all(|s| s.abs() <= 0.900001));
        for f in busy.chunks_exact(2) { assert!((f[0] / f[1] - 2.0).abs() < 1e-5); }
        let mut quiet = vec![0.1; 48000 * 2]; limiter.process(&mut quiet, 2, 48000);
        assert!(quiet.last().unwrap() > &0.085);
        limiter.process(&mut [], 2, 48000);
    }

}
