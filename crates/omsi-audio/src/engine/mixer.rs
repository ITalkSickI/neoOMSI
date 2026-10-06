//! The audio thread's own state, owned exclusively by whoever calls [`AudioCore::render`]:
//! the output callback with a device, or the offline renderer on the caller's thread. It
//! holds the voice list, the listener, the cabin blend and the effects, drains the bounded
//! command queue at the top of every block and retires finished voices to the game thread.
//!
//! Real-time constraints (documented once, here): `render` must not block on a game or
//! decoder lock, must not allocate in steady state and must not touch a file. The queue is
//! drained with `try_lock`, the buffers it reuses are preallocated to [`VOICE_CAPACITY`] /
//! [`COMMAND_CAPACITY`], the parameter lookup scans the (bounded) voice list instead of
//! building a `HashMap`, finished voices go to the bounded reaper and the stream decoder lock
//! is only `try_lock`ed (a busy lock is one silent block, counted).

use crate::clock::Clock;
use crate::device::OutputFormat;
use crate::dsp::limiter::Limiter;
use crate::dsp::reverb::Reverb;
use crate::engine::commands::{Command, CommandQueue, COMMAND_CAPACITY};
use crate::engine::feedback::{Counters, Reaper};
use crate::spatial::Legacy;
use crate::voice::{doppler_enabled, Voice};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

// Keep the old `omsi_audio::mixer::…` paths working.
pub use crate::assets::Clip;
pub use crate::voice::{Listener, VoiceId, VoiceParams};

/// At most this many clip voices are mixed at once (OMSI's `[sound_maxcount]` default).
pub const MAX_VOICES: usize = 200;

/// The voice list is preallocated to this many entries so a normal frame does not grow it.
/// Beyond it the vector grows once (an allocation, counted nowhere because it is not
/// unexpected - a single reallocation, not per-block work).
const VOICE_CAPACITY: usize = 512;

pub(crate) struct AudioCore {
    voices: Vec<Voice>,
    listener: Listener,
    /// The cabin blend target and when it was set, and the smoothed value.
    cabin: (f32, Instant),
    cabin_s: f32,
    reverb: Reverb,
    limiter: Limiter,
    spatial: Legacy,
    muted: bool,
    clock: Clock,
    /// The output device's rate and channels, read while rendering.
    format: Arc<OutputFormat>,
    queue: Arc<CommandQueue>,
    reaper: Arc<Reaper>,
    counters: Arc<Counters>,
    /// Reused command drain buffer (see the module note).
    cmds: Vec<Command>,
    /// Reused ranking buffers.
    ranked: Vec<(bool, f32, usize)>,
    keep: Vec<bool>,
}

impl AudioCore {
    pub(crate) fn new(
        clock: Clock,
        format: Arc<OutputFormat>,
        queue: Arc<CommandQueue>,
        reaper: Arc<Reaper>,
        counters: Arc<Counters>,
        muted: bool,
    ) -> AudioCore {
        let cabin = (0.0, clock.now());
        AudioCore {
            voices: Vec::with_capacity(VOICE_CAPACITY),
            listener: Listener::default(),
            cabin,
            cabin_s: 0.0,
            reverb: Reverb::default(),
            limiter: Limiter::default(),
            spatial: Legacy,
            muted,
            clock,
            format,
            queue,
            reaper,
            counters,
            cmds: Vec::with_capacity(COMMAND_CAPACITY),
            ranked: Vec::with_capacity(VOICE_CAPACITY),
            keep: Vec::new(),
        }
    }

    /// Apply one command, in the order it was queued.
    fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Play { id, clip, params } => {
                self.voices.push(Voice::clip_voice(id, clip, params));
            }
            Command::PlayStream { id, stream, params } => {
                self.voices.push(Voice::stream_voice(id, stream, params));
            }
            Command::Stop { id } => {
                if let Some(v) = self.voices.iter_mut().find(|v| v.id() == id) {
                    v.finish();
                }
            }
            Command::SetParams { id, params, at } => {
                let listener = self.listener.position;
                let doppler = doppler_enabled();
                if let Some(v) = self.voices.iter_mut().find(|v| v.id() == id) {
                    v.apply_params(params, at, listener, doppler);
                }
            }
            Command::SetListener(l) => self.listener = l,
            Command::SetCabin { h, at } => {
                let h = h.clamp(0.0, 1.0);
                let age = at.saturating_duration_since(self.cabin.1).as_secs_f32();
                self.cabin.0 = if age > 0.05 { h } else { self.cabin.0.max(h) };
                self.cabin.1 = at;
            }
        }
    }

    /// Mix one block into `out` (interleaved, device order).
    pub(crate) fn render(&mut self, out: &mut [f32]) {
        for s in out.iter_mut() {
            *s = 0.0;
        }
        // Commands first: start/stop/parameters take effect this block, in order.
        let mut cmds = std::mem::take(&mut self.cmds);
        self.queue.drain_into(&mut cmds);
        for cmd in cmds.drain(..) {
            self.apply(cmd);
        }
        self.cmds = cmds;

        let listener = self.listener;
        let ch = self.format.channels();
        let frames = out.len() / ch;
        let rate = self.format.sample_rate();
        let dev_rate = rate as f64;
        // More voices than OMSI's `[sound_maxcount]` (200 by default): keep `[important]`
        // sounds first, then the ordinary voices that reach the listener loudest. Voices left
        // out still advance in time, so a loop comes back at the right phase.
        let over = self
            .voices
            .iter()
            .filter(|v| !v.is_finished() && !v.is_stream())
            .count()
            > MAX_VOICES;
        let mixed = if over {
            self.ranked.clear();
            self.ranked.extend(
                self.voices
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| !v.is_finished() && !v.is_stream())
                    .map(|(i, v)| (v.important(), v.heard_gain(&listener), i)),
            );
            self.ranked
                .sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.total_cmp(&a.1)));
            if self.keep.len() < self.voices.len() {
                self.keep.resize(self.voices.len(), false);
            }
            for k in self.keep.iter_mut() {
                *k = false;
            }
            for (_, _, i) in self.ranked.iter().take(MAX_VOICES) {
                self.keep[*i] = true;
            }
            true
        } else {
            false
        };
        let mut stalls = 0u64;
        for (i, v) in self.voices.iter_mut().enumerate() {
            if v.is_finished() {
                continue;
            }
            if !v.is_stream() && mixed && !self.keep[i] {
                v.skip(frames, dev_rate);
                continue;
            }
            if v.render_into(out, ch, rate, &listener, &self.spatial) {
                stalls += 1;
            }
        }
        if stalls > 0 {
            self.counters
                .stream_underruns
                .fetch_add(stalls, Ordering::Relaxed);
        }
        // Finished voices go to the game thread; the drop of a large clip never happens here.
        self.reaper.retire_finished(&mut self.voices, &self.counters);

        let cab = {
            let age = self
                .clock
                .now()
                .saturating_duration_since(self.cabin.1)
                .as_secs_f32();
            let target = if age < 0.25 { self.cabin.0 } else { 0.0 };
            self.cabin_s += (target - self.cabin_s)
                * (1.0 - (-(frames as f32) / rate as f32 / 0.15).exp());
            self.cabin_s
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
                .process(out, ch, rate, rt.min(3.0), mix.min(1.0));
        }
        self.limiter.process(out, ch, rate);
        for s in out.iter_mut() {
            *s = if self.muted { 0.0 } else { s.clamp(-1.0, 1.0) };
        }
    }
}

impl Drop for AudioCore {
    fn drop(&mut self) {
        // A device change drops a core on the game thread; report whatever ended so the game
        // can restart (see `AudioEngine::replay`). Active voices are replayed, not reported.
        self.reaper.retire_finished(&mut self.voices, &self.counters);
    }
}

/// Whether `OMSI_MUTE` is set (see [`AudioCore`]).
pub(crate) fn muted() -> bool {
    omsi_cfg::env::var_os("OMSI_MUTE").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::commands::CommandQueue;
    use crate::engine::feedback::{Counters, Reaper};
    use crate::voice::VoiceId;
    use std::sync::Arc;

    fn core() -> AudioCore {
        let counters = Arc::new(Counters::default());
        AudioCore::new(
            Clock::real(),
            Arc::new(OutputFormat::new(48_000, 1)),
            Arc::new(CommandQueue::new(counters.clone())),
            Arc::new(Reaper::new()),
            counters,
            false,
        )
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
        let mut s = core();
        s.voices.push(voice(clip, 1.0));
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
        let mut s = core();
        s.voices.push(voice(clip, 1.0));
        s.queue.push(Command::SetParams {
            id: 1,
            params: VoiceParams {
                gain: 0.0,
                looping: true,
                ..Default::default()
            }
            .into(),
            at: Instant::now(),
        });
        let mut out = vec![0.0f32; 4];
        s.render(&mut out);
        assert_eq!(s.voices[0].params().level.gain(), 0.0);
    }

    #[test]
    fn a_play_then_parameters_takes_effect_in_order() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![16_384; 5],
        });
        let mut s = core();
        s.queue.push(Command::Play {
            id: 5,
            clip,
            params: VoiceParams {
                gain: 1.0,
                looping: true,
                ..Default::default()
            }
            .into(),
        });
        s.queue.push(Command::SetParams {
            id: 5,
            params: VoiceParams {
                gain: 0.25,
                looping: true,
                ..Default::default()
            }
            .into(),
            at: Instant::now(),
        });
        let mut out = vec![0.0f32; 4];
        s.render(&mut out);
        assert_eq!(s.voices.len(), 1);
        assert_eq!(
            s.voices[0].params().level.gain(),
            0.25,
            "the parameters followed the start"
        );
    }

    #[test]
    fn a_finished_voice_is_retired_to_the_game_thread() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![16_384; 5],
        });
        let mut s = core();
        s.voices.push(voice(clip, 1.0));
        s.queue.push(Command::Stop { id: 1 });
        let mut out = vec![0.0f32; 4];
        s.render(&mut out);
        assert!(s.voices.is_empty(), "the stopped voice left the mixer");
        let retired = s.reaper.drain();
        assert_eq!(retired, vec![1 as VoiceId]);
    }

    #[test]
    fn important_voices_win_the_mixer_limit() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![64; 100],
        });
        let mut s = core();
        for _ in 0..MAX_VOICES {
            s.voices.push(voice(clip.clone(), 1.0));
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
        s.voices.push(quiet_important);
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
        let mut s = core();
        for k in 0..MAX_VOICES + 50 {
            s.voices
                .push(voice(clip.clone(), if k < 50 { 0.001 } else { 1.0 }));
        }
        let mut out = vec![0.0f32; 16];
        s.render(&mut out);
        // the 50 quiet ones stayed out: exactly MAX_VOICES at full gain
        let expect = MAX_VOICES as f32 * 64.0 / 32_768.0;
        assert!((out[0] - expect).abs() < 1e-3, "{} vs {expect}", out[0]);
        assert_eq!(s.voices.len(), MAX_VOICES + 50);
    }
}
