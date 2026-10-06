//! Reproducible synthetic workload, timed OUTSIDE the renderer. Prints CSV for comparison
//! on the same release build/machine. Not a substitute for live map CPU/memory/hearing QA.
//! cargo run --release -p omsi-audio --example audio_load -- 200
use omsi_audio::{AudioEngine, Bus, Clip, Listener, VoiceParams};
use std::{sync::Arc, time::{Duration, Instant}};
const RATE: u32 = 48000;
const FRAMES: usize = 256;
fn main() {
    let count: usize = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(200);
    assert!(count <= 512);
    let engine = AudioEngine::new_offline(RATE, 2);
    let clip = Arc::new(Clip { sample_rate: 44100, channels: 1,
        samples: (0..44100).map(|i| ((i as f32 * 220.0 * std::f32::consts::TAU / 44100.0).sin() * 12000.0) as i16).collect() });
    let mut ids = Vec::new();
    for i in 0..count {
        let params = VoiceParams { gain: 0.1, pitch: 0.5 + (i % 9) as f32 * 0.25,
            looping: true, position: Some(glam::Vec3::new((i % 15) as f32, (i / 15) as f32, 0.0)),
            lowpass_hz: if i % 3 == 0 { 700.0 } else { 0.0 }, ..Default::default() };
        ids.push((engine.play(clip.clone(), params), params));
    }
    // Environment, passenger speech and radio share the same headroom in a dense street.
    for bus in [Bus::Ambience, Bus::Passenger, Bus::Announcement, Bus::Radio, Bus::Interface] {
        let mut mix = omsi_audio::voice::MixParams::from(VoiceParams {
            gain: 0.02, looping: true, ..Default::default() });
        mix.bus = bus;
        if bus == Bus::Announcement { mix.cabin_reverb = 0.25; }
        engine.play_mix(clip.clone(), mix);
    }
    engine.set_listener(Listener { reverb_time: 0.8, reverb_mix: 0.2, ..Default::default() });
    let mut out = [0.0; FRAMES * 2];
    for _ in 0..32 { engine.render_offline(&mut out); }
    let mut times = Vec::new();
    let mut total = Duration::ZERO;
    let mut peak = 0.0f32;
    for block in 0..375 {
        engine.clock().advance(Duration::from_secs_f64(FRAMES as f64 / RATE as f64));
        if block % 10 == 0 {
            for (id, params) in &ids {
                let mut p = *params;
                p.pitch *= 1.0 + 0.2 * (block as f32 * 0.03).sin();
                engine.set_params(*id, p);
            }
        }
        let start = Instant::now(); engine.render_offline(&mut out); let elapsed = start.elapsed();
        total += elapsed; times.push(elapsed.as_secs_f64() * 1000.0);
        peak = out.iter().map(|s| s.abs()).fold(peak, f32::max);
    }
    times.sort_by(f64::total_cmp);
    let budget_ms = FRAMES as f64 * 1000.0 / RATE as f64;
    println!("voices,active,mean_ms,p95_ms,max_ms,callback_budget_ms,render_budget_percent,shared_clip_bytes,peak,dropped,underruns");
    let stats = engine.stats();
    println!("{count},{},{:.4},{:.4},{:.4},{budget_ms:.4},{:.2},{},{peak:.6},{},{}",
        engine.voice_count(), total.as_secs_f64()*1000.0/375.0, times[356], times[374],
        total.as_secs_f64()/2.0*100.0, clip.samples.len()*2, stats.dropped_commands, stats.stream_underruns);
}
