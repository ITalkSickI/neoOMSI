//! Read-back from the mixer: what a voice plays with now and how loud it arrives, whether it
//! is still playing, and the listener's place. The game uses these instead of reaching into
//! the voice list.

use crate::spatial::distance_gain;
use crate::voice::{VoiceId, VoiceParams};
use glam::Vec3;

impl super::AudioEngine {
    /// What a voice plays with now, and how loud it arrives at the listener (gain after
    /// distance), while it plays.
    pub fn voice_state(&self, id: VoiceId) -> Option<(VoiceParams, f32)> {
        let listener = *self.shared.listener.lock();
        let voices = self.shared.voices.lock();
        let v = voices.iter().find(|v| v.id() == id && !v.is_finished())?;
        let spatial = v
            .params()
            .position
            .map(|p| distance_gain(v.params().range, (p - listener.position).length()))
            .unwrap_or(1.0);
        Some((v.params(), v.params().gain * spatial))
    }

    pub fn is_playing(&self, id: VoiceId) -> bool {
        self.shared
            .voices
            .lock()
            .iter()
            .any(|v| v.id() == id && !v.is_finished())
    }

    pub fn listener_position(&self) -> Vec3 {
        self.shared.listener.lock().position
    }

    pub fn voice_count(&self) -> usize {
        self.shared.voices.lock().len()
    }
}
