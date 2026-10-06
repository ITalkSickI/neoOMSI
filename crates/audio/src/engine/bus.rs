//! Independent mixer buses. Defaults keep dialogue clear, with ambience/radio lower.
//! The master reserves 6 dB before the stereo-linked limiter (ceiling 0.9, release 500 ms).
//! Bus controls affect rendering only; OMSI levels/admission remain unchanged.
use crate::voice::VoiceParams;
use crate::assets::{Clip, stream::StreamBuf};
use crate::voice::VoiceId;
use std::sync::Arc;

pub use crate::voice::bus::{Bus, BUS_COUNT, DEFAULT_GAINS, HEADROOM};

impl super::AudioEngine {
    pub fn play_on_bus(&self, clip: Arc<Clip>, params: VoiceParams, bus: Bus) -> VoiceId {
        let mut mix = crate::voice::MixParams::from(params);
        mix.bus = bus;
        self.play_mix(clip, mix)
    }
    pub fn play_stream_on_bus(&self, stream: Arc<StreamBuf>, params: VoiceParams, bus: Bus) -> VoiceId {
        let mut mix = crate::voice::MixParams::from(params);
        mix.bus = bus;
        self.play_stream_mix(stream, mix)
    }
    /// Linear bus gain (0..2), smoothed over 20 ms; retained across device changes.
    pub fn set_bus_gain(&self, bus: Bus, gain: f32) {
        let gain = if gain.is_finite() { gain.clamp(0.0, 2.0) } else { 0.0 };
        let mut gains = self.bus_gains.get(); gains[bus as usize] = gain; self.bus_gains.set(gains);
        if self.output_available() { self.enqueue(super::commands::Command::SetBus { bus, gain }); }
    }
}
