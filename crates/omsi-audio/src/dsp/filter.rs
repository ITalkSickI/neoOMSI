//! A one-pole low-pass per voice: the sound heard through the bodywork from the cabin
//! loses its edge, not just some volume. The cutoff is smoothed so a door opening (or the
//! cabin blend moving) does not click.

/// The top of the audible range; a cutoff here means the filter is off.
pub const OPEN_HZ: f32 = 20_000.0;

pub struct LowPass {
    /// The filter's current cutoff in Hz; 0 until the first target is set.
    cutoff: f32,
    /// Filter state, left/right.
    state: [f32; 2],
}

impl Default for LowPass {
    fn default() -> Self {
        LowPass {
            cutoff: 0.0,
            state: [0.0; 2],
        }
    }
}

impl LowPass {
    /// Move the cutoff towards `lowpass_hz` (0 = none) over about 120 ms.
    pub fn set_target(&mut self, lowpass_hz: f32, frames: usize, rate: f32) {
        let target = if lowpass_hz > 0.0 {
            lowpass_hz.min(OPEN_HZ)
        } else {
            OPEN_HZ
        };
        if self.cutoff <= 0.0 {
            self.cutoff = target;
        } else {
            let k = 1.0 - (-(frames as f32) / rate / 0.12).exp();
            self.cutoff = (self.cutoff.ln() + (target.ln() - self.cutoff.ln()) * k).exp();
        }
    }

    /// Whether the filter does anything (the cutoff is below the open end).
    pub fn is_on(&self) -> bool {
        self.cutoff < OPEN_HZ * 0.95
    }

    /// The one-pole coefficient for the current cutoff at `rate`.
    pub fn alpha(&self, rate: f32) -> f32 {
        1.0 - (-2.0 * std::f32::consts::PI * self.cutoff / rate).exp()
    }

    /// Filter one stereo frame. With the filter off the state simply follows the input.
    pub fn process(&mut self, l: f32, r: f32, alpha: f32) -> (f32, f32) {
        if self.is_on() {
            self.state[0] += (l - self.state[0]) * alpha;
            self.state[1] += (r - self.state[1]) * alpha;
            (self.state[0], self.state[1])
        } else {
            self.state = [l, r];
            (l, r)
        }
    }
}
