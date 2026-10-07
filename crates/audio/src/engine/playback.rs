//! The playback surface the OMSI runtime depends on. Keeping `SoundSet` on this trait means
//! the runtime names neither the concrete engine nor cpal; the engine remains the only
//! implementation for now.

use crate::assets::{clip::Clip, stream::StreamBuf};
use crate::clock::Clock;
use crate::engine::AudioEngine;
use crate::voice::{MixParams, VoiceId, VoiceParams};
use glam::Vec3;
use std::path::Path;
use std::sync::Arc;

pub trait Playback {
    fn play(&self, clip: Arc<Clip>, params: VoiceParams) -> VoiceId;
    /// Play with the runtime's separate legacy levels (see [`MixParams`]). The default
    /// forwards through the raw path, so a non-OMSI implementation stays compatible; the
    /// concrete engine overrides it so the split survives.
    fn play_mix(&self, clip: Arc<Clip>, params: MixParams) -> VoiceId {
        self.play(clip, params.into())
    }
    fn play_stream(&self, stream: Arc<StreamBuf>, params: VoiceParams) -> VoiceId;
    fn stop(&self, id: VoiceId);
    fn set_params(&self, id: VoiceId, params: VoiceParams);
    /// Set the runtime's separate levels (see [`MixParams`]).
    fn set_mix_params(&self, id: VoiceId, params: MixParams) {
        self.set_params(id, params.into());
    }
    fn is_playing(&self, id: VoiceId) -> bool;
    fn voice_state(&self, id: VoiceId) -> Option<(VoiceParams, f32)>;
    fn listener_position(&self) -> Vec3;
    fn load_clip(&self, path: &Path) -> Option<Arc<Clip>>;
    /// The clock the runtime runs on (so an offline engine stays reproducible).
    fn clock(&self) -> Clock;
    /// Whether logical playback is enabled; hardware outages retain loops for reconnection.
    fn enabled(&self) -> bool;
}

impl Playback for AudioEngine {
    fn play(&self, clip: Arc<Clip>, params: VoiceParams) -> VoiceId {
        AudioEngine::play(self, clip, params)
    }

    fn play_mix(&self, clip: Arc<Clip>, params: MixParams) -> VoiceId {
        AudioEngine::play_mix(self, clip, params)
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

    fn set_mix_params(&self, id: VoiceId, params: MixParams) {
        AudioEngine::set_mix_params(self, id, params)
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
