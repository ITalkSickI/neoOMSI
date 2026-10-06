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
        let (left, right) = pan_gains(side, pan);
        Placed { left, right, gain }
    }
}

/// The stereo pair of a spatial voice, reproducing DirectSound's `SetPan`: one channel is
/// damped relative to the other by up to 100 dB, the other stays at full level. DirectSound
/// takes hundredths of a decibel from -10000 (full left) to 10000 (full right), 0 centred
/// (both channels at 0 dB). The reference's `TSound` update computes the direction and hands
/// it to `SetPan` as an integer (`00750444`, vtable +0x40); the old `sqrt * 1.2` formula
/// damped *both* channels and behaved differently. `side` is the direction along the
/// listener's right (negative left, positive right), `amount` scales how far it pans.
/// `hundredths_db` is clamped so the near channel never falls below 1.0 and the far one
/// never goes below -100 dB.
fn pan_gains(side: f32, amount: f32) -> (f32, f32) {
    let pan = (side.clamp(-1.0, 1.0) * amount.clamp(0.0, 1.0) * 10_000.0).round();
    let far = 10f32.powf(-pan.abs() / 2_000.0);
    if pan >= 0.0 {
        (far, 1.0)
    } else {
        (1.0, far)
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

    #[test]
    fn pan_damps_one_channel_like_directsound() {
        // centred: both channels full
        assert_eq!(pan_gains(0.0, 1.0), (1.0, 1.0));
        // full left: only the right channel is damped, by 100 dB
        let (left, right) = pan_gains(-1.0, 1.0);
        assert_eq!(left, 1.0);
        assert!((right - 1e-5).abs() < 1e-6, "right {right}");
        // full right: only the left channel is damped
        let (left, right) = pan_gains(1.0, 1.0);
        assert!((left - 1e-5).abs() < 1e-6, "left {left}");
        assert_eq!(right, 1.0);
        // a quarter to the right: the left channel is damped by 25 dB, the right untouched
        let (left, right) = pan_gains(0.25, 1.0);
        assert!((left - 0.0562).abs() < 0.005, "left {left}");
        assert_eq!(right, 1.0);
        // `amount` 0 stays centred
        assert_eq!(pan_gains(1.0, 0.0), (1.0, 1.0));
    }
}
