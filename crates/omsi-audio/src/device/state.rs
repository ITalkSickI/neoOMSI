//! The output device's sample rate and channel count, shared between the device (which sets
//! them when it opens a stream) and the mixer (which reads them while rendering).

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub struct OutputFormat {
    sample_rate: AtomicU32,
    channels: AtomicUsize,
}

impl OutputFormat {
    pub fn new(sample_rate: u32, channels: usize) -> OutputFormat {
        OutputFormat {
            sample_rate: AtomicU32::new(sample_rate.max(1)),
            channels: AtomicUsize::new(channels.max(1)),
        }
    }

    pub fn set(&self, sample_rate: u32, channels: usize) {
        self.sample_rate.store(sample_rate.max(1), Ordering::Relaxed);
        self.channels.store(channels.max(1), Ordering::Relaxed);
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate.load(Ordering::Relaxed).max(1)
    }

    pub fn channels(&self) -> usize {
        self.channels.load(Ordering::Relaxed).max(1)
    }
}
