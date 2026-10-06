//! Where a voice sits relative to the listener: distance attenuation and stereo panning.
//!
//! This keeps legacy distance and listening-tuned stereo behind [`Spatializer`] so that Phase 2
//! (Steam Audio) can add another implementation without the voice or runtime code changing.
//! No Steam Audio here.

use glam::Vec3;

/// The stereo levels and distance gain of a placed voice.
pub struct Placed {
    pub left: f32,
    pub right: f32,
    pub gain: f32,
}

/// How a voice is placed for the listener.
pub trait Spatializer {
    fn place(
        &self,
        position: Option<Vec3>,
        range: f32,
        pan: f32,
        listener_pos: Vec3,
        listener_right: Vec3,
    ) -> Placed;
}

/// The legacy spatialiser; a non-spatial voice is centred and full level.
pub struct Legacy;

impl Spatializer for Legacy {
    fn place(
        &self,
        position: Option<Vec3>,
        range: f32,
        pan: f32,
        listener_pos: Vec3,
        listener_right: Vec3,
    ) -> Placed {
        let Some(p) = position else {
            return Placed {
                left: 1.0,
                right: 1.0,
                gain: 1.0,
            };
        };
        let d = p - listener_pos;
        let dist = d.length().max(0.1);
        let gain = distance_gain(range, dist);
        let side = d.normalize_or_zero().dot(listener_right.normalize_or_zero());
        let (left, right) = pan_gains(side, pan);
        Placed { left, right, gain }
    }
}

/// Keep the near ear at unity and limit broadband head shadow to 12 dB. Mapping a
/// geometric side directly to DirectSound's entire +/-10000 range produced up to
/// 100 dB of attenuation: even a small head turn effectively silenced one ear.
/// This bounded stereo tuning follows listening feedback, not a confirmed OMSI pan scale.
fn pan_gains(side: f32, amount: f32) -> (f32, f32) {
    let pan = side.clamp(-1.0, 1.0) * amount.clamp(0.0, 1.0);
    let far = 10f32.powf(-12.0 * pan.abs() / 20.0);
    if pan >= 0.0 { (far, 1.0) } else { (1.0, far) }
}

/// How loud a sound `dist` metres away arrives, with `range` its `[3d]` reference distance:
/// OMSI hands it to DirectSound 3D as the minimum distance with the default roll-off, so it
/// is full up to that distance and then falls as 1/d (6 dB per doubling). Ours fell as
/// (range/d)^1.6: at ten times the distance a sound was 4 % instead of 10 % - nearly
/// everything was too quiet, a blinker relay half a metre from the head included.
pub fn distance_gain(range: f32, dist: f32) -> f32 {
    (range.max(0.01) / dist.max(0.01)).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_distance_beyond_the_reference() {
        assert_eq!(distance_gain(2.0, 1.0), 1.0);
        assert!((distance_gain(2.0, 4.0) - 0.5).abs() < 1e-6);
        assert!((distance_gain(1.0, 10.0) - 0.1).abs() < 1e-6);
    }

    #[test]
    fn panning_keeps_both_ears_audible_and_preserves_direction() {
        assert_eq!(pan_gains(0.0, 1.0), (1.0, 1.0));
        for side in [-1.0, -0.5, -0.25, 0.0, 0.25, 0.5, 1.0] {
            let (left, right) = pan_gains(side, 1.0);
            assert!(left >= 0.25 && right >= 0.25);
            assert_eq!(pan_gains(-side, 1.0), (right, left));
            if side > 0.0 { assert!(right > left); }
        }
        assert!((pan_gains(0.25, 1.0).0 - 0.708).abs() < 0.001);
        assert_eq!(pan_gains(1.0, 0.0), (1.0, 1.0));
    }
}
