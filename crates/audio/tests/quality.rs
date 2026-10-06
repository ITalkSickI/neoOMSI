//! End-to-end quality fixtures through the public offline/device-equivalent renderer.
use ::audio::{AudioEngine, Bus, Clip, VoiceParams};
use std::sync::Arc;
fn clip() -> Arc<Clip> { Arc::new(Clip { sample_rate: 48000, channels: 2, samples: vec![8192; 9600] }) }
fn steady(engine: &AudioEngine, channels: usize) -> Vec<f32> {
    let mut out = vec![0.0; 4800 * channels]; engine.render_offline(&mut out); out
}
#[test]
fn each_bus_can_be_muted_independently_and_raw_updates_keep_routing() {
    for bus in [Bus::Vehicle, Bus::Ambience, Bus::Passenger, Bus::Announcement, Bus::Radio, Bus::Interface] {
        let engine = AudioEngine::new_offline(48000, 2);
        let params = VoiceParams { looping: true, ..Default::default() };
        let id = engine.play_on_bus(clip(), params, bus);
        assert!(steady(&engine, 2).last().unwrap() > &0.05);
        engine.set_bus_gain(bus, 0.0); engine.set_params(id, params);
        steady(&engine, 2); let muted = steady(&engine, 2);
        assert!(muted.last().unwrap().abs() < 0.0001, "{bus:?}");
        assert!(engine.is_playing(id), "bus mute must not stop the runtime voice");
        let other = if bus == Bus::Vehicle { Bus::Passenger } else { Bus::Vehicle };
        engine.play_on_bus(clip(), params, other);
        assert!(steady(&engine, 2).last().unwrap() > &0.05);
    }
}
#[test]
fn stereo_mono_and_surround_front_channels_share_the_renderer() {
    let params = VoiceParams { looping: true, ..Default::default() };
    let mono = AudioEngine::new_offline(48000, 1); mono.play(clip(), params);
    let stereo = AudioEngine::new_offline(48000, 2); stereo.play(clip(), params);
    let surround = AudioEngine::new_offline(48000, 6); surround.play(clip(), params);
    let m = steady(&mono, 1); let s = steady(&stereo, 2); let six = steady(&surround, 6);
    for i in 0..4800 {
        assert_eq!(m[i], (s[i*2] + s[i*2+1]) * 0.5);
        assert_eq!(&six[i*6..i*6+2], &s[i*2..i*2+2]);
        assert!(six[i*6+2..i*6+6].iter().all(|s| *s == 0.0));
    }
}
#[test]
fn dense_traffic_is_finite_bounded_and_virtualized_voices_keep_running() {
    let engine = AudioEngine::new_offline(48000, 2);
    for _ in 0..250 { engine.play(clip(), VoiceParams { looping: true, ..Default::default() }); }
    let out = steady(&engine, 2);
    assert!(out.iter().all(|s| s.is_finite() && s.abs() <= 0.900001));
    assert_eq!(engine.voice_count(), 250);
    assert!(engine.stats().clean());
}
#[test]
fn offline_output_is_reproducible_and_empty_blocks_are_valid() {
    let render = || {
        let engine = AudioEngine::new_offline(44100, 2);
        engine.play(clip(), VoiceParams { pitch: 1.37, looping: true, ..Default::default() });
        engine.render_offline(&mut []); steady(&engine, 2)
    };
    assert_eq!(render(), render());
}

#[test]
fn radio_uses_the_same_fades_and_a_closed_buffer_has_a_short_stop_tail() {
    let engine = AudioEngine::new_offline(48000, 2);
    let stream = Arc::new(::audio::stream::StreamBuf::default());
    stream.push(48000, std::iter::repeat_n([0.5; 2], 72000));
    engine.play_stream_on_bus(stream.clone(), VoiceParams::default(), Bus::Radio);
    let playing = steady(&engine, 2);
    assert!(playing[0].abs() < 0.001 && playing.last().unwrap() > &0.17);
    stream.close();
    let mut tail = [0.0; 512]; engine.render_offline(&mut tail);
    assert!(tail[0] > 0.15 && tail[400..].iter().all(|s| *s == 0.0));
    assert_eq!(engine.voice_count(), 0);
}
