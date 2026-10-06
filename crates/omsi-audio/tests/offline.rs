//! Offline playback tests: a sound set driven without an audio device, on a manual clock,
//! so the mixer runs exactly as it would in the output callback - just into a buffer we can
//! look at.

use omsi_audio::{AudioEngine, Clip, EventSource, SoundEvent, SoundSet, SoundState};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const RATE: u32 = 48_000;
const BLOCK: usize = 960; // 20 ms
const DT: Duration = Duration::from_millis(20);

fn cfg(text: &str) -> omsi_vehicle::SoundCfg {
    omsi_vehicle::SoundCfg::parse(&omsi_cfg::CfgFile::from_str("sound.cfg", text))
}

/// A quiet constant clip, `seconds` long.
fn clip(seconds: f32) -> Arc<Clip> {
    let n = (RATE as f32 * seconds) as usize;
    Arc::new(Clip {
        sample_rate: RATE,
        channels: 1,
        samples: vec![16_384; n.max(1)],
    })
}

fn engine_with(cfg: &omsi_vehicle::SoundCfg) -> AudioEngine {
    let engine = AudioEngine::new_offline(RATE, 2);
    for path in SoundSet::clip_paths(cfg, Path::new("")) {
        engine.cache_clip(path, clip(0.25));
    }
    engine
}

fn step(set: &mut SoundSet, engine: &AudioEngine, triggers: &[String], out: &mut [f32]) {
    engine.clock().advance(DT);
    set.update(engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, triggers);
    engine.render_offline(out);
}

#[test]
fn a_triggered_horn_plays_then_ends() {
    let cfg = cfg(include_str!("fixtures/soundcfg/trigger.cfg"));
    let engine = engine_with(&cfg);
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    let mut out = vec![0.0f32; BLOCK * 2];

    // nothing fires it yet
    step(&mut set, &engine, &[], &mut out);
    assert_eq!(engine.voice_count(), 0, "no trigger, no horn");
    assert!(out.iter().all(|s| *s == 0.0));

    // the trigger starts it, and it is heard this block
    step(&mut set, &engine, &["ev_horn".to_string()], &mut out);
    assert_eq!(engine.voice_count(), 1, "the horn started");
    assert!(out.iter().any(|s| s.abs() > 1e-3), "the horn is audible");

    // the one-shot ends by itself after its clip
    for _ in 0..20 {
        step(&mut set, &engine, &[], &mut out);
    }
    assert_eq!(engine.voice_count(), 0, "the one-shot ended");
}

#[test]
fn a_loop_keeps_playing_and_follows_its_pitch_variable() {
    let cfg = cfg(include_str!("fixtures/soundcfg/loop.cfg"));
    let engine = engine_with(&cfg);
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    let var = |n: &str| match n {
        "fan_speed" => Some(1200.0),
        _ => Some(1.0), // fan_on
    };
    let mut out = vec![0.0f32; BLOCK * 2];
    engine.clock().advance(DT);
    set.update(&engine, &var, &glam::Mat4::IDENTITY, &[]);
    engine.render_offline(&mut out);

    let playing = set.playing(&engine);
    assert_eq!(playing.len(), 1, "the fan loop is on");
    // 1200 * 44100 / 600 = 88200 Hz against the 48000 Hz clip
    let pitch = playing[0].2;
    assert!((pitch - 1.8375).abs() < 1e-3, "pitch {pitch}");
    assert!(out.iter().any(|s| s.abs() > 1e-3), "the fan is audible");
}

#[test]
fn a_missing_file_plays_nothing_and_does_not_panic() {
    let cfg = cfg(include_str!("fixtures/soundcfg/trigger.cfg"));
    let engine = AudioEngine::new_offline(RATE, 2); // nothing cached: "horn.wav" is missing
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    let mut out = vec![0.0f32; BLOCK * 2];
    step(&mut set, &engine, &["ev_horn".to_string()], &mut out);
    assert_eq!(engine.voice_count(), 0);
    assert!(out.iter().all(|s| *s == 0.0));
}

fn events(names: &[&str]) -> Vec<SoundEvent> {
    names
        .iter()
        .enumerate()
        .map(|(i, n)| SoundEvent::trigger(EventSource::Player, i as u32, *n))
        .collect()
}

/// A trigger fired twice in one frame: both firings survive in the stream, but the entry
/// restarts once (the current, OPEN rule).
#[test]
fn two_identical_triggers_in_one_frame_start_one_voice() {
    let cfg = cfg(include_str!("fixtures/soundcfg/trigger.cfg"));
    let engine = engine_with(&cfg);
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    let mut out = vec![0.0f32; BLOCK * 2];
    engine.clock().advance(DT);
    set.update_events(
        &engine,
        &|_| Some(1.0),
        &glam::Mat4::IDENTITY,
        &events(&["ev_horn", "ev_horn"]),
        &|_| None,
    );
    engine.render_offline(&mut out);
    assert_eq!(engine.voice_count(), 1, "restarted once, not twice");
    assert_eq!(set.state(0), Some(SoundState::Running));
}

/// Two `[onlyone]` entries sharing one file: one firing starts one voice (central registry).
#[test]
fn onlyone_entries_share_a_single_voice() {
    let cfg = cfg(include_str!("fixtures/soundcfg/onlyone.cfg"));
    let engine = engine_with(&cfg);
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    let mut out = vec![0.0f32; BLOCK * 2];
    engine.clock().advance(DT);
    set.update_events(
        &engine,
        &|_| Some(1.0),
        &glam::Mat4::IDENTITY,
        &events(&["ev_beep"]),
        &|_| None,
    );
    engine.render_offline(&mut out);
    assert_eq!(engine.voice_count(), 1, "both entries share the file's voice");
}

/// A trigger accepted while its file is still loading starts when the clip arrives: an
/// asynchronous load must not swallow an accepted event.
#[test]
fn a_triggered_event_not_loaded_yet_keeps_its_start() {
    let cfg = cfg(include_str!("fixtures/soundcfg/trigger.cfg"));
    let engine = AudioEngine::new_offline(RATE, 2); // "horn.wav" not cached
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    let mut out = vec![0.0f32; BLOCK * 2];
    engine.clock().advance(DT);
    set.update_events(
        &engine,
        &|_| Some(1.0),
        &glam::Mat4::IDENTITY,
        &events(&["ev_horn"]),
        &|_| None,
    );
    engine.render_offline(&mut out);
    assert_eq!(engine.voice_count(), 0, "not loaded yet");
    assert_eq!(set.state(0), Some(SoundState::Loading));
    // the background loader delivers the clip
    for path in SoundSet::clip_paths(&cfg, Path::new("")) {
        engine.cache_clip(path, clip(0.25));
    }
    engine.clock().advance(DT);
    set.update_events(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &[], &|_| None);
    engine.render_offline(&mut out);
    assert_eq!(engine.voice_count(), 1, "the accepted event was not swallowed");
    assert_eq!(set.state(0), Some(SoundState::Running));
}

/// A `[3d]` sound to the listener's right: DirectSound's pan damps the far (left) channel,
/// it does not spread the level over both.
#[test]
fn a_sound_on_the_right_damps_the_left_channel() {
    let cfg = cfg(include_str!("fixtures/soundcfg/pan.cfg"));
    let engine = engine_with(&cfg);
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    let mut out = vec![0.0f32; BLOCK * 2];
    step(&mut set, &engine, &[], &mut out);
    let left: f32 = out.iter().step_by(2).map(|s| s * s).sum();
    let right: f32 = out.iter().skip(1).step_by(2).map(|s| s * s).sum();
    assert!(right > left * 100.0, "left {left}, right {right}");
}

/// A recording level over 1 is clamped only after the set master (OMSI's 0 dB buffer): the
/// set master 0.5 with a level 4.0 gives 1.0, not 0.5.
#[test]
fn a_loud_level_is_clamped_after_the_set_master() {
    let cfg = cfg(include_str!("fixtures/soundcfg/volume.cfg"));
    let engine = engine_with(&cfg);
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    set.master = 0.5;
    let mut out = vec![0.0f32; BLOCK * 2];
    step(&mut set, &engine, &[], &mut out);
    let played = set.playing(&engine);
    assert_eq!(played.len(), 1);
    assert!(
        (played[0].1 - 1.0).abs() < 0.02,
        "the product is clamped after the master, not cut early: {}",
        played[0].1
    );
}
