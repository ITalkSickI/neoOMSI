//! Interior PA transmission through the bus, using the public file-event path.
use ::audio::{AudioEngine, Clip, EventSource, Listener, SoundEvent, SoundSet};
use ::legacy_vehicle::{SoundCfg, SoundEntry};
use std::{path::Path, sync::Arc, time::Duration};

fn fixture(inside: bool, position: glam::Vec3, hz: f64) -> (AudioEngine, SoundSet) {
    let engine = AudioEngine::new_offline(48000, 2);
    engine.set_listener(Listener { position, ..Default::default() });
    // No viewpoint is how the tested C2 declares its interior announcement entry.
    let cfg = SoundCfg { sounds: vec![SoundEntry { file: "0".into(), volume: 1.0,
        triggers: vec!["announce".into()], ..Default::default() }], unknown_keywords: Vec::new() };
    engine.cache_clip(Path::new("Announcements").join("pa.wav"), Arc::new(Clip { sample_rate: 48000, channels: 1,
        samples: (0..96000).map(|i| (12000.0 * (i as f64 * hz / 48000.0 *
            std::f64::consts::TAU).sin()) as i16).collect() }));
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    set.set_inside(inside);
    (engine, set)
}

fn fire(engine: &AudioEngine, set: &mut SoundSet, opening: f32) {
    let events = [SoundEvent::file(EventSource::Player, 0, "announce", "Announcements/pa.wav")];
    set.update_events(engine, &|name| (name == "Snd_OutsideVol").then_some(opening),
        &glam::Mat4::IDENTITY, &events, &|_| None);
}

fn energy(inside: bool, opening: f32, position: glam::Vec3, hz: f64) -> f64 {
    let (engine, mut set) = fixture(inside, position, hz);
    fire(&engine, &mut set, opening);
    let mut out = vec![0.0; 9600]; engine.render_offline(&mut out);
    out[4800..].iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / 4800.0
}

#[test]
fn outside_pa_is_audible_but_quieter_and_duller_with_closed_doors() {
    let at_bus = glam::Vec3::ZERO;
    let interior = energy(true, 0.0, at_bus, 4000.0);
    let shut = energy(false, 0.0, at_bus, 4000.0);
    let open = energy(false, 0.5, at_bus, 4000.0);
    assert!(shut > 1e-7 && shut < interior * 0.001);
    assert!(open > shut * 8.0 && open < interior * 0.1);
    let low_shut = energy(false, 0.0, at_bus, 400.0);
    let low_inside = energy(true, 0.0, at_bus, 400.0);
    assert!(shut / interior < low_shut / low_inside * 0.3,
        "closed-body transfer must remove treble, not only gain");
}

#[test]
fn an_ongoing_announcement_follows_camera_and_doors_without_restarting() {
    let (engine, mut set) = fixture(true, glam::Vec3::ZERO, 4000.0);
    fire(&engine, &mut set, 0.0);
    let mut block = vec![0.0; 1920]; engine.render_offline(&mut block);
    set.set_inside(false);
    for _ in 0..30 {
        engine.clock().advance(Duration::from_millis(20));
        set.update_events(&engine, &|name| (name == "Snd_OutsideVol").then_some(0.0),
            &glam::Mat4::IDENTITY, &[], &|_| None);
        engine.render_offline(&mut block);
    }
    let outside = engine.voice_state(1).expect("original voice is still running").0;
    assert!((outside.gain - 0.08).abs() < 1e-6);
    assert!((outside.lowpass_hz - 900.0).abs() < 0.01);
    set.update_events(&engine, &|name| (name == "Snd_OutsideVol").then_some(0.5),
        &glam::Mat4::IDENTITY, &[], &|_| None);
    engine.render_offline(&mut block);
    let open = engine.voice_state(1).unwrap().0;
    assert!((open.gain - 0.30).abs() < 1e-6);
    assert!((open.lowpass_hz - 4000.0).abs() < 0.02);
    assert_eq!(engine.voice_count(), 1);
    assert_eq!(engine.stats().retired, 0);
    assert!(engine.stats().clean());
}

#[test]
fn an_outside_camera_hears_the_pa_from_the_bus_with_distance_falloff() {
    let near = energy(false, 0.0, glam::Vec3::ZERO, 1000.0);
    let far = energy(false, 0.0, glam::Vec3::new(200.0, 0.0, 0.0), 1000.0);
    assert!(far < near * 0.1);
}
