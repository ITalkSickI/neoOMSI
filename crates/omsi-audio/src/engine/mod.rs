//! The audio engine façade: it owns the output stream, the clip cache and the shared mixer
//! state, and is the only type the game talks to. The OMSI runtime talks to it through
//! [`Playback`] instead of the concrete type (see [`crate::runtime`]).

pub mod commands;
pub mod feedback;
pub mod mixer;
pub mod playback;

pub use playback::Playback;

use crate::assets::clip::{Clip, ClipCache};
use crate::assets::stream::StreamBuf;
use crate::clock::Clock;
use crate::device::{watch_default_device, DeviceOutput};
use crate::engine::mixer::Shared;
use crate::voice::{Listener, Voice, VoiceId, VoiceParams};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Owns the output stream. Dropping it stops playback.
pub struct AudioEngine {
    device: DeviceOutput,
    shared: Arc<Shared>,
    next_id: AtomicU64,
    /// Clips by file (see [`ClipCache`]).
    cache: Arc<ClipCache>,
    pub enabled: bool,
    clock: Clock,
}

impl AudioEngine {
    /// The shared mixer state of a fresh engine.
    fn shared(clock: Clock, sample_rate: u32, channels: usize) -> Arc<Shared> {
        Arc::new(Shared::new(clock, sample_rate, channels))
    }

    fn assemble(shared: Arc<Shared>, clock: Clock, enabled: bool) -> AudioEngine {
        let now = clock.now();
        let device = DeviceOutput::new(shared.format.clone(), now);
        AudioEngine {
            device,
            shared,
            next_id: AtomicU64::new(1),
            cache: Arc::new(ClipCache::new(now)),
            enabled,
            clock,
        }
    }

    /// Open the default output device. Returns a silent engine if none is available.
    pub fn new() -> AudioEngine {
        let clock = Clock::real();
        let shared = Self::shared(clock.clone(), 48_000, 2);
        let engine = Self::assemble(shared, clock, false);
        let enabled = engine.open_default();
        let engine = AudioEngine { enabled, ..engine };
        if enabled {
            watch_default_device(
                engine.device.name(),
                Arc::downgrade(&engine.device.reopen_flag()),
            );
        }
        engine
    }

    /// An engine without an output device, for tests and the offline renderer: playback is
    /// driven and mixed with [`AudioEngine::render_offline`]. It runs on a manual clock, so
    /// the caller moves time and a run is reproducible; [`AudioEngine::enabled`] is true so
    /// sound sets run as usual.
    pub fn new_offline(sample_rate: u32, channels: usize) -> AudioEngine {
        let clock = Clock::manual(Instant::now());
        let shared = Self::shared(clock.clone(), sample_rate, channels);
        Self::assemble(shared, clock, true)
    }

    /// The clock this engine runs on (see [`AudioEngine::new_offline`]).
    pub fn clock(&self) -> Clock {
        self.clock.clone()
    }

    /// Mix `out.len() / channels` frames straight into `out` (interleaved, device order):
    /// the offline counterpart of the output callback, for tests and [`AudioEngine::new_offline`].
    pub fn render_offline(&self, out: &mut [f32]) {
        self.shared.render(out);
    }

    /// Play on the system's default output device from now on. Returns whether a stream
    /// is playing.
    fn open_default(&self) -> bool {
        let shared = self.shared.clone();
        self.device.open(move |data| shared.render(data))
    }

    /// Follow the system's output: when its default device changed (headphones plugged in,
    /// a Bluetooth headset connected) or the one played on went away, the stream is opened
    /// again on the default device - the sound had stayed on the speakers, or stopped for
    /// good when the headset was disconnected. Cheap; called every frame.
    pub fn follow_device(&self) {
        if !self.enabled || self.device.opened_age() < 1.0 || !self.device.reopen_flag().swap(false, Ordering::Relaxed)
        {
            return;
        }
        self.device.mark_opened(Instant::now());
        let before = self.device.name();
        if self.open_default() {
            let now = self.device.name();
            if now != before {
                log::info!("audio: output moved from {before} to {now}");
            }
        } else {
            // (no device right now: tried again when the watcher sees one)
            self.device.clear_name();
        }
    }

    /// Put an already decoded clip into the cache under `path`: later [`AudioEngine::load_clip`]
    /// calls for it return this one instead of reading a file. Tests and the offline renderer
    /// use it to supply synthetic sounds for a sound set.
    pub fn cache_clip(&self, path: impl Into<PathBuf>, clip: Arc<Clip>) {
        self.cache.insert(path, clip);
    }

    /// A clip from the cache, read now if it is not there.
    pub fn load_clip(&self, path: &Path) -> Option<Arc<Clip>> {
        self.cache.load(path)
    }

    /// Let go of the clips nobody holds (no sound set, no voice) and nobody asked for in
    /// `unused`, once every ten seconds at most. Returns the bytes let go.
    pub fn trim_clips(&self, unused: std::time::Duration) -> usize {
        self.cache.trim(unused)
    }

    /// Whether all of `paths` are in the cache (or known to be missing). The ones that are
    /// not are read on a background thread, started on the first call.
    pub fn clips_ready(&self, paths: &[PathBuf]) -> bool {
        ClipCache::ready(&self.cache, paths, self.enabled)
    }

    pub fn play(&self, clip: Arc<Clip>, params: VoiceParams) -> VoiceId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.shared
            .voices
            .lock()
            .push(Voice::clip_voice(id, clip, params));
        id
    }

    /// Play what `stream` is fed with (see [`crate::assets::stream::StreamBuf`]); the voice
    /// ends when the stream is closed.
    pub fn play_stream(&self, stream: Arc<StreamBuf>, params: VoiceParams) -> VoiceId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.shared
            .voices
            .lock()
            .push(Voice::stream_voice(id, stream, params));
        id
    }

    /// New parameters for a voice: queued for the mixer's next block (a few milliseconds),
    /// so the game never waits for a block being mixed. Given twice before that block, the
    /// later ones win.
    pub fn set_params(&self, id: VoiceId, params: VoiceParams) {
        let now = self.clock.now();
        // (no device, no mixer to take them in: straight onto the voice)
        if !self.enabled {
            let listener = self.shared.listener.lock().position;
            let doppler = crate::voice::doppler_enabled();
            if let Some(v) = self.shared.voices.lock().iter_mut().find(|v| v.id() == id) {
                v.apply_params(params, now, listener, doppler);
            }
            return;
        }
        self.shared.updates.push(id, params, now);
    }

    pub fn stop(&self, id: VoiceId) {
        if let Some(v) = self.shared.voices.lock().iter_mut().find(|v| v.id() == id) {
            v.finish();
        }
    }

    pub fn set_cabin(&self, h: f32) {
        let now = self.clock.now();
        let mut c = self.shared.cabin.lock();
        let h = h.clamp(0.0, 1.0);
        let age = now.saturating_duration_since(c.1).as_secs_f32();
        c.0 = if age > 0.05 { h } else { c.0.max(h) };
        c.1 = now;
    }

    pub fn set_listener(&self, l: Listener) {
        *self.shared.listener.lock() = l;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// The stream opened again (as when the system's output device changed) keeps playing;
    /// on a machine without an output device there is nothing to follow.
    #[test]
    fn follows_the_output_device() {
        let e = AudioEngine::new();
        if !e.enabled {
            return;
        }
        let before = e.device.name();
        e.device.reopen_flag().store(true, Ordering::Relaxed);
        e.device.mark_opened(Instant::now() - Duration::from_secs(2));
        e.follow_device();
        assert!(e.device.is_open());
        assert_eq!(e.device.name(), before);
        assert!(!e.device.reopen_flag().load(Ordering::Relaxed));
    }
}
