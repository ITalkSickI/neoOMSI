//! The audio engine façade: it owns the output stream, the clip cache, the command channel to
//! the audio thread and the game-side playback map, and is the only type the game talks to.
//! The audio thread (or, offline, the caller's thread) owns the [`mixer::AudioCore`] and its
//! voices; the game never reaches into it. The OMSI runtime talks to the engine through
//! [`Playback`] instead of the concrete type (see [`crate::runtime`]).

pub mod commands;
pub mod feedback;
pub mod mixer;
pub mod playback;

pub use playback::Playback;

use crate::assets::clip::{Clip, ClipCache};
use crate::assets::stream::StreamBuf;
use crate::clock::Clock;
use crate::device::{watch_default_device, DeviceOutput, OutputFormat};
use crate::engine::commands::{Command, CommandQueue};
use crate::engine::feedback::{ActiveVoice, Counters, Reaper, VoiceAsset};
use crate::engine::mixer::AudioCore;
use crate::voice::{Listener, VoiceId, VoiceParams};
use hashbrown::HashMap;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Where the mixer lives: the output callback owns it with a device, the engine owns it
/// offline (and without a device) so `render_offline` can drive the same code synchronously.
enum Mode {
    /// No callback: `render_offline` mixes on the caller's thread.
    Offline(RefCell<AudioCore>),
    /// A callback is (or was) playing; the core lives inside the stream's closure. A device
    /// change builds a fresh core and replays the still-active voices (see `replay`).
    Device(DeviceOutput),
}

/// Owns the output stream. Dropping it stops playback.
pub struct AudioEngine {
    mode: Mode,
    commands: Arc<CommandQueue>,
    counters: Arc<Counters>,
    reaper: Arc<Reaper>,
    /// Clips by file (see [`ClipCache`]).
    cache: Arc<ClipCache>,
    /// The voices the game started and still believes are playing. Written and read on the
    /// game thread only; the audio thread reports ends through the reaper.
    active: RefCell<HashMap<VoiceId, ActiveVoice>>,
    /// The listener the game last set, for [`AudioEngine::listener_position`] and the
    /// replies; the audio thread keeps its own copy, fed by `SetListener`.
    listener: Cell<Listener>,
    next_id: AtomicU64,
    pub enabled: bool,
    clock: Clock,
}

impl AudioEngine {
    /// Open the default output device. Returns a silent engine if none is available.
    pub fn new() -> AudioEngine {
        let clock = Clock::real();
        let counters = Arc::new(Counters::default());
        let commands = Arc::new(CommandQueue::new(counters.clone()));
        let reaper = Arc::new(Reaper::new());
        let device = DeviceOutput::new(Arc::new(OutputFormat::new(48_000, 2)), clock.now());
        let engine = AudioEngine {
            mode: Mode::Device(device),
            commands,
            counters,
            reaper,
            cache: Arc::new(ClipCache::new(clock.now())),
            active: RefCell::new(HashMap::new()),
            listener: Cell::new(Listener::default()),
            next_id: AtomicU64::new(1),
            enabled: false,
            clock,
        };
        let enabled = engine.open_default();
        let engine = AudioEngine { enabled, ..engine };
        if enabled {
            if let Some(d) = engine.device() {
                watch_default_device(d.name(), Arc::downgrade(&d.reopen_flag()));
            }
        }
        engine
    }

    /// An engine without an output device, for tests and the offline renderer: playback is
    /// driven and mixed with [`AudioEngine::render_offline`]. It runs on a manual clock, so
    /// the caller moves time and a run is reproducible; [`AudioEngine::enabled`] is true so
    /// sound sets run as usual.
    pub fn new_offline(sample_rate: u32, channels: usize) -> AudioEngine {
        let clock = Clock::manual(Instant::now());
        let counters = Arc::new(Counters::default());
        let commands = Arc::new(CommandQueue::new(counters.clone()));
        let reaper = Arc::new(Reaper::new());
        let format = Arc::new(OutputFormat::new(sample_rate, channels));
        let core = AudioCore::new(
            clock.clone(),
            format,
            commands.clone(),
            reaper.clone(),
            counters.clone(),
            mixer::muted(),
        );
        AudioEngine {
            mode: Mode::Offline(RefCell::new(core)),
            commands,
            counters,
            reaper,
            cache: Arc::new(ClipCache::new(clock.now())),
            active: RefCell::new(HashMap::new()),
            listener: Cell::new(Listener::default()),
            next_id: AtomicU64::new(1),
            enabled: true,
            clock,
        }
    }

    /// The clock this engine runs on (see [`AudioEngine::new_offline`]).
    pub fn clock(&self) -> Clock {
        self.clock.clone()
    }

    /// Mix `out.len() / channels` frames straight into `out` (interleaved, device order):
    /// the offline counterpart of the output callback, for tests and [`AudioEngine::new_offline`].
    pub fn render_offline(&self, out: &mut [f32]) {
        self.pump();
        if let Mode::Offline(core) = &self.mode {
            core.borrow_mut().render(out);
        }
    }

    fn device(&self) -> Option<&DeviceOutput> {
        match &self.mode {
            Mode::Device(d) => Some(d),
            Mode::Offline(_) => None,
        }
    }

    /// Build a fresh core and open the default output device on it. Used at start-up and when
    /// the device is followed; on success the still-active voices are replayed.
    fn open_default(&self) -> bool {
        let Some(device) = self.device() else {
            return false;
        };
        let mut core = AudioCore::new(
            self.clock.clone(),
            device.format(),
            self.commands.clone(),
            self.reaper.clone(),
            self.counters.clone(),
            mixer::muted(),
        );
        if !device.open(move |data| core.render(data)) {
            return false;
        }
        // The old core (if any) ended its voices on drop; forget them before replaying the
        // ones that were still active.
        self.pump();
        self.replay();
        true
    }

    /// Re-issue the still-active voices on a fresh core, keeping their ids: a device change
    /// does not turn a radio or an engine loop permanently silent. A looping clip restarts
    /// near its beginning - the only observable effect of a device change.
    fn replay(&self) {
        self.commands.push(Command::SetListener(self.listener.get()));
        let active = self.active.borrow();
        for (&id, a) in active.iter() {
            let cmd = match &a.asset {
                VoiceAsset::Clip(c) => Command::Play {
                    id,
                    clip: c.clone(),
                    params: a.params,
                },
                VoiceAsset::Stream(s) => Command::PlayStream {
                    id,
                    stream: s.clone(),
                    params: a.params,
                },
            };
            self.commands.push(cmd);
            self.counters.replays.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Follow the system's output: when its default device changed (headphones plugged in,
    /// a Bluetooth headset connected) or the one played on went away, the stream is opened
    /// again on the default device. Cheap; called every frame.
    pub fn follow_device(&self) {
        self.pump();
        let Some(device) = self.device() else {
            return;
        };
        if !self.enabled
            || device.opened_age() < 1.0
            || !device.reopen_flag().swap(false, Ordering::Relaxed)
        {
            return;
        }
        let lost = device.lost_flag().swap(false, Ordering::Relaxed);
        device.mark_opened(Instant::now());
        if lost {
            device.note_lost();
        } else {
            device.note_default_changed();
        }
        let before = device.name();
        if self.open_default() {
            let now = device.name();
            if now != before {
                log::info!("audio: output moved from {before} to {now}");
            }
        } else {
            // (no device right now: tried again when the watcher sees one)
            device.clear_name();
        }
    }

    /// The game thread lets go of voices the audio thread has ended. Called at the top of
    /// every public method, so no caller needs a new hook.
    fn pump(&self) {
        if !self.reaper.has_pending() {
            return;
        }
        let ids = self.reaper.drain();
        if ids.is_empty() {
            return;
        }
        let mut active = self.active.borrow_mut();
        for id in ids {
            active.remove(&id);
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
        self.pump();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.active.borrow_mut().insert(
            id,
            ActiveVoice {
                params,
                asset: VoiceAsset::Clip(clip.clone()),
            },
        );
        if let Some(dropped) = self.commands.push(Command::Play { id, clip, params }) {
            self.active.borrow_mut().remove(&dropped);
        }
        id
    }

    /// Play what `stream` is fed with (see [`crate::assets::stream::StreamBuf`]); the voice
    /// ends when the stream is closed.
    pub fn play_stream(&self, stream: Arc<StreamBuf>, params: VoiceParams) -> VoiceId {
        self.pump();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.active.borrow_mut().insert(
            id,
            ActiveVoice {
                params,
                asset: VoiceAsset::Stream(stream.clone()),
            },
        );
        if let Some(dropped) = self.commands.push(Command::PlayStream { id, stream, params }) {
            self.active.borrow_mut().remove(&dropped);
        }
        id
    }

    /// New parameters for a voice: queued for the mixer's next block (a few milliseconds),
    /// so the game never waits for a block being mixed. Given twice before that block, the
    /// later ones win.
    pub fn set_params(&self, id: VoiceId, params: VoiceParams) {
        self.pump();
        let at = self.clock.now();
        if let Some(a) = self.active.borrow_mut().get_mut(&id) {
            a.params = params;
        }
        self.commands.push(Command::SetParams { id, params, at });
    }

    pub fn stop(&self, id: VoiceId) {
        self.pump();
        self.active.borrow_mut().remove(&id);
        self.commands.push(Command::Stop { id });
    }

    pub fn set_cabin(&self, h: f32) {
        let at = self.clock.now();
        self.commands.push(Command::SetCabin {
            h: h.clamp(0.0, 1.0),
            at,
        });
    }

    pub fn set_listener(&self, l: Listener) {
        self.listener.set(l);
        self.commands.push(Command::SetListener(l));
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
        let device = e.device().expect("a device engine keeps its device");
        let before = device.name();
        device.reopen_flag().store(true, Ordering::Relaxed);
        device.mark_opened(Instant::now() - Duration::from_secs(2));
        e.follow_device();
        assert!(device.is_open());
        assert_eq!(device.name(), before);
        assert!(!device.reopen_flag().load(Ordering::Relaxed));
    }
}
