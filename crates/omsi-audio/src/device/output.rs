//! Opening and re-opening the CPAL output stream. The render callback is handed in by the
//! engine, so this module names neither the mixer state nor the OMSI runtime. The device's
//! state is explicit (see [`DeviceState`]): a lost stream or a changed default is a defined
//! transition, and the engine's `follow_device` drives the reopen.

use crate::device::state::{DeviceState, OutputFormat};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// The stream on the device played on now, what it is called, and when it was last opened.
pub struct DeviceOutput {
    stream: RefCell<Option<cpal::Stream>>,
    name: RefCell<String>,
    /// Set when the stream fails (its device went away) or the system's default output
    /// changed (a watcher thread looks every two seconds).
    reopen: Arc<AtomicBool>,
    /// Set when a stream error was a lost device (not just a hiccup), so the engine can tell
    /// a lost device from a changed default.
    lost: Arc<AtomicBool>,
    /// When the stream was last opened (at most one new stream a second).
    opened: Cell<Instant>,
    /// The rate and channels of the open stream, read by the mixer.
    format: Arc<OutputFormat>,
    state: Cell<DeviceState>,
}

impl DeviceOutput {
    pub fn new(format: Arc<OutputFormat>, now: Instant) -> DeviceOutput {
        DeviceOutput {
            stream: RefCell::new(None),
            name: RefCell::new(String::new()),
            reopen: Arc::new(AtomicBool::new(false)),
            lost: Arc::new(AtomicBool::new(false)),
            opened: Cell::new(now),
            format,
            state: Cell::new(DeviceState::NoDevice),
        }
    }

    /// The flag the watcher sets when the default output device changed.
    pub fn reopen_flag(&self) -> Arc<AtomicBool> {
        self.reopen.clone()
    }

    /// The flag the stream error callback sets when the device went away.
    pub fn lost_flag(&self) -> Arc<AtomicBool> {
        self.lost.clone()
    }

    /// The rate/channels bridge the mixer reads.
    pub fn format(&self) -> Arc<OutputFormat> {
        self.format.clone()
    }

    pub fn state(&self) -> DeviceState {
        self.state.get()
    }

    /// Record that the device went away (before a reopen attempt).
    pub fn note_lost(&self) {
        self.state.set(self.state.get().lost());
    }

    /// Record that the default output changed (before a reopen attempt).
    pub fn note_default_changed(&self) {
        self.state.set(self.state.get().retrying());
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    pub fn is_open(&self) -> bool {
        self.stream.borrow().is_some()
    }

    pub fn opened_age(&self) -> f32 {
        self.opened.get().elapsed().as_secs_f32()
    }

    pub fn mark_opened(&self, now: Instant) {
        self.opened.set(now);
    }

    pub fn clear_name(&self) {
        self.name.borrow_mut().clear();
    }

    /// Open the default output device from now on, feeding it `render`. Returns whether a
    /// stream is playing; the state machine follows along (`Opening` -> `Open`/`NoDevice`/`Lost`).
    pub fn open<F>(&self, render: F) -> bool
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        // (the old stream first: some drivers give a device to one stream at a time)
        self.stream.borrow_mut().take();
        self.state.set(self.state.get().opening());
        let host = cpal::default_host();
        let Some(dev) = host.default_output_device() else {
            log::warn!("audio: no output device");
            self.state.set(self.state.get().open_failed());
            return false;
        };
        let name = dev
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_default();
        let cfg = match dev.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("audio: no output config on {name}: {e}");
                self.state.set(self.state.get().open_failed());
                return false;
            }
        };
        self.format
            .set(cfg.sample_rate(), cfg.channels().max(1) as usize);
        let reopen = self.reopen.clone();
        let lost = self.lost.clone();
        let mut render = render;
        let stream = dev.build_output_stream(
            cfg.config(),
            move |data: &mut [f32], _| render(data),
            move |e| {
                // (a lost device only: a driver's hiccups are not worth a new stream)
                if e.kind() == cpal::ErrorKind::DeviceNotAvailable {
                    lost.store(true, Ordering::Relaxed);
                    reopen.store(true, Ordering::Relaxed);
                }
                log::warn!("audio stream error: {e}");
            },
            None,
        );
        match stream {
            Ok(s) => {
                if let Err(e) = s.play() {
                    log::warn!("audio: cannot start stream: {e}");
                }
                log::info!(
                    "audio: playing on {name} ({} Hz, {} channels)",
                    cfg.sample_rate(),
                    cfg.channels()
                );
                *self.stream.borrow_mut() = Some(s);
                *self.name.borrow_mut() = name;
                self.state.set(self.state.get().opened());
                true
            }
            Err(e) => {
                log::warn!("audio: cannot open stream on {name}: {e}");
                self.state.set(self.state.get().open_failed());
                false
            }
        }
    }
}
