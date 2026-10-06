//! The audio thread's own state, owned exclusively by whoever calls [`AudioCore::render`]:
//! the output callback with a device, or the offline renderer on the caller's thread. It
//! holds the voice list, the listener, the cabin blend and the effects, drains the bounded
//! command queue at the top of every block and retires finished voices to the game thread.
//!
//! Real-time constraints (documented once, here): `render` must not block on a game or
//! decoder lock, must not allocate in steady state and must not touch a file. The queue is
//! drained with `try_lock`, the buffers it reuses are preallocated to [`VOICE_CAPACITY`] /
//! [`COMMAND_CAPACITY`], the parameter lookup scans the (bounded) voice list instead of
//! building a `HashMap`, complete finished voices go to the bounded reaper, and radio reads
//! atomic ring slots without a decoder lock.

use crate::clock::Clock;
use crate::device::OutputFormat;
use crate::dsp::limiter::Limiter;
use crate::dsp::{envelope, resample::Kernel};
use crate::voice::bus::{BUS_COUNT, DEFAULT_GAINS, HEADROOM};
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

/// Hard storage bound, including streams, virtual voices and stop tails. Excess starts
/// are counted and returned to the game thread without growing the callback storage.
pub(crate) const VOICE_CAPACITY: usize = 512;
const BLOCK_FRAMES: usize = 256;

pub(crate) struct AudioCore {
    voices: Vec<Voice>,
    kernel: Arc<Kernel>,
    buses: [[f32; BLOCK_FRAMES * 2]; BUS_COUNT],
    bus_gains: [f32; BUS_COUNT],
    bus_targets: [f32; BUS_COUNT],
    stereo: [f32; BLOCK_FRAMES * 2],
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
    rejected: Vec<Command>,
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
        let mut reverb = Reverb::default();
        reverb.prepare(2, format.sample_rate());
        AudioCore {
            voices: Vec::with_capacity(VOICE_CAPACITY),
            kernel: Arc::new(Kernel::default()),
            buses: [[0.0; BLOCK_FRAMES * 2]; BUS_COUNT],
            bus_gains: DEFAULT_GAINS, bus_targets: DEFAULT_GAINS,
            stereo: [0.0; BLOCK_FRAMES * 2],
            listener: Listener::default(),
            cabin,
            cabin_s: 0.0,
            reverb,
            limiter: Limiter::default(),
            spatial: Legacy,
            muted,
            clock,
            format,
            queue,
            reaper,
            counters,
            cmds: Vec::with_capacity(COMMAND_CAPACITY),
            rejected: Vec::with_capacity(COMMAND_CAPACITY),
            ranked: Vec::with_capacity(VOICE_CAPACITY),
            keep: Vec::with_capacity(VOICE_CAPACITY),
        }
    }

    /// Apply one command, in the order it was queued.
    fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Play { id, clip, params } => {
                self.voices.push(Voice::clip_with_kernel(id, clip, params, self.kernel.clone()));
            }
            Command::PlayStream { id, stream, params } => {
                self.voices.push(Voice::stream_with_kernel(id, stream, params, self.kernel.clone()));
            }
            Command::Stop { id } => {
                if let Some(v) = self.voices.iter_mut().find(|v| v.id() == id) {
                    v.stop();
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
            Command::SetBus { bus, gain } => self.bus_targets[bus as usize] = gain,
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
        let mut rejected = std::mem::take(&mut self.rejected);
        for cmd in rejected.drain(..) {
            if let Err(cmd) = self.reaper.reject(cmd) { cmds.push(cmd); }
        }
        std::mem::swap(&mut cmds, &mut rejected);
        if rejected.is_empty() {
            self.queue.drain_into(&mut cmds);
            for cmd in cmds.drain(..) {
                if self.voices.len() >= VOICE_CAPACITY && matches!(cmd, Command::Play { .. } | Command::PlayStream { .. }) {
                    self.counters.dropped_commands.fetch_add(1, Ordering::Relaxed);
                    if let Err(cmd) = self.reaper.reject(cmd) { rejected.push(cmd); }
                } else { self.apply(cmd); }
            }
        }
        self.cmds = cmds;
        self.rejected = rejected;

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
                .sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.total_cmp(&a.1)).then_with(|| a.2.cmp(&b.2)));
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
        // A cabin flag is a bodywork hint, not a reverb preset. The forced 0.22 wet
        // mix coloured every switch/button and loop with a synthetic noisy tail.
        // Only explicitly authored world/trigger-box reverb controls the shared effect.
        let _cab = cab;
        let (rt, mix) = (listener.reverb_time, listener.reverb_mix);
        let bus_k = envelope::coefficient(rate, 0.02);
        let mut stalls = 0u64;
        for chunk in out.chunks_mut(BLOCK_FRAMES * ch) {
            let count = chunk.len() / ch;
            let samples = count * 2;
            for bus in &mut self.buses { bus[..samples].fill(0.0); }
            for (i, voice) in self.voices.iter_mut().enumerate() {
                if voice.is_finished() { continue; }
                if !voice.is_stream() && mixed && !self.keep[i] { voice.skip(count, dev_rate); continue; }
                let bus = voice.params().bus as usize;
                if voice.render_into(&mut self.buses[bus][..samples], 2, rate, &listener, &self.spatial) { stalls += 1; }
            }
            self.stereo[..samples].fill(0.0);
            for f in 0..count {
                for bus in 0..BUS_COUNT {
                    self.bus_gains[bus] += (self.bus_targets[bus] - self.bus_gains[bus]) * bus_k;
                    for c in 0..2 {
                        self.stereo[f * 2 + c] += self.buses[bus][f * 2 + c] * self.bus_gains[bus] * HEADROOM;
                    }
                }
            }
            if mix > 0.001 && rt > 0.05 {
                self.reverb.process(&mut self.stereo[..samples], 2, rate, rt.min(3.0), mix.min(1.0));
            }
            self.limiter.process(&mut self.stereo[..samples], 2, rate);
            for f in 0..count {
                let (l, r) = (self.stereo[f * 2], self.stereo[f * 2 + 1]);
                if !self.muted {
                    if ch == 1 { chunk[f] = (l + r) * 0.5; }
                    else { chunk[f * ch] = l; chunk[f * ch + 1] = r; }
                }
            }
        }
        if stalls > 0 { self.counters.stream_underruns.fetch_add(stalls, Ordering::Relaxed); }
        self.reaper.retire_finished(&mut self.voices, &self.counters);

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
#[path = "mixer_tests.rs"]
mod tests;
