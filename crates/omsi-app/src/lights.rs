use crate::scene::{LightSwitch, World};
use glam::{DVec3, Vec3};
use omsi_render::{Corona, LightMode, Lighting, PointLight, Scene};

use omsi_sim::{Daylight, VehicleInstance};

const HEADLIGHT_INTENSITY: f32 = 45.0;

const VANILLA_HEADLIGHT_INTENSITY: f32 = 0.2;

pub fn lighting_from(d: &Daylight, fog_range: f32) -> Lighting {
    let density = (2.3 / fog_range.max(50.0)).max(0.00005);
    Lighting {
        sun_dir: d.sun_dir,
        sun_intensity: 1.0,
        sun_color: d.sun_color,
        secondary: d.secondary,
        ambient: d.ambient,
        fog_color: d.sky,
        fog_density: density,
        sky_color: d.sky,
        night: d.night,
        night_maps: Some(if d.lamps_on || d.night >= 0.5 { 1.0 } else { 0.0 }),
        sun_azimuth: d.azimuth_rad,
        sky_weights: d.sky_weights,
        envir_tint: d.envir_tint,
        ..Default::default()
    }
}

pub fn apply_weather(
    l: &mut Lighting,
    cloud_density: f32,
    precip_kind: i32,
    precip: f32,
    snow: f32,
) {
    let o = cloud_density.clamp(0.0, 1.0);
    let overcast = (o - 0.45).max(0.0) / 0.55;
    let grey = |c: Vec3, k: f32| -> Vec3 {
        let lum = c.dot(Vec3::new(0.3, 0.59, 0.11));
        c.lerp(Vec3::splat(lum), k)
    };
    l.sun_intensity *= 1.0 - 0.85 * overcast;
    l.sun_color = grey(l.sun_color, 0.6 * o);
    let sky_lum = l.sky_color.dot(Vec3::new(0.3, 0.59, 0.11));
    let cloud_sky = Vec3::splat(sky_lum * 0.82)
        .lerp(Vec3::new(0.62, 0.65, 0.70) * sky_lum.max(0.25) * 1.3, 0.5);
    l.sky_color = l.sky_color.lerp(cloud_sky, overcast * 0.9);
    l.secondary = grey(l.secondary, o * 0.7) * (1.0 - 0.15 * overcast);
    l.ambient = grey(l.ambient, o * 0.7) * (1.0 + 0.25 * overcast);
    l.fog_color = l.fog_color.lerp(l.sky_color, overcast);
    let rain = if precip_kind != 0 {
        precip.clamp(0.0, 1.0)
    } else {
        0.0
    };
    l.sun_intensity *= 1.0 - 0.75 * rain;
    l.ambient *= 1.0 - 0.28 * rain;
    l.secondary *= 1.0 - 0.32 * rain;
    l.sky_color *= 1.0 - 0.22 * rain;
    l.fog_color *= 1.0 - 0.15 * rain;
    if rain > 0.0 {
        l.fog_density = l.fog_density.max(2.3 / (2500.0 - 1800.0 * rain));
    }
    l.overcast = overcast;
    l.rain = rain;
    let gloom = (overcast * 0.5 + rain * 0.5).clamp(0.0, 1.0);
    l.night = l.night.max(0.45 * gloom);
    if l.night >= 0.5 {
        l.night_maps = Some(1.0);
    }
    if snow > 0.0 {
        l.ambient *= 1.0 + 0.35 * snow;
        l.secondary *= 1.0 + 0.2 * snow;
        let day = 1.0 - 0.93 * l.night.clamp(0.0, 1.0);
        l.fog_color = l.fog_color.lerp(Vec3::new(0.86, 0.88, 0.92) * day, 0.4 * snow);
    }
    l.snow = snow;
}

pub fn vehicle_lights(
    v: &VehicleInstance,
    coronas: &mut Vec<Corona>,
    lights: &mut Vec<PointLight>,
    night: f32,
) {
    let ty = &v.ty;
    let value_of = |name: &str| -> f32 {
        let t = name.trim();
        if let Ok(x) = t.parse::<f32>() {
            return x;
        }
        v.var(t).unwrap_or(0.0)
    };
    let mesh_xf = |def_index: usize| -> glam::Mat4 {
        match ty.meshes.iter().position(|m| m.def_index == def_index) {
            Some(i) => v.mesh_local_transform(i),
            None => v.body_rotation(),
        }
    };
    coronas.extend(crate::scene::model_lights_faded(&ty.model, &mesh_xf, v.position, &value_of, &v.light_fade));
    for t in &v.trailers {
        let part_mesh_xf = |def_index: usize| -> glam::Mat4 {
            match t.ty.meshes.iter().position(|m| m.def_index == def_index) {
                Some(i) => t.mesh_local_transform(i),
                None => t.body_rotation(),
            }
        };
        coronas.extend(crate::scene::model_lights_faded(
            &t.ty.model,
            &part_mesh_xf,
            t.position,
            &value_of,
            &t.light_fade,
        ));
    }
    let body = v.body_rotation();
    let forced = omsi_cfg::env::var("OMSI_SPOT_SELECT").ok().and_then(|s| s.trim().parse::<f32>().ok());
    if let Some(sel) = forced.or_else(|| v.var("Spot_Select")) {
        if sel >= 0.0 {
            if let Some(sp) = ty.model.spotlights.get(sel as usize) {
                let vals = sp.values;
                let d = body
                    .transform_vector3(Vec3::new(vals[3], vals[4], vals[5]))
                    .normalize_or_zero();
                let color = [vals[6] / 255.0, vals[7] / 255.0, vals[8] / 255.0];
                let mut apex = Vec3::new(vals[0], vals[1], vals[2]);
                let dl = Vec3::new(vals[3], vals[4], vals[5]).normalize_or_zero();
                let lamps: Vec<[f32; 3]> = ty
                    .model
                    .meshes
                    .iter()
                    .flat_map(|m| m.light_enh.iter().map(|l| l.pos).chain(m.light_enh_2.iter().map(|l| l.pos)))
                    .collect();
                let nose = lamps.iter().map(|l| l[1]).reduce(f32::max);
                let tail = lamps.iter().map(|l| l[1]).reduce(f32::min);
                let bb = ty.def.bounding_box.map(|bb| (bb[4] + bb[1] * 0.5, bb[4] - bb[1] * 0.5));
                if dl.y > 0.3 {
                    let face = match (nose.filter(|n| *n > apex.y), bb) {
                        (Some(n), Some((front, _))) => Some(n.min(front)),
                        (Some(n), None) => Some(n),
                        (None, Some((front, _))) => Some(front.min(apex.y + 1.5)),
                        (None, None) => None,
                    };
                    if let Some(face) = face.filter(|f| *f > apex.y) {
                        apex.y = face + 0.05;
                    }
                } else if dl.y < -0.3 {
                    let face = match (tail.filter(|t| *t < apex.y), bb) {
                        (Some(t), Some((_, rear))) => Some(t.max(rear)),
                        (Some(t), None) => Some(t),
                        (None, Some((_, rear))) => Some(rear.max(apex.y - 1.5)),
                        (None, None) => None,
                    };
                    if let Some(face) = face.filter(|f| *f < apex.y) {
                        apex.y = face - 0.05;
                    }
                }
                let half_width = ty.def.bounding_box.map_or(1.25, |bb| (bb[0] * 0.5).min(1.25));
                let on_face: Vec<f32> = lamps
                    .iter()
                    .filter(|l| (dl.y > 0.3 || dl.y < -0.3) && (l[1] - apex.y).abs() < 0.35)
                    .map(|l| (l[0] - apex.x).abs())
                    .collect();
                let spread = (on_face.iter().sum::<f32>() / on_face.len().max(1) as f32).min(half_width);
                let right = body.transform_vector3(Vec3::X).normalize_or_zero();
                let apex = body.transform_point3(apex);
                let (inner, outer) = (
                    vals.get(10).copied().unwrap_or(30.0),
                    vals.get(11).copied().unwrap_or(70.0),
                );
                let half = |deg: f32| (deg.clamp(1.0, 179.0) * 0.5).to_radians().cos();
                let cone = [half(inner.min(outer)), half(outer)];
                let sides: &[f32] = if spread > 0.1 { &[-1.0, 1.0] } else { &[0.0] };
                for side in sides {
                    let at = v.position + (apex + right * spread * side).as_dvec3();
                    lights.push(PointLight {
                        position: at,
                        radius: vals[9].clamp(10.0, 45.0),
                        color,
                        intensity: VANILLA_HEADLIGHT_INTENSITY / sides.len() as f32 * (0.3 + 0.7 * night),
                        direction: d,
                        cone,
                        mode: LightMode::Vanilla,
                        ..Default::default()
                    });
                    lights.push(PointLight {
                        position: at,
                        radius: vals[9].clamp(10.0, 60.0),
                        color,
                        intensity: HEADLIGHT_INTENSITY / sides.len() as f32,
                        direction: d,
                        cone,
                        core: 1.0,
                        beam: 0.0,
                        mode: LightMode::Enhanced,
                        ..Default::default()
                    });
                }
            }
        }
    }
    if night > 0.05 {
        let mut sections: Vec<(&omsi_model::Model, Option<[f32; 6]>, glam::Mat4, DVec3)> =
            vec![(&ty.model, ty.def.bounding_box, body, v.position)];
        for t in &v.trailers {
            sections.push((&t.ty.model, t.ty.def.bounding_box, t.body_rotation(), t.position));
        }
        let tilt = INTERIOR_SPILL_TILT.to_radians();
        let cone = [INTERIOR_SPILL_INNER.to_radians().cos(), INTERIOR_SPILL_OUTER.to_radians().cos()];
        for (model, bb, xf, origin) in sections {
            let mut count = 0usize;
            let mut sum = Vec3::ZERO;
            let mut color = Vec3::ZERO;
            for il in &model.interior_lights {
                if value_of(&il.variable) >= 0.5 {
                    count += 1;
                    sum += Vec3::from(il.pos);
                    color += Vec3::from(il.color);
                }
            }
            if count == 0 {
                continue;
            }
            let c = sum / count as f32;
            let c = Vec3::new(c.x, c.y, c.z.min(INTERIOR_SPILL_HEIGHT));
            let color = color / count as f32 / 255.0;
            let color = (color * (1.0 - INTERIOR_SPILL_WHITE) + Vec3::splat(color.max_element()) * INTERIOR_SPILL_WHITE).to_array();
            let (half_w, half_l, cx, cy) = match bb {
                Some(b) => ((b[0] * 0.5).min(1.5) - INTERIOR_SPILL_INSET, b[1] * 0.5 - INTERIOR_SPILL_INSET, b[3], b[4]),
                None => (1.25 - INTERIOR_SPILL_INSET, 4.0, 0.0, c.y),
            };
            let strength = (count.min(INTERIOR_SPILL_MAX) as f32 / INTERIOR_SPILL_MAX as f32).max(0.25) * night.clamp(0.0, 1.0);
            let mut faces = vec![
                (Vec3::new(c.x, cy + half_l, c.z), Vec3::Y, INTERIOR_SPILL_END),
                (Vec3::new(c.x, cy - half_l, c.z), -Vec3::Y, INTERIOR_SPILL_END),
            ];
            for k in 0..INTERIOR_SPILL_ALONG {
                let y = cy + half_l * 0.75 * (2.0 * (k as f32 + 0.5) / INTERIOR_SPILL_ALONG as f32 - 1.0);
                faces.push((Vec3::new(cx + half_w, y, c.z), Vec3::X, INTERIOR_SPILL_SIDE));
                faces.push((Vec3::new(cx - half_w, y, c.z), -Vec3::X, INTERIOR_SPILL_SIDE));
            }
            for (at, out, gain) in faces {
                let dir = (out * tilt.cos() - Vec3::Z * tilt.sin()).normalize();
                lights.push(PointLight {
                    position: origin + xf.transform_point3(at).as_dvec3(),
                    radius: INTERIOR_SPILL_RADIUS,
                    color,
                    intensity: gain * strength,
                    direction: xf.transform_vector3(dir).normalize_or_zero(),
                    cone,
                    core: INTERIOR_SPILL_CORE,
                    mode: LightMode::Enhanced,
                    ..Default::default()
                });
            }
        }
    }
}

const INTERIOR_SPILL_SIDE: f32 = 0.45;
const INTERIOR_SPILL_END: f32 = 0.3;
const INTERIOR_SPILL_ALONG: usize = 4;
const INTERIOR_SPILL_RADIUS: f32 = 9.0;
const INTERIOR_SPILL_CORE: f32 = 1.2;
const INTERIOR_SPILL_HEIGHT: f32 = 1.8;
const INTERIOR_SPILL_INSET: f32 = 0.5;
const INTERIOR_SPILL_WHITE: f32 = 0.6;
const INTERIOR_SPILL_MAX: usize = 8;
const INTERIOR_SPILL_TILT: f32 = 40.0;
const INTERIOR_SPILL_INNER: f32 = 30.0;
const INTERIOR_SPILL_OUTER: f32 = 80.0;

const MAP_LIGHT_RANGE: f64 = 600.0;
const CORONA_RANGE: f64 = 1500.0;
const NEAR_MARGIN: f64 = 100.0;
const OCC_HALF: f64 = 1.0;
const OCC_HEIGHT: f64 = 2.5;
const OCC_LIGHT_RANGE: f64 = 400.0;
const OCC_CORONA_RANGE: f64 = 600.0;
const OCC_RECHECK: f64 = 3.0;

const ENCL_REACH: f64 = 12.0;
const ENCL_UP: f64 = 10.0;
const ENCL_MIN_WALLS: usize = 4;
const ENCL_MIN_RADIUS: f32 = 3.0;

fn seg_hit(a: DVec3, b: DVec3, o: &omsi_sim::collision::Obb) -> Option<f64> {
    let [r, f] = o.axes();
    let local = |p: DVec3| {
        let rel = p.truncate() - o.center;
        glam::DVec2::new(rel.dot(r), rel.dot(f))
    };
    let (la, lb) = (local(a), local(b));
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let dl = lb - la;
    for (p, dd, h) in [(la.x, dl.x, o.half.x), (la.y, dl.y, o.half.y)] {
        if dd.abs() < 1e-9 {
            if p.abs() > h {
                return None;
            }
        } else {
            let (u0, u1) = ((-h - p) / dd, (h - p) / dd);
            t0 = t0.max(u0.min(u1));
            t1 = t1.min(u0.max(u1));
            if t0 > t1 {
                return None;
            }
        }
    }
    let (za, zb) = (a.z + (b.z - a.z) * t0, a.z + (b.z - a.z) * t1);
    if za.min(zb) < o.z1 && za.max(zb) > o.z0 {
        Some(t0)
    } else {
        None
    }
}

/// `Some(extent)` when the point sits in a closed room (roof above and walls round it):
/// `extent` is how far the room reaches (m). `None` = outdoors.
fn enclosure(coll: &omsi_sim::collision::CollisionWorld, p: DVec3) -> Option<f32> {
    let parts = coll.obstacles_near(&omsi_sim::collision::Obb::point(p, ENCL_REACH));
    if parts.is_empty() {
        return None;
    }
    // a point inside a solid part (a lamp sunk into a wall or a ceiling)
    if parts.iter().any(|o| {
        o.half.x >= 0.1 && o.half.y >= 0.1 && seg_hit(p, p + DVec3::Z * 1e-3, o).is_some()
    }) {
        return Some(0.0);
    }
    let nearest = |dir: DVec3, len: f64| -> Option<f64> {
        let end = p + dir * len;
        parts
            .iter()
            .filter_map(|o| seg_hit(p, end, o))
            .reduce(f64::min)
            .map(|t| t * len)
    };
    nearest(DVec3::Z, ENCL_UP)?;
    let mut walls = 0usize;
    let mut extent = 0.0f64;
    for k in 0..8 {
        let a = k as f64 * std::f64::consts::FRAC_PI_4;
        if let Some(d) = nearest(DVec3::new(a.cos(), a.sin(), 0.0), ENCL_REACH) {
            walls += 1;
            extent = extent.max(d);
        }
    }
    (walls >= ENCL_MIN_WALLS).then_some(extent as f32)
}

const SHADOW_RANGE: f64 = 150.0;
const SHADOW_REACH: f64 = 40.0;
const SHADOW_MAX: usize = 48;

type OccKey = (i64, i64, i64, u32);

struct OccCache {
    generation: u64,
    map: std::collections::HashMap<OccKey, Vec<omsi_render::Occluder>>,
}

static OCC_CACHE: std::sync::Mutex<Option<OccCache>> = std::sync::Mutex::new(None);

fn gather_occluders(
    coll: &omsi_sim::collision::CollisionWorld,
    seen: &omsi_sim::collision::CollisionWorld,
    pos: DVec3,
    radius: f32,
) -> Vec<omsi_render::Occluder> {
    let reach = (radius as f64).clamp(2.0, SHADOW_REACH);
    let probe = omsi_sim::collision::Obb::point(pos, reach);
    let mut all = seen.obstacles_near(&probe);
    all.extend(coll.obstacles_near(&probe));
    let mut parts: Vec<(f64, omsi_sim::collision::Obb)> = all
        .into_iter()
        .filter(|o| {
            o.mass == 0.0
                && o.pole.is_none()
                && o.half.x.max(o.half.y) >= 0.05
                && o.z1 - o.z0 >= 0.3
                && seg_hit(pos, pos + DVec3::Z * 1e-3, o).is_none()
                && !(o.radius() < 1.0 && (o.center - pos.truncate()).length() < 0.6)
        })
        .map(|o| ((o.center - pos.truncate()).length() - o.radius(), o))
        .filter(|(d, _)| *d < reach)
        .collect();
    parts.sort_by(|a, b| a.0.total_cmp(&b.0));
    parts.truncate(SHADOW_MAX);
    parts
        .into_iter()
        .map(|(_, o)| omsi_render::Occluder {
            center: o.center,
            half: glam::Vec2::new(o.half.x as f32, o.half.y as f32),
            z0: o.z0,
            z1: o.z1,
            heading: o.heading,
        })
        .collect()
}

fn assign_occluders(
    coll: &omsi_sim::collision::CollisionWorld,
    seen: &omsi_sim::collision::CollisionWorld,
    generation: u64,
    scene: &mut Scene,
    camera_pos: DVec3,
    vehicles: &[&VehicleInstance],
) {
    scene.occluders.clear();
    let mut bodies: Vec<(omsi_sim::collision::Obb, omsi_render::Occluder)> = Vec::new();
    for v in vehicles.iter().filter(|v| (v.position - camera_pos).length() < SHADOW_RANGE + 60.0) {
        let mut sections = vec![(v.ty.def.bounding_box, v.body_rotation(), v.position)];
        for t in &v.trailers {
            sections.push((t.ty.def.bounding_box, t.body_rotation(), t.position));
        }
        for (bb, xf, origin) in sections {
            let Some(bb) = bb else { continue };
            let f = xf.transform_vector3(Vec3::Y);
            let heading = (f.x as f64).atan2(f.y as f64);
            let o = omsi_sim::collision::Obb::from_box(bb, origin, heading.to_degrees());
            bodies.push((
                o,
                omsi_render::Occluder {
                    center: o.center,
                    half: glam::Vec2::new(o.half.x as f32 - 0.1, o.half.y as f32 - 0.1),
                    z0: o.z0,
                    z1: o.z1,
                    heading: o.heading,
                },
            ));
        }
    }
    let mut guard = OCC_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = guard.get_or_insert_with(|| OccCache { generation, map: Default::default() });
    if cache.generation != generation || cache.map.len() > 4000 {
        cache.generation = generation;
        cache.map.clear();
    }
    let mut lights = std::mem::take(&mut scene.lights);
    for l in lights.iter_mut() {
        l.occ_first = 0;
        l.occ_count = 0;
        if (l.position - camera_pos).length() > SHADOW_RANGE || l.radius <= 0.0 {
            continue;
        }
        let key = (
            (l.position.x * 2.0).round() as i64,
            (l.position.y * 2.0).round() as i64,
            (l.position.z * 2.0).round() as i64,
            l.radius.to_bits(),
        );
        let occ = cache
            .map
            .entry(key)
            .or_insert_with(|| gather_occluders(coll, seen, l.position, l.radius));
        let first = scene.occluders.len() as u32;
        scene.occluders.extend_from_slice(occ);
        for (o, oc) in &bodies {
            if (o.center - l.position.truncate()).length() < o.radius() + l.radius.min(SHADOW_REACH as f32) as f64 {
                let at = (l.position, l.position + DVec3::Z * 1e-3);
                let core = omsi_sim::collision::Obb {
                    half: (o.half - glam::DVec2::splat(0.6)).max(glam::DVec2::splat(0.05)),
                    z0: o.z0 + 0.6,
                    z1: o.z1 - 0.2,
                    ..*o
                };
                if seg_hit(at.0, at.1, o).is_none() {
                    scene.occluders.push(*oc);
                } else if seg_hit(at.0, at.1, &core).is_none() {
                    // a lamp in the skin of the body (a head or tail light): the body
                    // neither shades nor holds it
                } else {
                    // a light inside the body lights only the inside (and a little through
                    // the windows): negative half width marks the box as a container
                    let mut hollow = *oc;
                    hollow.half.x = -(o.half.x as f32);
                    hollow.half.y = o.half.y as f32;
                    scene.occluders.push(hollow);
                }
            }
        }
        let n = scene.occluders.len() as u32 - first;
        if n > 0 {
            l.occ_first = first;
            l.occ_count = n;
        }
    }
    scene.lights = lights;
}

const BODY_INNER: f32 = 0.25;
const BODY_SKIN: f32 = 0.15;

fn body_hides(v: &VehicleInstance, camera_pos: DVec3, c: DVec3) -> bool {
    let mut sections: Vec<(Option<[f32; 6]>, glam::Mat4, DVec3)> =
        vec![(v.ty.def.bounding_box, v.body_rotation(), v.position)];
    for t in &v.trailers {
        sections.push((t.ty.def.bounding_box, t.body_rotation(), t.position));
    }
    for (bb, xf, origin) in sections {
        let Some(b) = bb else { continue };
        let inv = xf.inverse();
        let e = inv.transform_point3((camera_pos - origin).as_vec3());
        let p = inv.transform_point3((c - origin).as_vec3());
        let h = Vec3::new(b[0], b[1], b[2]) * 0.5;
        let mid = Vec3::new(b[3], b[4], b[5]);
        let (e, p) = (e - mid, p - mid);
        let inside = |q: Vec3, m: f32| q.abs().cmplt(h - Vec3::splat(m)).all();
        if inside(e, -0.2) {
            continue;
        }
        if inside(p, BODY_INNER) {
            return true;
        }
        let lo = -(h - Vec3::splat(BODY_SKIN));
        let hi = h - Vec3::splat(BODY_SKIN);
        let d = p - e;
        let mut t0 = 0.0f32;
        let mut t1 = 1.0f32;
        let mut hit = true;
        for k in 0..3 {
            if d[k].abs() < 1e-6 {
                if e[k] < lo[k] || e[k] > hi[k] {
                    hit = false;
                    break;
                }
            } else {
                let (u0, u1) = ((lo[k] - e[k]) / d[k], (hi[k] - e[k]) / d[k]);
                t0 = t0.max(u0.min(u1));
                t1 = t1.min(u0.max(u1));
                if t0 > t1 {
                    hit = false;
                    break;
                }
            }
        }
        if hit && t0 < 1.0 {
            return true;
        }
    }
    false
}

fn blocked_by_meshes(
    coll: &omsi_sim::collision::CollisionWorld,
    seen: &omsi_sim::collision::CollisionWorld,
    eye: DVec3,
    p: DVec3,
) -> bool {
    let d = p - eye;
    let len = d.length();
    if len < 3.0 || len > 150.0 {
        return false;
    }
    let dir = d / len;
    let end = p - dir * 0.4;
    let steps = (len / 5.0).ceil() as usize;
    for k in 0..=steps {
        let q = eye + dir * (k as f64 * 5.0).min(len);
        let probe = omsi_sim::collision::Obb::point(q, 3.0);
        let mut parts = seen.obstacles_near(&probe);
        parts.extend(coll.obstacles_near(&probe));
        for o in parts {
            if o.mass != 0.0 || o.pole.is_some() || o.half.x.max(o.half.y) < 0.1 || o.z1 - o.z0 < 0.8 {
                continue;
            }
            if seg_hit(eye, eye + DVec3::Z * 1e-3, &o).is_some() {
                continue;
            }
            if seg_hit(eye, end, &o).is_some() {
                return true;
            }
        }
    }
    false
}

fn sees(coll: &omsi_sim::collision::CollisionWorld, eye: DVec3, p: DVec3) -> bool {
    let d = eye - p;
    let l = d.length();
    if l < 1.0 {
        return true;
    }
    let q = p + d / l * 0.4;
    !coll.ray_blocked(eye, q, OCC_HALF, OCC_HEIGHT)
}

#[derive(Default)]
struct NearLights {
    world: usize,
    generation: u64,
    lamps_on: bool,
    centre: DVec3,
    counts: (usize, usize),
    lights: Vec<PointLight>,
    coronas: Vec<Corona>,
    vis_centre: DVec3,
    vis_valid: bool,
    light_vis: Vec<bool>,
    corona_vis: Vec<bool>,
}

static NEAR_LIGHTS: std::sync::Mutex<Option<NearLights>> = std::sync::Mutex::new(None);

pub fn collect(
    world: &World,
    scene: &mut Scene,
    daylight: &Daylight,
    camera_pos: DVec3,
    vehicles: &[&VehicleInstance],
) {
    scene.lights.clear();
    scene.coronas.clear();
    scene.smoke.clear();
    omsi_sim::particles::set_eye(camera_pos);
    let night = daylight.night;
    let coll = world.collision.lock().clone();
    {
        let mut guard = NEAR_LIGHTS.lock().unwrap_or_else(|e| e.into_inner());
        let near = guard.get_or_insert_with(NearLights::default);
        let static_lights = world.static_lights.lock();
        let static_coronas = world.static_coronas.lock();
        let world_id = world as *const World as usize;
        let generation = world
            .tiles_generation
            .load(std::sync::atomic::Ordering::Relaxed);
        let stale = near.world != world_id
            || near.generation != generation
            || near.lamps_on != daylight.lamps_on
            || near.counts != (static_lights.len(), static_coronas.len())
            || (near.centre - camera_pos).length() > NEAR_MARGIN;
        if stale {
            *near = NearLights {
                world: world_id,
                generation,
                lamps_on: daylight.lamps_on,
                centre: camera_pos,
                counts: (static_lights.len(), static_coronas.len()),
                lights: Vec::new(),
                coronas: Vec::new(),
                ..Default::default()
            };
            if daylight.lamps_on {
                near.lights.extend(static_lights.iter().filter(|l| {
                    (l.position - camera_pos).length() < MAP_LIGHT_RANGE + NEAR_MARGIN
                }));
            }
            near.lights.retain_mut(|l| match enclosure(&coll, l.position) {
                Some(ext) => {
                    let r = (ext + 1.0).max(ENCL_MIN_RADIUS);
                    l.radius = l.radius.min(r);
                    if l.core > l.radius {
                        l.core = l.radius;
                    }
                    true
                }
                None => true,
            });
            for c in static_coronas.iter() {
                let on = match &c.switch {
                    LightSwitch::Constant(x) => *x,
                    LightSwitch::Night => daylight.lamps_on as i32 as f32,
                    LightSwitch::Variable(_) => daylight.lamps_on as i32 as f32,
                };
                if on <= 0.0
                    || (c.corona.position - camera_pos).length() > CORONA_RANGE + NEAR_MARGIN
                {
                    continue;
                }
                if enclosure(&coll, c.corona.position).is_some() {
                    continue;
                }
                let mut corona = c.corona;
                corona.brightness *= on.min(1.0);
                near.coronas.push(corona);
            }
        }
        if !near.vis_valid || (near.vis_centre - camera_pos).length() > OCC_RECHECK {
            let lv: Vec<bool> = near
                .lights
                .iter()
                .map(|l| (l.position - camera_pos).length() > OCC_LIGHT_RANGE || sees(&coll, camera_pos, l.position))
                .collect();
            let cv: Vec<bool> = near
                .coronas
                .iter()
                .map(|c| (c.position - camera_pos).length() > OCC_CORONA_RANGE || sees(&coll, camera_pos, c.position))
                .collect();
            near.light_vis = lv;
            near.corona_vis = cv;
            near.vis_centre = camera_pos;
            near.vis_valid = true;
        }
        scene.lights.extend(
            near.lights
                .iter()
                .zip(&near.light_vis)
                .filter(|(l, vis)| **vis && (l.position - camera_pos).length() < MAP_LIGHT_RANGE)
                .map(|(l, _)| *l),
        );
        scene.coronas.extend(
            near.coronas
                .iter()
                .zip(&near.corona_vis)
                .filter(|(c, vis)| **vis && (c.position - camera_pos).length() <= CORONA_RANGE)
                .map(|(c, _)| *c),
        );
    }
    for lamp in world.light_objects.lock().iter() {
        let dist = (lamp.pos - camera_pos).length();
        if dist > 1500.0 {
            continue;
        }
        if dist < OCC_CORONA_RANGE && !sees(&coll, camera_pos, lamp.pos) {
            continue;
        }
        for ((c, _), lit) in lamp.coronas.iter().zip(&lamp.lit) {
            if *lit <= 0.0 {
                continue;
            }
            let mut corona = *c;
            corona.brightness *= lit.min(1.0);
            scene.coronas.push(corona);
        }
    }
    for list in world.particle_objects.lock().values() {
        for po in list {
            if (po.pos - camera_pos).length() < 1500.0 {
                particle_sprites(&po.set, &mut scene.smoke, &mut scene.coronas);
            }
        }
    }
    if omsi_cfg::env::var_os("OMSI_DEBUG_PARTICLES").is_some() {
        if let Some(p) = scene.smoke.first() {
            log::info!("smoke: {} particles from objects, first at ({:.1}, {:.1}, {:.1}) size {:.2} alpha {:.2}", scene.smoke.len(), p.position.x, p.position.y, p.position.z, p.size, p.alpha);
        }
    }
    for v in vehicles {
        let first_corona = scene.coronas.len();
        vehicle_lights(v, &mut scene.coronas, &mut scene.lights, night);
        let mut k = first_corona;
        let seen_world = world.light_occluders.lock().clone();
        scene.coronas.retain_mut(|c| {
            let mine = k >= first_corona;
            k += 1;
            if !mine {
                return true;
            }
            if body_hides(v, camera_pos, c.position) || blocked_by_meshes(&coll, &seen_world, camera_pos, c.position) {
                return false;
            }
            if !c.beam && !c.halo {
                c.size = c.size.min(0.6);
                c.brightness = c.brightness.min(1.0);
            }
            true
        });
        particle_sprites(&v.particles, &mut scene.smoke, &mut scene.coronas);
        for t in &v.trailers {
            particle_sprites(&t.particles, &mut scene.smoke, &mut scene.coronas);
        }
    }
    let (vis, night) = cone_weather();
    scene.coronas.retain_mut(|c| {
        if !c.beam && !c.halo {
            return true;
        }
        if vis >= 2000.0 {
            return false;
        }
        let glow = (night * night + 0.8) * 0.6 * c.brightness;
        let reach = 3.0 * (100.0 / vis.max(1.0)).sqrt() * glow * c.size;
        c.size = if c.beam { 2.0 * reach } else { reach };
        c.brightness = if c.beam { 0.3 } else { 0.2 };
        c.beam_width = vis.max(1.0);
        c.size > 0.05
    });
    if omsi_cfg::env::var_os("OMSI_DEBUG_CONES").is_some() {
        log::info!("cones: visibility {vis:.0} m, dark {night:.2}, {} cones of {} coronas", scene.coronas.iter().filter(|c| c.beam).count(), scene.coronas.len());
        for c in scene.coronas.iter().filter(|c| c.beam).take(4) {
            log::info!("  cone at ({:.1}, {:.1}, {:.1}) dir {:?} radius {:.2} half angles {:.0}/{:.0} deg tex {}", c.position.x, c.position.y, c.position.z, c.direction, c.size, c.inner_cos.to_degrees(), c.cone_cos.to_degrees(), c.texture);
        }
    }
    if omsi_cfg::env::var_os("OMSI_DEBUG_LIGHT").is_some() {
        scene.lights.push(PointLight {
            position: camera_pos + DVec3::new(0.0, 15.0, -2.0),
            radius: 40.0,
            color: [1.0, 0.9, 0.7],
            intensity: 2.0,
            ..Default::default()
        });
        scene.coronas.push(Corona {
            position: camera_pos + DVec3::new(0.0, 15.0, 0.0),
            size: 1.0,
            color: [1.0, 0.9, 0.7],
            brightness: 1.0,
            direction: Vec3::ZERO,
            cone_cos: -1.0,
            ..Default::default()
        });
        log::info!(
            "static lights: {:?}",
            world
                .static_lights
                .lock()
                .iter()
                .take(3)
                .collect::<Vec<_>>()
        );
        log::info!(
            "static coronas: {:?}",
            world
                .static_coronas
                .lock()
                .iter()
                .take(3)
                .collect::<Vec<_>>()
        );
    }
    scene.lights.sort_by(|a, b| (a.position - camera_pos).length_squared().total_cmp(&(b.position - camera_pos).length_squared()));
    let generation = world
        .tiles_generation
        .load(std::sync::atomic::Ordering::Relaxed);
    let seen = world.light_occluders.lock().clone();
    assign_occluders(&coll, &seen, generation, scene, camera_pos, vehicles);
}

pub fn particle_sprites(set: &omsi_sim::particles::ParticleSet, smoke: &mut Vec<omsi_render::SmokeParticle>, coronas: &mut Vec<Corona>) {
    for (p, def) in set.particles() {
        let alpha = p.alpha();
        if alpha <= 0.002 {
            continue;
        }
        if def.emissive {
            coronas.push(Corona {
                position: p.pos,
                size: (p.size() * 0.5).max(0.02),
                color: p.color,
                brightness: alpha,
                direction: Vec3::ZERO,
                cone_cos: -1.0,
                z_offset: 0.0,
                ..Default::default()
            });
        } else {
            smoke.push(omsi_render::SmokeParticle { position: p.pos, size: p.size() * 0.5, color: p.color, alpha });
        }
    }
}

pub fn load_smoke_texture(renderer: &mut omsi_render::Renderer, root: &std::path::Path) {
    let path = omsi_cfg::resolve_path(root, "Texture/rauch.tga");
    match omsi_texture::decode_file(&path) {
        Ok(img) => renderer.set_smoke_texture(&img),
        Err(e) => log::warn!("smoke texture {}: {e}", path.display()),
    }
}

struct CoronaTextures {
    ids: std::collections::HashMap<std::path::PathBuf, u16>,
    pending: Vec<(u16, std::path::PathBuf)>,
    root: Option<std::path::PathBuf>,
}

static CORONA_TEXTURES: std::sync::Mutex<Option<CoronaTextures>> = std::sync::Mutex::new(None);

pub fn set_corona_root(root: &std::path::Path) {
    let mut g = CORONA_TEXTURES.lock().unwrap_or_else(|e| e.into_inner());
    let t = g.get_or_insert_with(|| CoronaTextures { ids: Default::default(), pending: Vec::new(), root: None });
    t.root = Some(root.to_path_buf());
}

fn texture_id_of(path: std::path::PathBuf) -> u16 {
    let mut g = CORONA_TEXTURES.lock().unwrap_or_else(|e| e.into_inner());
    let t = g.get_or_insert_with(|| CoronaTextures { ids: Default::default(), pending: Vec::new(), root: None });
    if let Some(id) = t.ids.get(&path) {
        return *id;
    }
    let id = (t.ids.len() + 1).min(u16::MAX as usize) as u16;
    t.ids.insert(path.clone(), id);
    t.pending.push((id, path));
    id
}

pub fn corona_texture_id(model_dir: &std::path::Path, name: &str) -> u16 {
    static KNOWN: std::sync::Mutex<Option<std::collections::HashMap<(std::path::PathBuf, String), u16>>> = std::sync::Mutex::new(None);
    let key = (model_dir.to_path_buf(), name.to_string());
    if let Some(&id) = KNOWN.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|m| m.get(&key)) {
        return id;
    }
    let root = CORONA_TEXTURES.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|t| t.root.clone()).unwrap_or_default();
    let mut id = 0;
    for d in crate::scene::texture_dirs(&root, model_dir) {
        let p = omsi_cfg::resolve_path(&d, name);
        if omsi_cfg::vfs::is_file(&p) {
            id = texture_id_of(p);
            break;
        }
    }
    KNOWN.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Default::default).insert(key, id);
    id
}

fn stock_texture_id(name: &str) -> u16 {
    static KNOWN: std::sync::Mutex<Option<std::collections::HashMap<(std::path::PathBuf, String), u16>>> = std::sync::Mutex::new(None);
    let root = CORONA_TEXTURES.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|t| t.root.clone()).unwrap_or_default();
    let key = (root, name.to_string());
    if let Some(&id) = KNOWN.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|m| m.get(&key)) {
        return id;
    }
    let p = omsi_cfg::resolve_path(&key.0, &format!("Texture/{name}"));
    let id = if omsi_cfg::vfs::is_file(&p) { texture_id_of(p) } else { 0 };
    KNOWN.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Default::default).insert(key, id);
    id
}

pub fn cone_texture_id() -> u16 {
    stock_texture_id("light_cone.bmp")
}

pub fn glow_texture_id() -> u16 {
    stock_texture_id("licht.bmp")
}

pub fn star_texture_id() -> u16 {
    stock_texture_id("light_effect1.bmp")
}

pub fn upload_corona_textures(renderer: &mut omsi_render::Renderer) {
    let pending = {
        let mut g = CORONA_TEXTURES.lock().unwrap_or_else(|e| e.into_inner());
        match g.as_mut() {
            Some(t) => std::mem::take(&mut t.pending),
            None => return,
        }
    };
    for (id, path) in pending {
        match omsi_texture::decode_file(&path) {
            Ok(img) => renderer.set_corona_texture(id, &img),
            Err(e) => log::warn!("corona picture {}: {e}", path.display()),
        }
    }
}

static CONE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

static CONE_NIGHT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn set_cone_strength(fog_visibility_m: f32, _precip: f32, night: f32) {
    CONE.store(fog_visibility_m.to_bits(), std::sync::atomic::Ordering::Relaxed);
    CONE_NIGHT.store(night.clamp(0.0, 1.0).to_bits(), std::sync::atomic::Ordering::Relaxed);
}

fn cone_weather() -> (f32, f32) {
    let vis = f32::from_bits(CONE.load(std::sync::atomic::Ordering::Relaxed));
    let night = f32::from_bits(CONE_NIGHT.load(std::sync::atomic::Ordering::Relaxed));
    (if vis > 0.0 { vis } else { 1.0e6 }, night)
}

pub fn vehicle_velocity(v: &omsi_sim::VehicleInstance) -> glam::Vec3 {
    let h = v.heading.to_radians();
    glam::Vec3::new(h.sin() as f32, h.cos() as f32, 0.0) * v.physics.speed
}