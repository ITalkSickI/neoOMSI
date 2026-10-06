use super::*;
use crate::{AudioEngine, Listener};
use ::legacy_vehicle::SoundCfg;
use std::path::Path;

fn fixture(connected: bool) -> (AudioEngine, SoundSet) {
    let engine = AudioEngine::new_offline(48000, 2);
    let cfg = SoundCfg { sounds: vec![], unknown_keywords: vec![] };
    let mut main = SoundSet::new(&engine, &cfg, Path::new(""));
    let mut rear = SoundSet::new(&engine, &cfg, Path::new(""));
    main.set_hull(Some([2.52, 9.6, 2.5, 0.0, 1.02988, 1.7]));
    rear.set_hull(Some([2.52, 6.8, 2.5, 0.0, 0.2151, 1.7]));
    rear.set_cabin_connected(connected);
    main.add_part(0, rear);
    (engine, main)
}
fn rear_xf(main: Mat4) -> Mat4 { main * Mat4::from_translation(Vec3::new(0.0, -9.07687, 0.0)) }

#[test]
fn c2_front_rear_and_bellows_share_cabin_membership_in_a_rotated_world() {
    let (engine, mut set) = fixture(true);
    let xf = Mat4::from_translation(Vec3::new(70.0, 30.0, 2.0)) * Mat4::from_rotation_z(1.1);
    let rear = rear_xf(xf);
    for y in [3.0, 0.0, -3.0, -4.6, -6.0, -9.0, -11.0] {
        let listener = xf.transform_point3(Vec3::new(0.0, y, 1.7));
        engine.set_listener(Listener { position: listener, ..Default::default() });
        assert_eq!(set.prepare_listener_cabin(&engine, &xf, &|_| Some(rear)), 1.0, "y={y}");
        assert_eq!(set.inside_factor(0.0, &xf, listener), 1.0);
        assert_eq!(set.parts[0].1.inside_factor(0.0, &rear, listener), 1.0);
    }
    let outside = xf.transform_point3(Vec3::new(3.0, -4.6, 1.7));
    assert_eq!(set.camera_cabin_factor(outside, &xf, &|_| Some(rear)), Some(0.0));
}

#[test]
fn a_closed_coupling_keeps_separate_compartments() {
    let (engine, mut set) = fixture(false);
    let rear = rear_xf(Mat4::IDENTITY);
    let listener = Vec3::new(0.0, -9.0, 1.7);
    engine.set_listener(Listener { position: listener, ..Default::default() });
    assert_eq!(set.prepare_listener_cabin(&engine, &Mat4::IDENTITY, &|_| Some(rear)), 0.0);
    assert_eq!(set.inside_factor(0.0, &Mat4::IDENTITY, listener), 0.0);
    assert_eq!(set.parts[0].1.inside_factor(0.0, &rear, listener), 1.0);
}

#[test]
fn angled_joint_has_a_narrow_cabin_bridge_without_an_enclosing_outside_box() {
    let (_engine, set) = fixture(true);
    let rear = Mat4::from_translation(Vec3::new(1.2, -9.0, 0.0)) * Mat4::from_rotation_z(0.1);
    let a = Vec3::new(0.0, 1.02988 - 4.85, 1.7);
    let b = rear.transform_point3(Vec3::new(0.0, 0.2151 + 3.45, 1.7));
    assert_eq!(set.camera_cabin_factor((a + b) * 0.5, &Mat4::IDENTITY, &|_| Some(rear)), Some(1.0));
    assert_eq!(set.camera_cabin_factor(Vec3::new(4.0, -4.6, 1.7), &Mat4::IDENTITY,
        &|_| Some(rear)), Some(0.0));
}

#[test]
fn missing_part_cannot_leave_a_phantom_shared_cabin() {
    let (engine, mut set) = fixture(true);
    let listener = Vec3::new(0.0, -9.0, 1.7);
    engine.set_listener(Listener { position: listener, ..Default::default() });
    assert_eq!(set.prepare_listener_cabin(&engine, &Mat4::IDENTITY, &|_| Some(rear_xf(Mat4::IDENTITY))), 1.0);
    assert_eq!(set.prepare_listener_cabin(&engine, &Mat4::IDENTITY, &|_| None), 0.0);
    assert_eq!(set.inside_factor(0.0, &Mat4::IDENTITY, listener), 0.0);
}
