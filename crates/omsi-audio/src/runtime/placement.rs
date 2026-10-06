//! Pure placement rules: which view an entry may be heard from, and where a sound sits in
//! the world with its reach and how much it pans.

use glam::{Mat4, Vec3};

/// The `[viewpoint]` bits an entry must carry to be heard now: 1 from outside, 2 from
/// inside the vehicle, 4 on an AI vehicle (the same bits as a mesh's `[viewpoint]`).
/// Without them the EN92's exterior engine samples (`[viewpoint] 5`) played on top of the
/// cab's own engine loop, and the rain on the roof (`[viewpoint] 2`) was heard while
/// standing in the street.
pub(crate) fn view_mask(inside: bool, ai: bool) -> i32 {
    (if inside { 2 } else { 1 }) | (if ai { 4 } else { 0 })
}

/// Where an entry sits in the world, with its reach and how much it pans. Every sound is
/// placed: a non-`[3d]` entry sits at the vehicle's origin. The listener in the cab
/// (`inside` 1) hears it as before - centred and at full level (a huge range, no pan);
/// stepping out, it moves out of the cab with them: range and pan fade in, and the sound
/// comes from the bus and fades with distance.
pub(crate) fn place(
    pos: Option<[f32; 3]>,
    range: f32,
    exterior: bool,
    object_to_world: &Mat4,
) -> (Option<Vec3>, f32, f32) {
    match pos {
        Some(p) => (
            Some(object_to_world.transform_point3(Vec3::from_array(p))),
            if range > 0.0 {
                range
            } else if exterior {
                40.0
            } else {
                5.0
            },
            1.0,
        ),
        None => (
            Some(object_to_world.transform_point3(Vec3::ZERO)),
            if range > 0.0 { range.max(40.0) } else { 40.0 },
            1.0,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_mask_needs_the_right_side() {
        assert_eq!(view_mask(false, false), 1);
        assert_eq!(view_mask(true, false), 2);
        assert_eq!(view_mask(false, true), 1 | 4);
        assert_eq!(view_mask(true, true), 2 | 4);
    }

    #[test]
    fn a_non_3d_sound_sits_at_the_vehicle_and_reaches_far() {
        let xf = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        let (pos, range, pan) = place(None, 0.0, false, &xf);
        assert_eq!(pos, Some(Vec3::new(5.0, 0.0, 0.0)));
        assert_eq!(range, 40.0);
        assert_eq!(pan, 1.0);
    }

    #[test]
    fn a_3d_sound_keeps_its_range_or_takes_a_default() {
        let xf = Mat4::IDENTITY;
        assert_eq!(place(Some([1.0, 2.0, 3.0]), 12.0, false, &xf).1, 12.0);
        assert_eq!(place(Some([0.0; 3]), 0.0, false, &xf).1, 5.0);
        assert_eq!(place(Some([0.0; 3]), 0.0, true, &xf).1, 40.0);
    }
}
