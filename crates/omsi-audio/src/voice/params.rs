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

/// The separate legacy levels OMSI hands the renderer, kept apart from the raw gain the
/// application (`ambience`, radio, passenger voices) still sets. The reference's `TSound`
/// update (`00750444`) computes a recording level, a script level, an inside/outside
/// transmission and the global masters separately and only then clamps once to 0 dB - see
/// [`crate::runtime::level`]. Keeping the two paths distinct means the OMSI 0 dB clamp never
/// touches the application's raw gain (a radio knob may exceed 1.0).
#[derive(Debug, Clone, Copy)]
pub enum Level {
    /// A raw linear gain: multiplied by the listener master and distance, never clamped by
    /// the runtime's 0 dB rule (the limiter handles the output).
    Raw(f32),
    /// OMSI's buffer volume before the 0 dB clamp: recording level × script volume ×
    /// inside/outside transmission × the set master. `listener.master` is folded in by the
    /// voice, so the whole product is clamped together (see [`crate::voice`]).
    Omsi {
        record: f32,
        script: f32,
        transmission: f32,
        master: f32,
    },
}

impl Level {
    /// The linear level before the 0 dB clamp: the raw gain, or the product of the separate
    /// OMSI factors. Used by the voice so the global listener master can be folded in before
    /// the single clamp.
    pub fn product(self) -> f32 {
        match self {
            Level::Raw(g) => g,
            Level::Omsi {
                record,
                script,
                transmission,
                master,
            } => record * script * transmission * master,
        }
    }

    /// The linear level after the 0 dB clamp but before the listener master and distance.
    /// For [`Level::Raw`] the raw gain; for [`Level::Omsi`] the clamped product. Used for
    /// ranking and read-back.
    pub fn gain(self) -> f32 {
        match self {
            Level::Raw(g) => g,
            Level::Omsi { .. } => self.product().clamp(0.0, 1.0),
        }
    }
}

/// What the voice and mixer actually read: [`MixParams`] is the renderer's canonical
/// parameter set. It is not the application-facing type (that stays [`VoiceParams`], so the
/// OMSI runtime can hand separate levels without changing any caller); the runtime builds
/// `MixParams` directly and the engine converts `VoiceParams` into the raw path.
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct MixParams {
    /// Distance blend: 0 = local/centred cabin sound, 1 = fully spatial.
    pub spatial_blend: f32,
    pub bus: crate::voice::bus::Bus,
    /// The separate legacy level (see [`Level`]).
    pub level: Level,
    /// Playback speed relative to the clip's own rate (OMSI's playback frequency).
    pub pitch: f32,
    pub looping: bool,
    /// World position; `None` = non-spatial (distance is applied by the spatializer).
    pub position: Option<Vec3>,
    /// Whether motion relative to the listener changes playback pitch.
    pub doppler: bool,
    /// Distance of full volume for spatial voices.
    pub range: f32,
    /// A one-pole low-pass cutoff in Hz, or 0.0 for none (inside/outside timbre).
    pub lowpass_hz: f32,
    /// OMSI's `[important]`: keep this sound ahead of ordinary voices at the mixer limit.
    pub important: bool,
    /// Spatial direction: how much the spatializer pans the voice (1.0 = full).
    pub pan: f32,
}

impl Default for MixParams {
    fn default() -> Self {
        MixParams::from(VoiceParams::default())
    }
}

impl From<VoiceParams> for MixParams {
    fn from(p: VoiceParams) -> Self {
        MixParams {
            spatial_blend: 1.0,
            bus: crate::voice::bus::Bus::Vehicle,
            level: Level::Raw(p.gain),
            pitch: p.pitch,
            looping: p.looping,
            position: p.position,
            doppler: p.doppler,
            range: p.range,
            lowpass_hz: p.lowpass_hz,
            important: p.important,
            pan: p.pan,
        }
    }
}

impl From<MixParams> for VoiceParams {
    fn from(p: MixParams) -> Self {
        VoiceParams {
            gain: p.level.gain(),
            pitch: p.pitch,
            looping: p.looping,
            position: p.position,
            doppler: p.doppler,
            range: p.range,
            lowpass_hz: p.lowpass_hz,
            important: p.important,
            pan: p.pan,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_omsi_level_is_clamped_only_after_the_set_master() {
        // record 4.0 with a set master of 0.5: OMSI multiplies first and clamps the product
        // (2.0 -> 1.0), it does not cut the recording to 1.0 and then halve it to 0.5
        let level = Level::Omsi {
            record: 4.0,
            script: 1.0,
            transmission: 1.0,
            master: 0.5,
        };
        assert_eq!(level.product(), 2.0, "the product is not cut early");
        assert_eq!(level.gain(), 1.0, "and is clamped to 0 dB at the end");
    }

    #[test]
    fn a_raw_gain_keeps_its_value() {
        // the application may hand a gain over 1.0 (a radio knob); it must not be clamped
        assert_eq!(Level::Raw(2.0).gain(), 2.0);
    }

    #[test]
    fn voice_params_round_trip_through_the_mix_params() {
        let raw = VoiceParams {
            gain: 0.3,
            pitch: 1.5,
            looping: true,
            important: true,
            ..Default::default()
        };
        let mix = MixParams::from(raw);
        let back = VoiceParams::from(mix);
        assert_eq!(back.gain, 0.3);
        assert_eq!(back.pitch, 1.5);
        assert!(back.looping && back.important);
    }
}
