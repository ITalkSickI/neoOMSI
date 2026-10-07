//! Free/exterior camera inside an articulated C2, including rear-door sound leakage.
use ::audio::{AudioEngine, Clip, EventSource, Listener, SoundEvent, SoundSet};
use ::legacy_vehicle::{SoundCfg, SoundEntry};
use glam::{Mat4, Vec3};
use std::{path::Path, sync::Arc};

fn fixture(announcement: bool) -> (AudioEngine, SoundSet, Mat4) {
    let engine = AudioEngine::new_offline(48000, 2);
    let cfg = SoundCfg { sounds: vec![SoundEntry {
        file: if announcement { "0" } else { "engine.wav" }.into(), volume: 1.0,
        viewpoint: if announcement { 0 } else { 1 },
        triggers: if announcement { vec!["announce".into()] } else { vec![] },
        ..Default::default() }], unknown_keywords: vec![] };
    let clip = Arc::new(Clip { sample_rate: 48000, channels: 1,
        samples: vec![8192; if announcement { 480 } else { 96000 }] });
    engine.cache_clip("engine.wav", clip.clone()); engine.cache_clip(Path::new("Announcements").join("pa.wav"), clip);
    let mut main = SoundSet::new(&engine, &cfg, Path::new(""));
    let mut rear = SoundSet::new(&engine, &cfg, Path::new(""));
    main.set_hull(Some([2.52, 9.6, 2.5, 0.0, 1.02988, 1.7]));
    rear.set_hull(Some([2.52, 6.8, 2.5, 0.0, 0.2151, 1.7]));
    rear.set_cabin_connected(true); main.add_part(0, rear);
    (engine, main, Mat4::from_translation(Vec3::new(0.0, -9.07687, 0.0)))
}
fn render(engine: &AudioEngine, samples: usize) -> Vec<f32> {
    let mut output = vec![0.0; samples]; engine.render_offline(&mut output); output
}

#[test]
fn physical_camera_inside_either_section_uses_rear_door_opening() {
    let (engine, mut set, rear) = fixture(false);
    // This single test owns the global opening in this integration-test process.
    for y in [3.0, -4.6, -9.0] {
        engine.set_listener(Listener { position: Vec3::new(0.0, y, 1.7), ..Default::default() });
        assert_eq!(set.prepare_listener_cabin(&engine, &Mat4::IDENTITY, &|_| Some(rear)), 1.0);
        set.set_muffled(true);
        let mut closed_output = Vec::new();
        for opening in [0.0, 0.1] { // actual C2 script: rear door_4/5 each contributes 0.05
            ::audio::soundset::set_outside_open(Some(opening));
            let var = |name: &str| (name == "Snd_OutsideVol").then_some(opening);
            set.update_events(&engine, &var, &Mat4::IDENTITY, &[], &|_| None);
            set.update_parts_events(&engine, &var, &|_| Some(rear), &[], &|_| None);
            let output = render(&engine, 9600);
            let state = engine.voice_state(2).expect("rear engine is still the same voice").0;
            assert!((state.gain - (0.25 + 1.5 * opening)).abs() < 1e-6);
            assert!((state.lowpass_hz - 450.0 * (1.0 + 30.0 * opening)).abs() < 0.01);
            if opening == 0.0 { closed_output = output; }
            else {
                let rms = |out: &[f32]| (out[8000..].iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / 1600.0).sqrt();
                assert!(rms(&output) > rms(&closed_output) * 1.4, "rear door must audibly raise the level");
            }
        }
    }
    ::audio::soundset::set_outside_open(None);
    assert_eq!(engine.voice_count(), 2);
}

#[test]
fn front_rear_and_joint_cameras_receive_interior_pa_and_its_hall() {
    for y in [3.0, -4.6, -9.0] {
        let (engine, mut set, rear) = fixture(true);
        engine.set_listener(Listener { position: Vec3::new(0.0, y, 1.7), ..Default::default() });
        // Leave set_inside(false): this reproduces entering by the exterior camera.
        set.prepare_listener_cabin(&engine, &Mat4::IDENTITY, &|_| Some(rear));
        let events = [SoundEvent::file(EventSource::Player, 0, "announce", "Announcements/pa.wav")];
        let var = |name: &str| (name == "Snd_OutsideVol").then_some(0.0);
        set.update_events(&engine, &var, &Mat4::IDENTITY, &events, &|_| None);
        set.update_parts_events(&engine, &var, &|_| Some(rear), &events, &|_| None);
        for id in [1, 2] {
            let state = engine.voice_state(id).unwrap().0;
            assert_eq!(state.gain, 1.0); assert_eq!(state.lowpass_hz, 0.0);
        }
        let output = render(&engine, 10000);
        assert!(output[3000..8000].iter().any(|x| x.abs() > 0.0001), "no cabin hall at y={y}");
        assert!(engine.stats().clean());
    }
}
