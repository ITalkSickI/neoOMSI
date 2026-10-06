//! Linear resampling of a clip: where in the clip the next output frame is read, and the
//! interpolated stereo frame there.

use crate::assets::Clip;

/// How far the read position moves per output frame: pitch and Doppler against the clip's
/// own rate and the device's. Never zero (a standstill would divide nothing but stall).
pub fn step(pitch: f32, doppler: f32, clip_rate: u32, dev_rate: f64) -> f64 {
    (pitch * doppler).max(0.01) as f64 * clip_rate as f64 / dev_rate
}

/// The interpolated left/right sample between clip frames `i0` and `i1` at `t` (0..1).
/// Mono clips come back on both sides.
pub fn frame(clip: &Clip, i0: usize, i1: usize, t: f32, cch: usize) -> (f32, f32) {
    let sample = |c: usize| {
        let a = clip.samples[i0 * cch + c.min(cch - 1)] as f32 / 32768.0;
        let b = clip.samples[i1 * cch + c.min(cch - 1)] as f32 / 32768.0;
        a + (b - a) * t
    };
    if cch >= 2 {
        (sample(0), sample(1))
    } else {
        let m = sample(0);
        (m, m)
    }
}
