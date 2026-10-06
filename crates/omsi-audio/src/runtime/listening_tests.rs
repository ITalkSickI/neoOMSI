//! Listening regressions: initial cabin state, authored sides and door transfer.
use super::*;
use crate::{AudioEngine, Clip, SoundSet};
use omsi_vehicle::{SoundCfg, SoundEntry};
use std::{path::Path, sync::Arc};
fn fixture(viewpoint: i32, exterior: bool) -> (AudioEngine, SoundSet) {
    let engine = AudioEngine::new_offline(48000, 2);
    engine.cache_clip("tone.wav", Arc::new(Clip { sample_rate: 48000, channels: 1, samples: vec![8192; 4800] }));
    let cfg = SoundCfg { sounds: vec![SoundEntry { file: "tone.wav".into(), volume: 1.0, viewpoint, ..Default::default() }], unknown_keywords: Vec::new() };
    let mut set = if exterior { SoundSet::new_exterior(&engine, &cfg, Path::new("")) }
        else { SoundSet::new(&engine, &cfg, Path::new("")) };
    set.set_inside(true); set.set_muffled(true);
    (engine, set)
}
#[test]
fn a_new_muffled_set_starts_muffled_without_a_loud_burst() {
    let (engine, mut set) = fixture(0, true);
    let (inside, muffled) = set.advance_blend();
    assert_eq!((inside, muffled), (1.0, 1.0));
    set.set_muffled(false);
    engine.clock().advance(std::time::Duration::from_millis(20));
    let (_, muffled) = set.advance_blend();
    assert!(muffled < 1.0 && muffled > 0.9, "later transitions still fade");
}
#[test]
fn own_non_3d_cabin_sound_is_centred_and_has_no_origin_distance() {
    let (engine, mut set) = fixture(0, false);
    engine.set_listener(crate::Listener { position: glam::Vec3::new(100.0, 0.0, 0.0), ..Default::default() });
    set.update(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &[]);
    let heard = set.playing(&engine)[0].1;
    assert_eq!(heard, 1.0, "the vehicle origin must not attenuate a non-3D cabin recording");
    let mut out = vec![0.0; 2000]; engine.render_offline(&mut out);
    for pair in out.chunks_exact(2) { assert_eq!(pair[0], pair[1]); }
    assert!(out.last().unwrap() > &0.12);
}
#[test]
fn own_exterior_recording_is_damped_in_cab_even_without_an_open_variable() {
    // No global mutation: the default transfer can be checked through a pure helper.
    let def = SoundEntry { viewpoint: 1, ..Default::default() };
    assert_eq!(outside::entry_transfer_at(&def, 1.0, None).0, 0.25);
    assert_eq!(outside::entry_transfer_at(&def, 0.0, None).0, 1.0);
}
#[test]
fn closing_doors_reduces_own_exterior_level_and_high_frequencies() {
    let def = SoundEntry { viewpoint: 1, ..Default::default() };
    let shut = outside::entry_transfer_at(&def, 1.0, Some(0.0));
    let open = outside::entry_transfer_at(&def, 1.0, Some(0.5));
    assert_eq!(shut.0, 0.25); assert_eq!(open.0, 1.0);
    assert!(shut.1 < open.1);
    let interior = SoundEntry { viewpoint: 2, ..Default::default() };
    assert_eq!(outside::entry_transfer_at(&interior, 1.0, Some(0.0)), (1.0, 0.0));
    let mixed = SoundEntry { viewpoint: 3, ..Default::default() };
    assert_eq!(outside::entry_transfer_at(&mixed, 1.0, Some(0.0)), (1.0, 0.0));
}
#[test]
fn an_explicit_outside_curve_does_not_receive_the_gain_twice() {
    let def = SoundEntry { viewpoint: 1, vol_curves: vec![omsi_vehicle::VolCurve {
        variable: "Snd_OutsideVol".into(), points: vec![(0.0, 0.25), (0.5, 1.0)] }], ..Default::default() };
    let (gain, cutoff) = outside::entry_transfer_at(&def, 1.0, Some(0.0));
    assert_eq!(gain, 1.0); assert!((cutoff - 450.0).abs() < 0.01);
}

#[test]
fn a_declared_cab_view_is_inside_before_the_spawn_camera_arrives() {
    let (_engine, mut set) = fixture(1, false);
    set.set_hull(Some([2.0, 12.0, 3.0, 0.0, 0.0, 1.5]));
    assert_eq!(set.inside_factor(1.0, &glam::Mat4::IDENTITY, glam::Vec3::splat(1000.0)), 1.0);
    set.set_inside(false);
    assert_eq!(set.inside_factor(0.0, &glam::Mat4::IDENTITY, glam::Vec3::splat(1000.0)), 0.0);
}

#[test]
fn interior_pa_reaches_outside_softly_and_doors_open_its_timbre() {
    for viewpoint in [0, 2] {
        let def = SoundEntry { viewpoint, ..Default::default() };
        let closed = outside::announcement_transfer_at(&def, 0.0, Some(0.0));
        let opened = outside::announcement_transfer_at(&def, 0.0, Some(0.5));
        assert!((closed.0 - 0.08).abs() < 1e-6);
        assert!((closed.1 - 900.0).abs() < 0.01);
        assert!((opened.0 - 0.30).abs() < 1e-6);
        assert!((opened.1 - 4000.0).abs() < 0.02);
        assert_eq!(outside::announcement_transfer_at(&def, 1.0, Some(0.0)), (1.0, 0.0));
        assert_eq!(outside::announcement_transfer_at(&def, 0.0, None), closed);
    }
}

#[test]
fn announcement_curves_keep_authored_gain_without_duplicate_door_attenuation() {
    let def = SoundEntry { vol_curves: vec![omsi_vehicle::VolCurve {
        variable: "Snd_OutsideVol".into(), points: vec![(0.0, 0.2), (0.5, 1.0)] }],
        ..Default::default() };
    let (gain, cutoff) = outside::announcement_transfer_at(&def, 0.0, Some(0.0));
    assert_eq!(gain, 1.0);
    assert!((cutoff - 900.0).abs() < 0.01);
    for viewpoint in [1, 3] {
        let explicit = SoundEntry { viewpoint, ..Default::default() };
        assert_eq!(outside::announcement_transfer(&explicit, false, 0.0, 0.0, None),
            outside::entry_transfer(&explicit, false, 0.0, 0.0));
    }
}
