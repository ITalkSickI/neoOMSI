//! Render a `sound.cfg` without an audio device and report what plays.
//!
//! A synthetic tone stands in for every file the configuration names, so the example runs
//! without any OMSI content. Time is a manual clock moved one block at a time, so a run is
//! reproducible: the same fixture and settings give the same samples.
//!
//! usage: offline_render [sound.cfg] [seconds] [out.wav]
//!
//! Without a `sound.cfg` a small demo (a running engine loop and a horn) is used.

use ::audio::{AudioEngine, Clip, SoundSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const RATE: u32 = 48_000;
const BLOCK: usize = 480; // 10 ms

const DEMO: &str = "[sound]\nengine.wav\n0.6\n\n[sound]\nhorn.wav\n1.0\n\n[trigger]\nev_horn\n";

fn main() {
    env_logger::init();
    let mut args = std::env::args().skip(1);
    let cfg_arg: Option<PathBuf> = args.next().map(PathBuf::from);
    let seconds: f32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let wav_out = args.next();

    let text = match &cfg_arg {
        Some(p) => std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display())),
        None => DEMO.to_string(),
    };
    let dir = cfg_arg
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let cfg = ::legacy_vehicle::SoundCfg::parse(&::legacy_config::CfgFile::from_str("sound.cfg", &text));

    let engine = AudioEngine::new_offline(RATE, 2);
    for path in SoundSet::clip_paths(&cfg, &dir) {
        engine.cache_clip(path, tone(RATE));
    }
    let mut set = SoundSet::new(&engine, &cfg, &dir);

    let frames = (seconds * RATE as f32 / BLOCK as f32) as usize;
    let horn_at = frames / 3;
    let mut out = vec![0.0f32; BLOCK * 2];
    let mut all = Vec::with_capacity(frames * BLOCK);
    let mut last: Vec<String> = Vec::new();

    for frame in 0..frames {
        engine
            .clock()
            .advance(Duration::from_secs_f64(BLOCK as f64 / RATE as f64));
        let triggers: Vec<String> = if frame == horn_at {
            vec!["ev_horn".to_string()]
        } else {
            Vec::new()
        };
        set.update(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &triggers);
        engine.render_offline(&mut out);
        all.extend_from_slice(&out);
        let playing: Vec<String> = set.playing(&engine).into_iter().map(|(f, ..)| f).collect();
        if playing != last {
            let t = frame as f32 * BLOCK as f32 / RATE as f32;
            println!("t={t:5.2}s playing {playing:?}");
            last = playing;
        }
    }

    let peak = all.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let rms = (all.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / all.len() as f64).sqrt();
    println!("{frames} blocks, peak {peak:.3}, rms {rms:.4}");
    if let Some(path) = wav_out {
        write_wav(Path::new(&path), &all, RATE, 2);
        println!("wrote {path}");
    }
}

/// A one-second, quiet tone standing in for a real sound file.
fn tone(rate: u32) -> Arc<Clip> {
    let n = rate as usize;
    let samples = (0..n)
        .map(|i| {
            let t = i as f32 / rate as f32;
            ((t * 220.0 * std::f32::consts::TAU).sin() * 0.35 * 32767.0) as i16
        })
        .collect();
    Arc::new(Clip {
        sample_rate: rate,
        channels: 1,
        samples,
    })
}

/// Minimal 16-bit PCM WAV writer, interleaved `channels`.
fn write_wav(path: &Path, samples: &[f32], rate: u32, channels: u16) {
    let data: Vec<u8> = samples
        .iter()
        .flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
        .collect();
    let mut b = Vec::with_capacity(44 + data.len());
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&((36 + data.len()) as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend_from_slice(&data);
    let _ = std::fs::write(path, b);
}
