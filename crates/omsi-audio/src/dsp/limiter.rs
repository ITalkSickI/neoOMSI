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
        let ch = ch.max(1);
        let frames = (out.len() / ch).max(1);
        let release = (-1.0 / (0.5 * rate as f32)).exp();
        for f in 0..frames {
            let peak = (0..ch)
                .map(|c| out[f * ch + c].abs())
                .fold(0.0f32, f32::max);
            let want = if peak * self.gain > 0.9 {
                0.9 / peak
            } else {
                1.0
            };
            self.gain = if want < self.gain {
                want
            } else {
                want + (self.gain - want) * release
            };
            for c in 0..ch {
                out[f * ch + c] = soft_clip(out[f * ch + c] * self.gain);
            }
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
}
