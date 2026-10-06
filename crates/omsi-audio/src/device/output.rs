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
        if self.lost.load(Ordering::Relaxed) { DeviceState::Lost }
        else { self.state.get() }
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
    pub fn close(&self) { self.stream.borrow_mut().take(); }

    pub fn open<F>(&self, render: F) -> bool
    where F: FnMut(&mut [f32]) + Send + 'static {
        self.open_prepared(move || render)
    }

    /// Prepare the renderer only after rate/layout selection, outside any callback.
    pub(crate) fn open_prepared<P, F>(&self, prepare: P) -> bool
    where P: FnOnce() -> F, F: FnMut(&mut [f32]) + Send + 'static {
        let previous = self.state();
        self.close();
        // The old callback is stopped. Clear its flags before creating the next stream;
        // an error from the new stream must remain visible even during play().
        self.lost.store(false, Ordering::Relaxed);
        self.reopen.store(false, Ordering::Relaxed);
        self.state.set(previous.opening());
        let host = cpal::default_host();
        let Some(dev) = host.default_output_device() else {
            self.state.set(previous.open_failed()); self.clear_name(); return false;
        };
        let name = dev.description().map(|d| d.name().to_string()).unwrap_or_default();
        let cfg = match dev.default_output_config() {
            Ok(c) if suitable(&c) => c,
            _ => {
                let candidate = dev.supported_output_configs().ok().and_then(|configs| {
                    configs.filter(|c| c.channels() <= 2 && supported(c.sample_format()))
                        .filter_map(|c| c.try_with_sample_rate(48000).or_else(|| c.try_with_sample_rate(44100)))
                        .max_by_key(|c| (c.channels(), c.sample_format() == cpal::SampleFormat::F32))
                });
                let Some(c) = candidate else {
                    log::warn!("audio: no suitable output format on {name}");
                    self.state.set(previous.open_failed()); self.clear_name(); return false;
                };
                c
            }
        };
        self.format.set(cfg.sample_rate(), cfg.channels() as usize);
        let render = prepare();
        let config = cfg.config();
        let reopen = self.reopen.clone(); let lost = self.lost.clone();
        use cpal::SampleFormat as S;
        let stream = match cfg.sample_format() {
            S::F32 => super::convert::build::<f32, _>(&dev, config, render, reopen, lost),
            S::F64 => super::convert::build::<f64, _>(&dev, config, render, reopen, lost),
            S::I8 => super::convert::build::<i8, _>(&dev, config, render, reopen, lost),
            S::I16 => super::convert::build::<i16, _>(&dev, config, render, reopen, lost),
            S::I32 => super::convert::build::<i32, _>(&dev, config, render, reopen, lost),
            S::I64 => super::convert::build::<i64, _>(&dev, config, render, reopen, lost),
            S::U8 => super::convert::build::<u8, _>(&dev, config, render, reopen, lost),
            S::U16 => super::convert::build::<u16, _>(&dev, config, render, reopen, lost),
            S::U32 => super::convert::build::<u32, _>(&dev, config, render, reopen, lost),
            S::U64 => super::convert::build::<u64, _>(&dev, config, render, reopen, lost),
            _ => unreachable!("suitable() checked the sample format"),
        };
        match stream {
            Ok(stream) => {
                if let Err(error) = stream.play() {
                    log::warn!("audio: cannot start stream on {name}: {error}");
                    self.state.set(previous.open_failed()); self.clear_name(); return false;
                }
                log::info!("audio: playing on {name} ({} Hz, {} channels, {:?})",
                    cfg.sample_rate(), cfg.channels(), cfg.sample_format());
                *self.stream.borrow_mut() = Some(stream); *self.name.borrow_mut() = name;
                self.state.set(DeviceState::Open); true
            }
            Err(error) => {
                log::warn!("audio: cannot open stream on {name}: {error}");
                self.state.set(previous.open_failed()); self.clear_name(); false
            }
        }
    }
}

fn supported(format: cpal::SampleFormat) -> bool {
    use cpal::SampleFormat as S;
    matches!(format, S::F32 | S::F64 | S::I8 | S::I16 | S::I32 | S::I64 | S::U8 | S::U16 | S::U32 | S::U64)
}
fn suitable(config: &cpal::SupportedStreamConfig) -> bool {
    (1..=8).contains(&config.channels()) && (8000..=192000).contains(&config.sample_rate())
        && supported(config.sample_format())
}
