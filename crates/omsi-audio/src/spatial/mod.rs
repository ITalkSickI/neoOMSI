//! Where a voice sits relative to the listener: distance attenuation and stereo panning.
//!
//! This is the legacy OMSI / DirectSound model, kept behind [`Spatializer`] so that Phase 2
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
        let side = d.normalize_or_zero().dot(listener_right);
        let pan_side = side.clamp(-1.0, 1.0);
        let amount = pan.clamp(0.0, 1.0);
        Placed {
            left: 1.0 + (((1.0 - pan_side) * 0.5).sqrt() * 1.2 - 1.0) * amount,
            right: 1.0 + (((1.0 + pan_side) * 0.5).sqrt() * 1.2 - 1.0) * amount,
            gain,
        }
    }
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
}
