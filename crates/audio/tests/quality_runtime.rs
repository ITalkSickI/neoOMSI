use ::audio::{AudioEngine, Bus, Clip, EventSource, SoundEvent, SoundSet, SoundState};
use ::legacy_vehicle::{SoundCfg, SoundEntry, VolCurve};
use std::{path::Path, sync::Arc};
fn config(file: &str) -> SoundCfg {
    SoundCfg { sounds: vec![SoundEntry { file: file.into(), volume: 1.0,
        triggers: vec!["announce".into()],
        vol_curves: vec![VolCurve { variable: "level".into(), points: vec![(0.0, 0.0), (1.0, 1.0)] }],
        ..Default::default() }], unknown_keywords: Vec::new() }
}
fn cache(engine: &AudioEngine) {
    engine.cache_clip("voice.wav", Arc::new(Clip { sample_rate: 48000, channels: 1, samples: vec![16384; 4800] }));
}
#[test]
fn coupled_parts_receive_file_events_and_their_fire_time_level() {
    let engine = AudioEngine::new_offline(48000, 2); cache(&engine);
    let cfg = config("0");
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    set.set_inside(true);
    set.add_part(1, SoundSet::new(&engine, &cfg, Path::new("")));
    let events = [SoundEvent::file(EventSource::Player, 0, "announce", "voice.wav").with_vars(vec![0.75])];
    let current = |_: &str| Some(0.0); let slots = |_: &str| Some(0);
    set.update_events(&engine, &current, &glam::Mat4::IDENTITY, &events, &slots);
    set.update_parts_events(&engine, &current, &|_| Some(glam::Mat4::IDENTITY), &events, &slots);
    assert_eq!(engine.voice_count(), 2);
    assert_eq!(set.parts[0].1.state(0), Some(SoundState::Running));
    for (_, gain, _) in set.playing(&engine) { assert!((gain - 0.75).abs() < 1e-6); }
    for (_, gain, _) in set.parts[0].1.playing(&engine) { assert!((gain - 0.75).abs() < 1e-6); }
    // Subsequent frame updates must keep the announcement bus and its held fire-time peak.
    set.update_events(&engine, &current, &glam::Mat4::IDENTITY, &[], &slots);
    set.update_parts_events(&engine, &current, &|_| Some(glam::Mat4::IDENTITY), &[], &slots);
    assert!((set.playing(&engine)[0].1 - 0.75).abs() < 1e-6);
    engine.set_bus_gain(Bus::Announcement, 0.0);
    let mut out = vec![0.0; 8000]; engine.render_offline(&mut out);
    assert!(out[out.len()-1].abs() < 0.02);
}
#[test]
fn coupled_parts_keep_normal_event_snapshots_and_stop_on_detach() {
    let engine = AudioEngine::new_offline(48000, 2); cache(&engine);
    let cfg = config("voice.wav");
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    set.add_part(1, SoundSet::new(&engine, &cfg, Path::new("")));
    let events = [SoundEvent::trigger(EventSource::Traffic, 0, "announce").with_vars(vec![0.5])];
    set.update_parts_events(&engine, &|_| Some(0.0), &|_| Some(glam::Mat4::IDENTITY), &events, &|_| Some(0));
    assert_eq!(engine.voice_count(), 1);
    assert!((set.parts[0].1.playing(&engine)[0].1 - 0.5).abs() < 1e-6);
    set.update_parts_events(&engine, &|_| Some(1.0), &|_| None, &[], &|_| Some(0));
    assert_eq!(engine.voice_count(), 0);
}
