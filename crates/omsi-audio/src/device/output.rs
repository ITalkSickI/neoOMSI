//! Opening and re-opening the CPAL output stream. The render callback is handed in by the
//! engine, so this module names neither the mixer state nor the OMSI runtime.

use crate::device::state::OutputFormat;
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
    /// When the stream was last opened (at most one new stream a second).
    opened: Cell<Instant>,
    /// The rate and channels of the open stream, read by the mixer.
    format: Arc<OutputFormat>,
}

impl DeviceOutput {
    pub fn new(format: Arc<OutputFormat>, now: Instant) -> DeviceOutput {
        DeviceOutput {
            stream: RefCell::new(None),
            name: RefCell::new(String::new()),
            reopen: Arc::new(AtomicBool::new(false)),
            opened: Cell::new(now),
            format,
        }
    }

    /// The flag the watcher sets when the default output device changed.
    pub fn reopen_flag(&self) -> Arc<AtomicBool> {
        self.reopen.clone()
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
    /// stream is playing.
    pub fn open<F>(&self, render: F) -> bool
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        // (the old stream first: some drivers give a device to one stream at a time)
        self.stream.borrow_mut().take();
        let host = cpal::default_host();
        let Some(dev) = host.default_output_device() else {
            log::warn!("audio: no output device");
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
                return false;
            }
        };
        self.format
            .set(cfg.sample_rate(), cfg.channels().max(1) as usize);
        let lost = self.reopen.clone();
        let mut render = render;
        let stream = dev.build_output_stream(
            cfg.config(),
            move |data: &mut [f32], _| render(data),
            move |e| {
                // (a lost device only: a driver's hiccups are not worth a new stream)
                if e.kind() == cpal::ErrorKind::DeviceNotAvailable {
                    lost.store(true, Ordering::Relaxed);
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
                true
            }
            Err(e) => {
                log::warn!("audio: cannot open stream on {name}: {e}");
                false
            }
        }
    }
}
