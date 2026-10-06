use super::*;
    use crate::engine::commands::CommandQueue;
    use crate::engine::feedback::{Counters, Reaper};
    use crate::voice::VoiceId;
    use std::sync::Arc;

    fn core() -> AudioCore {
        let counters = Arc::new(Counters::default());
        AudioCore::new(
            Clock::real(),
            Arc::new(OutputFormat::new(48_000, 1)),
            Arc::new(CommandQueue::new(counters.clone())),
            Arc::new(Reaper::new()),
            counters,
            false,
        )
    }

    fn voice(clip: Arc<Clip>, gain: f32) -> Voice {
        Voice::test_voice(
            1,
            clip,
            VoiceParams {
                gain,
                pitch: 1.0,
                looping: true,
                position: None,
                doppler: true,
                range: 10.0,
                lowpass_hz: 0.0,
                important: false,
                pan: 1.0,
            },
        )
    }

    #[test]
    fn a_loop_has_no_gap_where_it_turns() {
        // a 5-frame loop of a constant level at the device rate: every output frame carries it
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![16_384; 5],
        });
        let mut s = core();
        s.voices.push(voice(clip, 1.0));
        let mut out = vec![0.0f32; 64];
        s.render(&mut out);
        assert!(out.iter().all(|x| (*x - 0.5 * HEADROOM).abs() < 1e-3), "{out:?}");
    }

    #[test]
    fn parameters_arrive_with_the_next_block() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![16_384; 5],
        });
        let mut s = core();
        s.voices.push(voice(clip, 1.0));
        s.queue.push(Command::SetParams {
            id: 1,
            params: VoiceParams {
                gain: 0.0,
                looping: true,
                ..Default::default()
            }
            .into(),
            at: Instant::now(),
        });
        let mut out = vec![0.0f32; 4];
        s.render(&mut out);
        assert_eq!(s.voices[0].params().level.gain(), 0.0);
    }

    #[test]
    fn a_play_then_parameters_takes_effect_in_order() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![16_384; 5],
        });
        let mut s = core();
        s.queue.push(Command::Play {
            id: 5,
            clip,
            params: VoiceParams {
                gain: 1.0,
                looping: true,
                ..Default::default()
            }
            .into(),
        });
        s.queue.push(Command::SetParams {
            id: 5,
            params: VoiceParams {
                gain: 0.25,
                looping: true,
                ..Default::default()
            }
            .into(),
            at: Instant::now(),
        });
        let mut out = vec![0.0f32; 4];
        s.render(&mut out);
        assert_eq!(s.voices.len(), 1);
        assert_eq!(
            s.voices[0].params().level.gain(),
            0.25,
            "the parameters followed the start"
        );
    }

    #[test]
    fn a_finished_voice_is_retired_to_the_game_thread() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![16_384; 5],
        });
        let mut s = core();
        s.voices.push(voice(clip, 1.0));
        s.queue.push(Command::Stop { id: 1 });
        let mut out = vec![0.0f32; 256];
        s.render(&mut out);
        assert!(s.voices.is_empty(), "the stopped voice left the mixer");
        let retired = s.reaper.drain();
        assert_eq!(retired, vec![1 as VoiceId]);
    }

    #[test]
    fn important_voices_win_the_mixer_limit() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![64; 100],
        });
        let mut s = core();
        for _ in 0..MAX_VOICES {
            s.voices.push(voice(clip.clone(), 1.0));
        }
        let quiet_important = Voice::test_voice(
            9_999,
            clip,
            VoiceParams {
                gain: 0.001,
                pitch: 1.0,
                looping: true,
                position: None,
                doppler: true,
                range: 10.0,
                lowpass_hz: 0.0,
                important: true,
                pan: 1.0,
            },
        );
        s.voices.push(quiet_important);
        let mut out = vec![0.0f32; 16];
        s.render(&mut out);
        let expect = ((MAX_VOICES - 1) as f32 + 0.001) * 64.0 / 32_768.0 * HEADROOM;
        assert!((out[0] - expect).abs() < 1e-4, "{} vs {expect}", out[0]);
    }

    #[test]
    fn only_the_loudest_voices_are_mixed() {
        let clip = Arc::new(Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![64; 100],
        });
        let mut s = core();
        for k in 0..MAX_VOICES + 50 {
            s.voices
                .push(voice(clip.clone(), if k < 50 { 0.001 } else { 1.0 }));
        }
        let mut out = vec![0.0f32; 16];
        s.render(&mut out);
        // the 50 quiet ones stayed out: exactly MAX_VOICES at full gain
        let expect = MAX_VOICES as f32 * 64.0 / 32_768.0 * HEADROOM;
        assert!((out[0] - expect).abs() < 1e-3, "{} vs {expect}", out[0]);
        assert_eq!(s.voices.len(), MAX_VOICES + 50);
    }

#[test]
fn starts_are_bounded_and_assets_are_freed_on_the_game_side() {
    let mut s = core();
    let clip = Arc::new(Clip { sample_rate: 48000, channels: 1, samples: vec![0; 10] });
    let weak = Arc::downgrade(&clip);
    for id in 0..VOICE_CAPACITY + 10 {
        s.queue.push(Command::Play { id: id as u64, clip: clip.clone(),
            params: VoiceParams { looping: true, ..Default::default() }.into() });
    }
    drop(clip);
    s.render(&mut [0.0; 256]);
    assert_eq!(s.voices.len(), VOICE_CAPACITY);
    assert_eq!(s.voices.capacity(), VOICE_CAPACITY);
    assert_eq!(s.counters.dropped_commands.load(Ordering::Relaxed), 10);
    for voice in &mut s.voices { voice.finish(); }
    s.render(&mut [0.0; 256]);
    assert!(s.voices.is_empty());
    assert!(weak.upgrade().is_some(), "the callback retained assets in the reaper");
    s.reaper.drain();
    assert!(weak.upgrade().is_none(), "game-side drain released the final asset");
}

#[test]
fn a_busy_reaper_retains_rejected_starts_without_replaying_them() {
    let mut s = core();
    let clip = Arc::new(Clip { sample_rate: 48000, channels: 1, samples: vec![0; 10] });
    // Saturate the return path deterministically, then exceed the voice list.
    for id in 0..crate::engine::feedback::REAPER_CAPACITY {
        s.reaper.reject(Command::Play { id: (10000 + id) as u64, clip: clip.clone(), params: Default::default() }).ok().unwrap();
    }
    for id in 0..VOICE_CAPACITY + 1 {
        s.queue.push(Command::Play { id: id as u64, clip: clip.clone(),
            params: VoiceParams { looping: true, ..Default::default() }.into() });
    }
    s.render(&mut [0.0; 16]); assert_eq!(s.rejected.len(), 1);
    s.reaper.drain();
    s.render(&mut [0.0; 16]);
    assert_eq!(s.voices.len(), VOICE_CAPACITY);
    assert!(s.rejected.is_empty());
    assert_eq!(s.reaper.drain(), [VOICE_CAPACITY as u64]);
}

#[test]
fn button_transients_have_no_implicit_cabin_reverb_tail() {
    let mut s = core();
    s.queue.push(Command::SetCabin { h: 1.0, at: s.clock.now() });
    let clip = Arc::new(Clip { sample_rate: 48000, channels: 1, samples: vec![12000; 240] });
    s.queue.push(Command::Play { id: 7, clip, params: Default::default() });
    let mut out = vec![0.0; 10000]; s.render(&mut out);
    assert!(out[..240].iter().any(|s| *s > 0.01));
    assert!(out[240..].iter().all(|s| *s == 0.0), "a switch must not excite invented cabin echo");
}
