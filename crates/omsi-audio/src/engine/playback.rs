//! The playback surface the OMSI runtime depends on. Keeping `SoundSet` on this trait means
//! the runtime names neither the concrete engine nor cpal; the engine remains the only
//! implementation for now.

use crate::assets::{clip::Clip, stream::StreamBuf};
use crate::clock::Clock;
use crate::engine::AudioEngine;
use crate::voice::{VoiceId, VoiceParams};
use glam::Vec3;
use std::path::Path;
use std::sync::Arc;

pub trait Playback {
    fn play(&self, clip: Arc<Clip>, params: VoiceParams) -> VoiceId;
    fn play_stream(&self, stream: Arc<StreamBuf>, params: VoiceParams) -> VoiceId;
    fn stop(&self, id: VoiceId);
    fn set_params(&self, id: VoiceId, params: VoiceParams);
    fn is_playing(&self, id: VoiceId) -> bool;
    fn voice_state(&self, id: VoiceId) -> Option<(VoiceParams, f32)>;
    fn listener_position(&self) -> Vec3;
    fn set_cabin(&self, h: f32);
    fn load_clip(&self, path: &Path) -> Option<Arc<Clip>>;
    /// The clock the runtime runs on (so an offline engine stays reproducible).
    fn clock(&self) -> Clock;
    /// Whether the engine plays at all (no device, or `OMSI_MUTE`).
    fn enabled(&self) -> bool;
}

impl Playback for AudioEngine {
    fn play(&self, clip: Arc<Clip>, params: VoiceParams) -> VoiceId {
        AudioEngine::play(self, clip, params)
    }

    fn play_stream(&self, stream: Arc<StreamBuf>, params: VoiceParams) -> VoiceId {
        AudioEngine::play_stream(self, stream, params)
    }

    fn stop(&self, id: VoiceId) {
        AudioEngine::stop(self, id)
    }

    fn set_params(&self, id: VoiceId, params: VoiceParams) {
        AudioEngine::set_params(self, id, params)
    }

    fn is_playing(&self, id: VoiceId) -> bool {
        AudioEngine::is_playing(self, id)
    }

    fn voice_state(&self, id: VoiceId) -> Option<(VoiceParams, f32)> {
        AudioEngine::voice_state(self, id)
    }

    fn listener_position(&self) -> Vec3 {
        AudioEngine::listener_position(self)
    }

    fn set_cabin(&self, h: f32) {
        AudioEngine::set_cabin(self, h)
    }

    fn load_clip(&self, path: &Path) -> Option<Arc<Clip>> {
        AudioEngine::load_clip(self, path)
    }

    fn clock(&self) -> Clock {
        AudioEngine::clock(self)
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}
