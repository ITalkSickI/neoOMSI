//! Puddle detection for wheels crossing standing water. The puddle itself - the
//! glass-flat, reflective patches on a wet `[moisture]` road, with the raindrop ripples
//! crossing it - is `enhanced.wgsl`'s; this file only works out *where* one sits (the same
//! low-frequency mask, evaluated here so a wheel can be asked whether it stands in one).

use glam::{DVec3, Vec3};

const PUDDLE_SPREAD: f32 = 0.45;

/// The puddle mask `enhanced.wgsl` paints on a `[moisture]` road, evaluated on the CPU with
/// the same two octaves of value noise so a splash starts exactly where the reflection does.
/// `wet_road` is the moisture-weighted wetness at this point (global wetness where the
/// surface is a `[moisture]` road, 0 elsewhere - a puddle never sits on bare terrain).
pub fn puddle_coverage(x: f64, y: f64, wet_road: f32) -> f32 {
    if wet_road <= 0.0 {
        return 0.0;
    }
    let (wx, wy) = (x as f32, y as f32);
    let pn = vnoise(wx * 0.22 + 17.3, wy * 0.22 - 9.1) * 0.65
        + vnoise(wx * 0.9 - 4.0, wy * 0.9 + 8.0) * 0.35;
    let t = 1.0 - wet_road * PUDDLE_SPREAD;
    smoothstep(t - 0.06, t + 0.06, pn)
}

fn hash2(x: f32, y: f32) -> f32 {
    let s = (x * 127.1 + y * 311.7).sin() * 43758.5453;
    s - s.floor()
}

fn vnoise(x: f32, y: f32) -> f32 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (x - ix, y - iy);
    let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = hash2(ix, iy);
    let b = hash2(ix + 1.0, iy);
    let c = hash2(ix, iy + 1.0);
    let d = hash2(ix + 1.0, iy + 1.0);
    let ab = a + (b - a) * ux;
    let cd = c + (d - c) * ux;
    ab + (cd - ab) * uy
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The player's wheel contact points this frame (world) and the vehicle's forward speed
/// (m/s, unsigned) - [`Splashes::update`] does not need each wheel's own speed, only
/// whether the bus is moving through what its own wheels stand on.
pub fn wheel_contacts(v: &::simulation::VehicleInstance) -> Vec<DVec3> {
    let Some(rb) = v.rigid.as_ref() else {
        return Vec::new();
    };
    let xf = v.world_transform();
    rb.wheels
        .iter()
        .filter(|w| w.on_ground)
        .map(|w| {
            xf.transform_point3(Vec3::new(w.attach.x, w.attach.y, w.attach.z - w.radius))
                .as_dvec3()
        })
        .collect()
}

/// Counts the wheels standing in water (for `OMSI_DEBUG_RAIN`). The water itself is
/// displaced in the puddle shader (`puddle_wake`), so nothing is spawned here.
#[derive(Default)]
pub struct Splashes {
    /// Wheels standing in water right now.
    pub wheels_in_puddle: u32,
}

impl Splashes {
    pub fn new() -> Splashes {
        Splashes::default()
    }

    /// One frame: `wheels` are this frame's ground contact points, `speed` the vehicle's own
    /// (m/s), `coverage_at` how much water lies at a world (x, y) (0..1).
    pub fn update(&mut self, wheels: &[DVec3], speed: f32, coverage_at: &dyn Fn(f64, f64) -> f32) {
        self.wheels_in_puddle = if speed < 0.4 {
            0
        } else {
            wheels
                .iter()
                .filter(|w| coverage_at(w.x, w.y) > 0.02)
                .count() as u32
        };
        if ::legacy_config::env::var_os("OMSI_DEBUG_RAIN").is_some() && self.wheels_in_puddle > 0 {
            log::info!(
                "puddles: {} of {} wheels in water",
                self.wheels_in_puddle,
                wheels.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_is_zero_off_the_moisture_mask_and_grows_with_wetness() {
        assert_eq!(
            puddle_coverage(1000.0, 2000.0, 0.0),
            0.0,
            "no [moisture] under the wheel: never a puddle"
        );
        // scan a stretch of road for a spot the mask calls a puddle at full wetness, and
        // check it grows monotonically as the road dries back out from there
        let mut best = None;
        for i in 0..400 {
            let x = i as f64 * 1.3;
            let c = puddle_coverage(x, 500.0, 1.0);
            if c > 0.4 {
                best = Some((x, c));
                break;
            }
        }
        let (x, full) =
            best.expect("some spot along 520 m of road should read as a puddle at full wetness");
        let half = puddle_coverage(x, 500.0, 0.5);
        let dry = puddle_coverage(x, 500.0, 0.05);
        assert!(
            full >= half && half >= dry && full > dry,
            "a puddle should shrink as the road dries: {full} (wet) vs {half} (half) vs {dry} (drier)"
        );
    }

    #[test]
    fn a_road_wet_through_has_puddles_in_patches() {
        let mut full_puddles = 0;
        let mut half_puddles = 0;
        for i in 0..500 {
            let (x, y) = (i as f64 * 0.77, i as f64 * 1.9);
            full_puddles += (puddle_coverage(x, y, 1.0) > 0.5) as usize;
            half_puddles += (puddle_coverage(x, y, 0.5) > 0.5) as usize;
        }
        let full_coverage = full_puddles as f32 / 500.0;
        let half_coverage = half_puddles as f32 / 500.0;
        assert!(
            (0.15..=0.50).contains(&full_coverage),
            "a soaked road should have puddles in patches, got {:.0}% coverage",
            full_coverage * 100.0
        );
        assert!(
            half_coverage < full_coverage * 0.5,
            "puddles should cover less than half as much at half wetness: {:.0}% vs {:.0}%",
            half_coverage * 100.0,
            full_coverage * 100.0
        );
    }
}