//! Voice parameters and the listener: the plain values the runtime hands to a voice and the
//! mixer reads back, plus `DOPPLER`, the one outward-facing switch the game writes.

use glam::Vec3;

pub type VoiceId = u64;

#[derive(Debug, Clone, Copy)]
pub struct VoiceParams {
    pub gain: f32,
    /// Playback speed relative to the clip's own rate.
    pub pitch: f32,
    pub looping: bool,
    /// World position; `None` = non-spatial.
    pub position: Option<Vec3>,
    /// Whether motion relative to the listener changes playback pitch.
    pub doppler: bool,
    /// Distance of full volume for spatial voices.
    pub range: f32,
    /// A one-pole low-pass cutoff in Hz, or 0.0 for none: a sound heard through the
    /// bodywork from the cabin loses its edge, not just some volume (a straight gain cut
    /// still reads as "the same sound, turned down" rather than "coming from outside").
    pub lowpass_hz: f32,
    /// OMSI's `[important]`: keep this sound ahead of ordinary voices when the
    /// mixer limit is reached.
    pub important: bool,
    pub pan: f32,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            gain: 1.0,
            pitch: 1.0,
            looping: false,
            position: None,
            doppler: true,
            range: 5.0,
            lowpass_hz: 0.0,
            important: false,
            pan: 1.0,
        }
    }
}

/// OMSI's `sound_doppler`: a sound coming closer is higher, one going away lower. The one
/// global the façade still exposes; its value is read once and passed to the voice as an
/// explicit `doppler_enabled` parameter (see [`crate::voice::Voice::apply_params`]).
pub static DOPPLER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

#[derive(Debug, Clone, Copy)]
pub struct Listener {
    pub position: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub master: f32,
    /// Echo of the place the listener is in (`[triggerbox_setreverb]`: under a bridge):
    /// its reverberation time in seconds, and how much of it is heard (0..1).
    pub reverb_time: f32,
    pub reverb_mix: f32,
}

impl Default for Listener {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            forward: Vec3::Y,
            right: Vec3::X,
            master: 1.0,
            reverb_time: 0.0,
            reverb_mix: 0.0,
        }
    }
}
