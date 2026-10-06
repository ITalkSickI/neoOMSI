//! A playing voice: the clip (or stream) it reads, its parameters and its position, and the
//! per-block mixing of one voice into the output buffer. The reverb, limiter and cabin blend
//! that surround this are the mixer's job (see [`crate::engine::mixer`]).

use crate::assets::{stream::StreamBuf, Clip};
use crate::dsp::{filter::LowPass, resample};
use crate::spatial::{self, Spatializer};
use crate::voice::params::{Listener, VoiceId, VoiceParams, DOPPLER};
use std::sync::Arc;
use std::time::Instant;

pub struct Voice {
    id: VoiceId,
    clip: Arc<Clip>,
    /// A voice fed while it plays (internet radio) instead of from `clip`.
    pub(crate) stream: Option<Arc<StreamBuf>>,
    params: VoiceParams,
    pos: f64,
    finished: bool,
    /// Smoothed gain to avoid clicks.
    cur_gain: f32,
    /// One-pole low-pass filter state.
    lp: LowPass,
    /// The Doppler shift: the distance to the listener when the position last came, when,
    /// and the (smoothed) pitch factor it gives.
    doppler: (f32, Option<Instant>, f32),
}

impl Voice {
    /// A voice playing `clip`.
    pub fn clip_voice(id: VoiceId, clip: Arc<Clip>, params: VoiceParams) -> Voice {
        Voice {
            id,
            clip,
            stream: None,
            params,
            pos: 0.0,
            finished: false,
            cur_gain: 0.0,
            lp: LowPass::default(),
            doppler: (0.0, None, 1.0),
        }
    }

    /// A voice playing what `stream` is fed with; the voice ends when the stream is closed.
    pub fn stream_voice(id: VoiceId, stream: Arc<StreamBuf>, params: VoiceParams) -> Voice {
        Voice {
            id,
            clip: Arc::new(Clip {
                sample_rate: 44100,
                channels: 2,
                samples: Vec::new(),
            }),
            stream: Some(stream),
            params,
            pos: 0.0,
            finished: false,
            cur_gain: 0.0,
            lp: LowPass::default(),
            doppler: (0.0, None, 1.0),
        }
    }

    pub fn id(&self) -> VoiceId {
        self.id
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn is_stream(&self) -> bool {
        self.stream.is_some()
    }

    pub fn params(&self) -> VoiceParams {
        self.params
    }

    pub fn important(&self) -> bool {
        self.params.important
    }

    pub fn finish(&mut self) {
        self.finished = true;
    }

    /// A voice whose smoothed gain already sits at `params.gain` (tests that expect the
    /// block to carry a steady level from the first frame).
    #[cfg(test)]
    pub(crate) fn test_voice(id: VoiceId, clip: Arc<Clip>, params: VoiceParams) -> Voice {
        let mut v = Voice::clip_voice(id, clip, params);
        v.cur_gain = params.gain;
        v
    }

    /// New parameters for this voice, given at `now` with the listener at `listener`: the
    /// Doppler shift from how fast the distance to the listener changes (the bus's own sounds
    /// move with the listener and keep their pitch). `doppler_enabled` is the outward
    /// `DOPPLER` switch read once by the engine.
    pub fn apply_params(
        &mut self,
        params: VoiceParams,
        now: Instant,
        listener: glam::Vec3,
        doppler_enabled: bool,
    ) {
        if let (Some(p), true) = (params.position, params.doppler && doppler_enabled) {
            let dist = (p - listener).length();
            let (last, at, factor) = self.doppler;
            let mut f = factor;
            if let Some(at) = at {
                let dt = now.saturating_duration_since(at).as_secs_f32();
                if (0.004..0.5).contains(&dt) {
                    let raw = (dist - last) / dt;
                    if raw.abs() < 80.0 {
                        let target = 343.0 / (343.0 + raw);
                        f += (target - f) * (dt / 0.25).min(1.0);
                    }
                }
            }
            self.doppler = (dist, Some(now), f);
        } else {
            self.doppler = (0.0, None, 1.0);
        }
        self.params = params;
    }

    /// How loud this voice reaches `listener` (its gain and distance), to rank voices by.
    pub fn heard_gain(&self, listener: &Listener) -> f32 {
        let spatial = self
            .params
            .position
            .map(|p| spatial::distance_gain(self.params.range, (p - listener.position).length()))
            .unwrap_or(1.0);
        self.params.gain * spatial
    }

    /// Move the voice on by `frames` output frames without mixing it (looping or ending as
    /// it would have).
    pub fn skip(&mut self, frames: usize, dev_rate: f64) {
        let nframes = self.clip.frames();
        if nframes == 0 {
            self.finished = true;
            return;
        }
        let step = resample::step(
            self.params.pitch,
            self.doppler.2,
            self.clip.sample_rate,
            dev_rate,
        );
        self.pos += step * frames as f64;
        self.cur_gain = 0.0;
        if self.pos >= nframes as f64 {
            if self.params.looping {
                self.pos %= nframes as f64;
            } else {
                self.finished = true;
            }
        }
    }

    /// Mix this voice into the interleaved output, `ch` channels at `rate`.
    pub fn render_into(
        &mut self,
        out: &mut [f32],
        ch: usize,
        rate: u32,
        listener: &Listener,
        spatializer: &dyn Spatializer,
    ) {
        let dev_rate = rate as f64;
        let frames = out.len() / ch;
        let placed = spatializer.place(
            self.params.position,
            self.params.range,
            self.params.pan,
            listener.position,
            listener.right,
        );
        let target_gain = (self.params.gain * placed.gain * listener.master).max(0.0);
        self.lp
            .set_target(self.params.lowpass_hz, frames, rate as f32);
        let lp_on = self.lp.is_on();
        let lp_alpha = if lp_on {
            self.lp.alpha(rate as f32)
        } else {
            1.0
        };
        if let Some(sb) = self.stream.clone() {
            let mut buf = sb.lock();
            if buf.closed {
                self.finished = true;
                return;
            }
            let step = buf.rate as f64 / dev_rate;
            for f in 0..frames {
                self.cur_gain += (target_gain - self.cur_gain) * 0.0025;
                // silence while the buffer fills (at the start, after a stall)
                let Some((a, b)) = buf.pair() else { break };
                let t = self.pos as f32;
                let (l, r) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
                let (l, r) = self.lp.process(l, r, lp_alpha);
                let g = self.cur_gain;
                out[f * ch] += l * g * placed.left;
                if ch > 1 {
                    out[f * ch + 1] += r * g * placed.right;
                }
                self.pos += step;
                while self.pos >= 1.0 {
                    self.pos -= 1.0;
                    buf.advance();
                }
            }
            return;
        }
        let cch = self.clip.channels as usize;
        let nframes = self.clip.frames();
        if nframes == 0 {
            self.finished = true;
            return;
        }
        let step = resample::step(
            self.params.pitch,
            self.doppler.2,
            self.clip.sample_rate,
            dev_rate,
        );
        for f in 0..frames {
            // smooth gain over ~5 ms
            self.cur_gain += (target_gain - self.cur_gain) * 0.005;
            let mut i0 = self.pos as usize;
            if i0 >= nframes {
                if !self.params.looping {
                    self.finished = true;
                    break;
                }
                // wrap and mix this output frame from the loop's start (skipping it left
                // a silent frame at every turn of the loop: a click on every engine loop)
                self.pos %= nframes as f64;
                i0 = (self.pos as usize).min(nframes - 1);
            }
            let i1 = if i0 + 1 < nframes {
                i0 + 1
            } else if self.params.looping {
                0
            } else {
                i0
            };
            let t = (self.pos - i0 as f64) as f32;
            let (l, r) = resample::frame(&self.clip, i0, i1, t, cch);
            let (l, r) = self.lp.process(l, r, lp_alpha);
            let g = self.cur_gain;
            out[f * ch] += l * g * placed.left;
            if ch > 1 {
                out[f * ch + 1] += r * g * placed.right;
            }
            self.pos += step;
        }
    }
}

/// Read the outward `DOPPLER` switch once, for passing to [`Voice::apply_params`].
pub fn doppler_enabled() -> bool {
    DOPPLER.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn listener_vehicle_keeps_spatial_sound_at_its_original_pitch() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![0; 5],
        });
        let params = |position, doppler| VoiceParams {
            position,
            doppler,
            ..Default::default()
        };
        let mut own = Voice::clip_voice(1, clip.clone(), params(None, false));
        let mut passing = Voice::clip_voice(2, clip, params(None, false));
        let now = Instant::now();
        for (distance, elapsed) in [(2.0, 0), (2.2, 20)] {
            let at = now + std::time::Duration::from_millis(elapsed);
            let position = Some(Vec3::new(distance, 0.0, 0.0));
            own.apply_params(params(position, false), at, Vec3::ZERO, true);
            passing.apply_params(params(position, true), at, Vec3::ZERO, true);
        }
        assert_eq!(own.doppler.2, 1.0);
        assert!(passing.doppler.2 < 1.0);
    }
}
