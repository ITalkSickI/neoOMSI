//! Camera-based cabin membership, including acoustically open articulated sections.
//! Each hull stays in its own local frame; a narrow joint corridor fills the bellows.
use super::soundset::SoundSet;
use crate::Playback;
use glam::{Mat4, Vec3};
type Hull = ([f32; 3], [f32; 3]);

fn ease_distance(distance: f32) -> f32 {
    if !distance.is_finite() { return 0.0; }
    let t = ((0.25 - distance) / 0.5).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
#[path = "cabin_tests.rs"]
mod tests;

pub(super) fn hull_factor((center, half): Hull, transform: Mat4, listener: Vec3) -> f32 {
    let local = transform.inverse().transform_point3(listener) - Vec3::from_array(center);
    if !local.is_finite() { return 0.0; }
    ease_distance((local.x.abs() - half[0]).max(local.y.abs() - half[1]).max(local.z.abs() - half[2]))
}

fn joint_factor(front: Hull, front_xf: Mat4, rear: Hull, rear_xf: Mat4, listener: Vec3) -> f32 {
    let mut a = Vec3::from_array(front.0); a.y -= front.1[1];
    let mut b = Vec3::from_array(rear.0); b.y += rear.1[1];
    let a = front_xf.transform_point3(a);
    let b = rear_xf.transform_point3(b);
    let delta = (b - a).truncate();
    let length = delta.length();
    // Never bridge detached/remote bodies or build a large enclosing box around a turn.
    if !length.is_finite() || !(0.05..=3.0).contains(&length) { return 0.0; }
    let axis = delta / length;
    let relative = (listener - a).truncate();
    let along = relative.dot(axis);
    let side = relative.perp_dot(axis).abs();
    let z = a.z + (b.z - a.z) * (along / length).clamp(0.0, 1.0);
    let distance = (side - front.1[0].min(rear.1[0]))
        .max((listener.z - z).abs() - front.1[2].min(rear.1[2]))
        .max(-along).max(along - length);
    ease_distance(distance)
}

impl SoundSet {
    /// Whether this part shares an open passenger cabin with the leading section.
    pub fn set_cabin_connected(&mut self, connected: bool) { self.cabin_connected = connected; }

    /// Physical camera membership independent of driver/exterior/free view selection.
    /// None means no usable hull, so existing view-based fallbacks remain available.
    pub fn camera_cabin_factor(&self, listener: Vec3, object_to_world: &Mat4,
        part_to_world: &dyn Fn(usize) -> Option<Mat4>) -> Option<f32> {
        let mut factor = self.hull.map(|h| hull_factor(h, *object_to_world, listener));
        let mut previous = self.hull.map(|h| (h, *object_to_world));
        for (index, part) in &self.parts {
            if !part.cabin_connected { continue; }
            if let (Some(hull), Some(transform)) = (part.hull, part_to_world(*index)) {
                let mut h = hull_factor(hull, transform, listener);
                if let Some((front, front_xf)) = previous {
                    h = h.max(joint_factor(front, front_xf, hull, transform, listener));
                }
                factor = Some(factor.unwrap_or(0.0).max(h));
                previous = Some((hull, transform));
            }
        }
        factor
    }

    /// Prepare the shared factor BEFORE leading and trailer file/normal events run.
    /// Otherwise a rear engine treats a camera in the front cabin as outside its own box.
    pub fn prepare_listener_cabin(&mut self, engine: &dyn Playback, object_to_world: &Mat4,
        part_to_world: &dyn Fn(usize) -> Option<Mat4>) -> f32 {
        let geometry = self.camera_cabin_factor(engine.listener_position(), object_to_world, part_to_world);
        let shared = if self.inside { Some(1.0) } else { geometry };
        if let Some(h) = shared { engine.set_cabin(h); }
        self.hull_override = shared;
        for (_, part) in &mut self.parts {
            part.hull_override = if part.cabin_connected || part.hull.is_none() { shared } else { None };
        }
        shared.unwrap_or(if self.inside { 1.0 } else { 0.0 })
    }
}
