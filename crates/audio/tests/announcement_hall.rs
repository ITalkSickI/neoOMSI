//! PA-only hall must create a tail, not just turn up the direct recording.
use ::audio::{AudioEngine, Bus, Clip, EventSource, Listener, SoundEvent, SoundSet};
use ::legacy_vehicle::{SoundCfg, SoundEntry};
use std::{path::Path, sync::Arc};

fn setup(inside: bool, mix: f32) -> (AudioEngine, SoundSet) {
    let engine = AudioEngine::new_offline(48000, 2);
    engine.cache_clip("pa.wav", Arc::new(Clip {
        sample_rate: 48000, channels: 1, samples: vec![12000; 480] }));
    let cfg = SoundCfg { sounds: vec![SoundEntry { file: "0".into(), volume: 1.0,
        triggers: vec!["announce".into()], ..Default::default() }], unknown_keywords: vec![] };
    let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
    set.set_inside(inside); set.set_announcement_reverb(mix);
    let events = [SoundEvent::file(EventSource::Player, 0, "announce", "pa.wav")];
    set.update_events(&engine, &|name| (name == "Snd_OutsideVol").then_some(0.0),
        &glam::Mat4::IDENTITY, &events, &|_| None);
    (engine, set)
}

fn render(engine: &AudioEngine, samples: usize) -> Vec<f32> {
    let mut out = vec![0.0; samples]; engine.render_offline(&mut out); out
}

#[test]
fn interior_hall_reduces_the_direct_part_and_continues_after_the_voice_ends() {
    let (wet_engine, _wet_set) = setup(true, 0.25);
    let (dry_engine, _dry_set) = setup(true, 0.0);
    let wet = render(&wet_engine, 48000); let dry = render(&dry_engine, 48000);
    assert!(wet[400] < dry[400], "hall must not be a gain boost to the direct path");
    assert!(dry[960..].iter().all(|x| *x == 0.0));
    assert!(wet[3000..8000].iter().any(|x| x.abs() > 0.0001), "missing delayed PA room tail");
    assert!(wet[40000..].iter().all(|x| x.abs() < 0.0001), "short cabin hall must decay");
    assert_eq!(wet_engine.voice_count(), 0, "tail must not prolong source voice lifetime");
    assert!(wet_engine.stats().clean());
}

#[test]
fn outside_pa_does_not_get_an_interior_hall_tail() {
    let (engine, _set) = setup(false, 0.25);
    let out = render(&engine, 12000);
    assert!(out[..960].iter().any(|x| x.abs() > 0.001));
    // The closed-body low-pass has a tiny filter tail within the last source frames,
    // not a new room return after the source retires.
    assert!(out[960..].iter().all(|x| *x == 0.0));
}

#[test]
fn pa_return_respects_announcement_bus_mute_and_pause() {
    for pause in [false, true] {
        let (engine, _set) = setup(true, 0.25);
        render(&engine, 4000); // excite hall and finish source
        if pause { engine.set_listener(Listener { master: 0.0, ..Default::default() }); }
        else { engine.set_bus_gain(Bus::Announcement, 0.0); }
        let out = render(&engine, 24000);
        assert!(out[20000..].iter().all(|x| x.abs() < 1e-8));
    }
}

#[test]
fn pa_hall_output_is_independent_of_callback_partition() {
    let (whole_engine, _set) = setup(true, 0.25);
    let (parts_engine, _set) = setup(true, 0.25);
    let whole = render(&whole_engine, 24000);
    let mut parts = vec![0.0; 24000];
    for block in parts.chunks_mut(128) { parts_engine.render_offline(block); }
    assert_eq!(whole, parts);
}
