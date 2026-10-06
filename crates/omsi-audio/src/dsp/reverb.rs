//! A small Schroeder reverb (four combs in parallel, two all-passes in series, per channel)
//! for the echo under a bridge: the combs' feedback gives the reverberation time.

/// A Schroeder reverb; `process` reads and writes the interleaved buffer in place.
#[derive(Default)]
pub struct Reverb {
    lines: Vec<Vec<(Vec<f32>, usize, f32)>>,
    allpass: Vec<Vec<(Vec<f32>, usize)>>,
    rate: u32,
}

impl Reverb {
    const COMBS: [f32; 4] = [0.0297, 0.0371, 0.0411, 0.0437];
    const ALLPASS: [f32; 2] = [0.005, 0.0017];

    pub fn process(&mut self, out: &mut [f32], ch: usize, rate: u32, rt60: f32, mix: f32) {
        if self.rate != rate || self.lines.len() != ch {
            self.rate = rate;
            self.lines = (0..ch)
                .map(|c| {
                    Self::COMBS
                        .iter()
                        .map(|d| {
                            (
                                vec![0.0; ((d + c as f32 * 0.0011) * rate as f32) as usize + 1],
                                0,
                                0.0,
                            )
                        })
                        .collect()
                })
                .collect();
            self.allpass = (0..ch)
                .map(|_| {
                    Self::ALLPASS
                        .iter()
                        .map(|d| (vec![0.0; (d * rate as f32) as usize + 1], 0))
                        .collect()
                })
                .collect();
        }
        let frames = out.len() / ch;
        for c in 0..ch {
            let combs = &mut self.lines[c];
            let aps = &mut self.allpass[c];
            // feedback so that a comb decays by 60 dB in rt60 seconds
            let gains: Vec<f32> = Self::COMBS
                .iter()
                .map(|d| 10f32.powf(-3.0 * d / rt60))
                .collect();
            for f in 0..frames {
                let x = out[f * ch + c];
                let mut y = 0.0;
                for (k, (buf, i, lp)) in combs.iter_mut().enumerate() {
                    let d = buf[*i];
                    *lp = d * 0.8 + *lp * 0.2;
                    buf[*i] = x + *lp * gains[k];
                    *i = (*i + 1) % buf.len();
                    y += d;
                }
                y *= 0.25;
                for (buf, i) in aps.iter_mut() {
                    let d = buf[*i];
                    let v = y + d * 0.5;
                    buf[*i] = v;
                    *i = (*i + 1) % buf.len();
                    y = d - v * 0.5;
                }
                out[f * ch + c] = x + y * mix;
            }
        }
    }
}
