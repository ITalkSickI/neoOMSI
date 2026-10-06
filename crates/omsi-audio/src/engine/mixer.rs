//! The audio-thread state: the voice list, the listener and the effects around the mix, and
//! the per-block `render` the output callback (or the offline renderer) drives. The
//! per-voice per-sample work lives in [`crate::voice`], the primitives in [`crate::dsp`].

use crate::clock::Clock;
use crate::device::OutputFormat;
use crate::dsp::limiter::Limiter;
use crate::dsp::reverb::Reverb;
use crate::engine::commands::VoiceUpdates;
use crate::spatial::Legacy;
use crate::voice::{doppler_enabled, Voice};
use hashbrown::HashMap;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;

// Keep the old `omsi_audio::mixer::…` paths working.
pub use crate::assets::Clip;
pub use crate::voice::{Listener, VoiceId, VoiceParams};

/// At most this many clip voices are mixed at once (OMSI's `[sound_maxcount]` default).
pub const MAX_VOICES: usize = 200;

/// State shared between the game thread (which queues plays and parameters) and the mixer.
pub(crate) struct Shared {
    pub(crate) voices: Mutex<Vec<Voice>>,
    /// New parameters for voices, taken in by the mixer at the start of its next block.
    pub(crate) updates: VoiceUpdates,
    pub(crate) listener: Mutex<Listener>,
    /// The echo of an underpass, fed from the mix.
    reverb: Mutex<Reverb>,
    /// The master limiter.
    limiter: Mutex<Limiter>,
    pub(crate) cabin: Mutex<(f32, Instant)>,
    cabin_s: Mutex<f32>,
    /// The output device's rate and channels (see [`crate::device`]).
    pub(crate) format: Arc<OutputFormat>,
    /// Time source for Doppler, the cabin blend and clip idle: the wall clock in normal
    /// playback, a manual clock offline (see [`crate::clock`]).
    clock: Clock,
    /// `OMSI_MUTE`: everything is mixed as usual (voices play and end), nothing is heard -
    /// for test runs on a machine somebody is working at.
    muted: bool,
    /// The legacy OMSI/DirectSound placement (see [`crate::spatial`]).
    spatial: Legacy,
}

impl Shared {
    /// A fresh mixer state for an engine at `sample_rate`/`channels`.
    pub(crate) fn new(clock: Clock, sample_rate: u32, channels: usize) -> Shared {
        let cabin = (0.0, clock.now());
        Shared {
            voices: Mutex::new(Vec::new()),
            updates: VoiceUpdates::new(),
            listener: Mutex::new(Listener::default()),
            reverb: Mutex::new(Reverb::default()),
            limiter: Mutex::new(Limiter::default()),
            cabin: Mutex::new(cabin),
            cabin_s: Mutex::new(0.0),
            format: Arc::new(OutputFormat::new(sample_rate, channels)),
            clock,
            muted: muted(),
            spatial: Legacy,
        }
    }

    pub(crate) fn render(&self, out: &mut [f32]) {
        for s in out.iter_mut() {
            *s = 0.0;
        }
        let listener = *self.listener.lock();
        let updates = self.updates.take();
        let mut voices = self.voices.lock();
        if !updates.is_empty() {
            let index: HashMap<VoiceId, usize> =
                voices.iter().enumerate().map(|(k, v)| (v.id(), k)).collect();
            let doppler = doppler_enabled();
            for (id, params, at) in updates {
                if let Some(&k) = index.get(&id) {
                    voices[k].apply_params(params, at, listener.position, doppler);
                }
            }
        }
        let ch = self.format.channels();
        let frames = out.len() / ch;
        let rate = self.format.sample_rate();
        let dev_rate = rate as f64;
        // More voices than OMSI's `[sound_maxcount]` (200 by default): keep `[important]`
        // sounds first, then the ordinary voices that reach the listener loudest. Voices
        // left out still advance in time, so a loop comes back at the right phase.
        let mixed: Option<Vec<bool>> = if voices
            .iter()
            .filter(|v| !v.is_finished() && !v.is_stream())
            .count()
            > MAX_VOICES
        {
            let mut ranked: Vec<(bool, f32, usize)> = voices
                .iter()
                .enumerate()
                .filter(|(_, v)| !v.is_finished() && !v.is_stream())
                .map(|(i, v)| (v.important(), v.heard_gain(&listener), i))
                .collect();
            ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.total_cmp(&a.1)));
            let mut keep = vec![false; voices.len()];
            for (_, _, i) in ranked.into_iter().take(MAX_VOICES) {
                keep[i] = true;
            }
            Some(keep)
        } else {
            None
        };
        for (i, v) in voices.iter_mut().enumerate() {
            if v.is_finished() {
                continue;
            }
            if !v.is_stream() && mixed.as_ref().is_some_and(|keep| !keep[i]) {
                v.skip(frames, dev_rate);
                continue;
            }
            v.render_into(out, ch, rate, &listener, &self.spatial);
        }
        voices.retain(|v| !v.is_finished());
        drop(voices);
        let cab = {
            let (v, at) = *self.cabin.lock();
            let age = self.clock.now().saturating_duration_since(at).as_secs_f32();
            let target = if age < 0.25 { v } else { 0.0 };
            let mut sm = self.cabin_s.lock();
            *sm += (target - *sm) * (1.0 - (-(frames as f32) / rate as f32 / 0.15).exp());
            *sm
        };
        let (rt, mix) = if cab > 0.01 {
            (
                listener.reverb_time.max(0.4),
                listener.reverb_mix.max(0.22 * cab),
            )
        } else {
            (listener.reverb_time, listener.reverb_mix)
        };
        if mix > 0.001 && rt > 0.05 {
            self.reverb
                .lock()
                .process(out, ch, rate, rt.min(3.0), mix.min(1.0));
        }
        self.limiter.lock().process(out, ch, rate);
        for s in out.iter_mut() {
            *s = if self.muted { 0.0 } else { s.clamp(-1.0, 1.0) };
        }
    }
}

/// Whether `OMSI_MUTE` is set (see [`Shared`]).
pub(crate) fn muted() -> bool {
    omsi_cfg::env::var_os("OMSI_MUTE").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared() -> Shared {
        Shared {
            voices: Mutex::new(Vec::new()),
            updates: VoiceUpdates::new(),
            listener: Mutex::new(Listener::default()),
            reverb: Mutex::new(Reverb::default()),
            limiter: Mutex::new(Limiter::default()),
            cabin: Mutex::new((0.0, Instant::now())),
            cabin_s: Mutex::new(0.0),
            format: Arc::new(OutputFormat::new(48_000, 1)),
            clock: Clock::real(),
            muted: false,
            spatial: Legacy,
        }
    }

    fn voice(clip: Arc<Clip>, gain: f32) -> Voice {
        Voice::test_voice(
            1,
            clip,
            VoiceParams {
                gain,
                pitch: 1.0,
                looping: true,
                position: None,
                doppler: true,
                range: 10.0,
                lowpass_hz: 0.0,
                important: false,
                pan: 1.0,
            },
        )
    }

    #[test]
    fn a_loop_has_no_gap_where_it_turns() {
        // a 5-frame loop of a constant level at the device rate: every output frame carries it
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![16_384; 5],
        });
        let s = shared();
        s.voices.lock().push(voice(clip, 1.0));
        let mut out = vec![0.0f32; 64];
        s.render(&mut out);
        assert!(out.iter().all(|x| (*x - 0.5).abs() < 1e-3), "{out:?}");
    }

    #[test]
    fn parameters_arrive_with_the_next_block() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![16_384; 5],
        });
        let s = shared();
        s.voices.lock().push(voice(clip, 1.0));
        s.updates.push(
            1,
            VoiceParams {
                gain: 0.0,
                looping: true,
                ..Default::default()
            },
            Instant::now(),
        );
        let mut out = vec![0.0f32; 4];
        s.render(&mut out);
        assert_eq!(s.voices.lock()[0].params().gain, 0.0);
        assert!(s.updates.is_empty());
    }

    #[test]
    fn important_voices_win_the_mixer_limit() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![64; 100],
        });
        let s = shared();
        for _ in 0..MAX_VOICES {
            s.voices.lock().push(voice(clip.clone(), 1.0));
        }
        let quiet_important = Voice::test_voice(
            9_999,
            clip,
            VoiceParams {
                gain: 0.001,
                pitch: 1.0,
                looping: true,
                position: None,
                doppler: true,
                range: 10.0,
                lowpass_hz: 0.0,
                important: true,
                pan: 1.0,
            },
        );
        s.voices.lock().push(quiet_important);
        let mut out = vec![0.0f32; 16];
        s.render(&mut out);
        let expect = ((MAX_VOICES - 1) as f32 + 0.001) * 64.0 / 32_768.0;
        assert!((out[0] - expect).abs() < 1e-4, "{} vs {expect}", out[0]);
    }

    #[test]
    fn only_the_loudest_voices_are_mixed() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![64; 100],
        });
        let s = shared();
        for k in 0..MAX_VOICES + 50 {
            s.voices
                .lock()
                .push(voice(clip.clone(), if k < 50 { 0.001 } else { 1.0 }));
        }
        let mut out = vec![0.0f32; 16];
        s.render(&mut out);
        // the 50 quiet ones stayed out: exactly MAX_VOICES at full gain
        let expect = MAX_VOICES as f32 * 64.0 / 32_768.0;
        assert!((out[0] - expect).abs() < 1e-3, "{} vs {expect}", out[0]);
        assert_eq!(s.voices.lock().len(), MAX_VOICES + 50);
    }
}
