//! Scenery must enter the listener's bus through its body, not become an own-bus sound.
use omsi_audio::{AudioEngine, Bus, Clip, EventSource, SoundEvent, SoundSet};
use omsi_vehicle::{SoundCfg, SoundEntry};
use glam::Mat4;
use std::{path::Path, sync::Arc};

fn fixture(viewpoint: i32, hz: f64) -> (AudioEngine, SoundSet) {
    let engine = AudioEngine::new_offline(48000, 2);
    engine.cache_clip("cock.WAV", Arc::new(Clip { sample_rate: 48000, channels: 1,
        samples: (0..96000).map(|i| (12000.0 * (i as f64 * hz / 48000.0 *
            std::f64::consts::TAU).sin()) as i16).collect() }));
    // The actual cock.cfg shape: untagged, triggered, 3D at the object, 10 m range.
    let cfg = SoundCfg { sounds: vec![SoundEntry { file: "cock.WAV".into(), volume: 0.8,
        pos: Some([0.0; 3]), range: 10.0, viewpoint, triggers: vec!["cock".into()],
        ..Default::default() }], unknown_keywords: vec![] };
    let set = SoundSet::new_world(&engine, &cfg, Path::new(""));
    (engine, set)
}

fn update(engine: &AudioEngine, set: &mut SoundSet, fired: bool) {
    let events = if fired { vec![SoundEvent::trigger(EventSource::Scenery, 0, "cock")] }
        else { vec![] };
    set.update_events(engine, &|_| None, &Mat4::IDENTITY, &events, &|_| None);
}
fn energy(engine: &AudioEngine) -> f64 {
    let mut out = vec![0.0; 38400]; engine.render_offline(&mut out);
    out[33600..].iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / 4800.0
}

#[test]
fn scenery_is_quieter_and_duller_inside_and_follows_doors_during_a_call() {
    // This is the only test that changes opening in this integration-test process.
    let (engine, mut set) = fixture(0, 2500.0);
    omsi_audio::soundset::set_outside_open(Some(0.0));
    update(&engine, &mut set, true);
    let outside = energy(&engine);
    assert!((engine.voice_state(1).unwrap().0.gain - 0.8).abs() < 1e-6);
    set.set_muffled(true);
    for _ in 0..5 {
        engine.clock().advance(std::time::Duration::from_millis(100));
        update(&engine, &mut set, false);
    }
    let closed = energy(&engine);
    let parameters = engine.voice_state(1).unwrap().0;
    assert!((parameters.gain - 0.2).abs() < 1e-6);
    assert!((parameters.lowpass_hz - 450.0).abs() < 0.01);
    assert!(closed < outside * 0.02, "cock must not be equally loud in the closed bus");
    omsi_audio::soundset::set_outside_open(Some(0.1)); // a C2 rear door
    update(&engine, &mut set, false);
    let opened = energy(&engine);
    let parameters = engine.voice_state(1).unwrap().0;
    assert!((parameters.gain - 0.32).abs() < 1e-6);
    assert!((parameters.lowpass_hz - 1800.0).abs() < 0.01);
    assert!(opened > closed * 4.0 && opened < outside);
    assert_eq!(engine.voice_count(), 1);
    assert_eq!(engine.stats().retired, 0, "door changes must not restart the cock call");
    // Routing remains Ambience, and an already-running source also loses level again.
    engine.set_bus_gain(Bus::Ambience, 0.0);
    assert!(energy(&engine) < outside * 0.0001);
    omsi_audio::soundset::set_outside_open(None);
}

#[test]
fn world_constructor_keeps_non_ai_viewpoint_admission() {
    let (engine, mut set) = fixture(4, 1000.0);
    update(&engine, &mut set, true);
    assert_eq!(engine.voice_count(), 0, "scenery must not be assigned traffic's AI bit");
    let (engine, mut set) = fixture(1, 1000.0);
    update(&engine, &mut set, true);
    assert_eq!(engine.voice_count(), 1);
}
