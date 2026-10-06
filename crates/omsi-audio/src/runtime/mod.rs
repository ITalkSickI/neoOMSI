//! The OMSI sound runtime: the rules that decide *what* is heard, kept apart from the mixer
//! that decides *how* it sounds.
//!
//! Everything here works on a parsed [`omsi_vehicle::SoundEntry`] and script variable
//! values. The rules are free functions so they can be read and tested without a device,
//! a thread or a running sound set.

pub(crate) mod conditions;
pub(crate) mod event;
pub(crate) mod level;
pub(crate) mod outside;
pub(crate) mod placement;
pub(crate) mod report;
pub(crate) mod sound;
pub mod soundset;
mod files;
mod parts;
mod cabin;

#[cfg(test)]
mod tests {
    use super::event::{EventSource, SoundEvent};
    use super::sound::SoundState;
    use super::soundset::SoundSet;
    use super::{conditions, outside};
    use omsi_vehicle::{SoundCfg, SoundEntry, VolCurve};
    use std::path::Path;
    use std::sync::Arc;

    fn quiet_clip() -> Arc<crate::assets::Clip> {
        Arc::new(crate::assets::Clip {
            sample_rate: 48_000,
            channels: 1,
            samples: vec![16_384; 4_800],
        })
    }

    fn engine_with(cfg: &SoundCfg) -> crate::engine::AudioEngine {
        let engine = crate::engine::AudioEngine::new_offline(48_000, 2);
        for path in SoundSet::clip_paths(cfg, Path::new("")) {
            engine.cache_clip(path, quiet_clip());
        }
        engine
    }

    fn triggered(file: &str) -> SoundCfg {
        SoundCfg {
            sounds: vec![SoundEntry {
                file: file.into(),
                volume: 1.0,
                triggers: vec!["ev_horn".into()],
                ..Default::default()
            }],
            unknown_keywords: Vec::new(),
        }
    }

    #[test]
    fn open_doors_let_the_outside_in() {
        // Pure inputs keep this fixture independent of other sets/tests using the global.
        assert_eq!(outside::outside_gain_at(1.0, true, None), 1.0);
        assert_eq!(outside::outside_gain_at(1.0, true, Some(0.0)), 0.25);
        assert_eq!(outside::outside_gain_at(1.0, false, Some(0.0)), 1.0);
        assert_eq!(outside::lowpass_of(1.0, false, Some(0.0)), 0.0);
        assert_eq!(outside::outside_gain_at(0.0, true, Some(0.0)), 1.0);
        assert_eq!(outside::outside_gain_at(1.0, true, Some(0.5)), 1.0);
        assert!(outside::lowpass_of(1.0, true, Some(0.5)) > 5000.0);
        let engine = SoundEntry { volume: 0.8, viewpoint: 5, ..Default::default() };
        let none = |_: &str| None;
        let vol = |def: &SoundEntry, view: i32, opening| {
            conditions::volume(def, &none, view, 0.0, 1.0, opening)
        };
        assert_eq!(vol(&engine, 2, Some(0.0)), None);
        assert_eq!(vol(&engine, 2, Some(0.5)), Some(0.4));
        assert_eq!(vol(&engine, 1, Some(0.5)), Some(0.8));
        let out_entry = SoundEntry { volume: 0.8, viewpoint: 1, ..Default::default() };
        assert_eq!(vol(&out_entry, 2, Some(0.5)), Some(0.4));
        assert_eq!(vol(&out_entry, 2 | 4, Some(0.5)), None);
        let cab = SoundEntry { volume: 0.8, viewpoint: 2, ..Default::default() };
        assert_eq!(vol(&cab, 1, Some(0.5)), None);
        assert_eq!(vol(&engine, 2, None), None);
    }

    /// The SD200's door hit: `[volcurve] doorSpeed_0` from 1 at -1 down to 0 at -0.5,
    /// fired while the door still closes at -1 m/s; by the frame's end the script has turned
    /// the speed round. The volume is the one of the moment it fired (#676).
    #[test]
    fn a_triggered_sound_reads_its_curve_when_it_fires() {
        let hit = SoundEntry {
            volume: 1.0,
            triggers: vec!["ev_doorhitclose_0".into()],
            vol_curves: vec![omsi_vehicle::VolCurve {
                variable: "doorSpeed_0".into(),
                points: vec![(-1.0, 1.0), (-0.5, 0.0)],
            }],
            ..Default::default()
        };
        let now = |n: &str| (n == "doorSpeed_0").then_some(0.8);
        let at_fire =
            |t: &str, n: &str| (t == "ev_doorhitclose_0" && n == "doorSpeed_0").then_some(-1.0);
        assert_eq!(
            conditions::volume(&hit, &now, 2, 0.0, 1.0, None),
            Some(0.0),
            "read at the frame's end: silent"
        );
        let fired = |n: &str| at_fire("ev_doorhitclose_0", n).or_else(|| now(n));
        assert_eq!(conditions::volume(&hit, &fired, 2, 0.0, 1.0, None), Some(1.0));
        let cfg = SoundCfg {
            sounds: vec![hit],
            unknown_keywords: Vec::new(),
        };
        let engine = crate::engine::AudioEngine::new_offline(48_000, 2);
        let set = SoundSet::new(&engine, &cfg, std::path::Path::new(""));
        assert_eq!(set.curve_triggers(), vec!["ev_doorhitclose_0".to_string()]);
    }

    /// The ordered stream keeps both firings, but an entry restarts only once a frame -
    /// the current (OPEN) rule, preserved by `update_events`.
    #[test]
    fn a_repeated_trigger_restarts_once_but_both_events_survive() {
        let cfg = triggered("horn.wav");
        let engine = engine_with(&cfg);
        let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
        let events = vec![
            SoundEvent::trigger(EventSource::Player, 0, "ev_horn"),
            SoundEvent::trigger(EventSource::Player, 1, "ev_horn"),
        ];
        set.update_events(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &events, &|_| None);
        assert_eq!(engine.voice_count(), 1, "restarted once, not twice");
        assert_eq!(set.state(0), Some(SoundState::Running));
    }

    /// A trigger accepted while the clip is still loading is not swallowed: it starts when
    /// the asset arrives (the asynchronous-load case).
    #[test]
    fn an_accepted_event_survives_an_asynchronous_load() {
        let cfg = triggered("horn.wav");
        let engine = crate::engine::AudioEngine::new_offline(48_000, 2); // nothing cached
        let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
        let events = vec![SoundEvent::trigger(EventSource::Player, 0, "ev_horn")];
        set.update_events(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &events, &|_| None);
        assert_eq!(engine.voice_count(), 0, "the file is not there yet");
        assert_eq!(set.state(0), Some(SoundState::Loading));
        // the background loader delivers the clip afterwards
        for path in SoundSet::clip_paths(&cfg, Path::new("")) {
            engine.cache_clip(path, quiet_clip());
        }
        set.update_events(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &[], &|_| None);
        assert_eq!(engine.voice_count(), 1, "the accepted event was not swallowed");
        assert_eq!(set.state(0), Some(SoundState::Running));
    }

    /// Two entries sharing a file, both `[onlyone]`: one firing starts a single voice - the
    /// central, file-related registry the reference uses (scope OPEN).
    #[test]
    fn onlyone_is_central_per_file() {
        let cfg = SoundCfg {
            sounds: vec![
                SoundEntry {
                    file: "beep.wav".into(),
                    volume: 1.0,
                    triggers: vec!["ev_beep".into()],
                    only_one: true,
                    ..Default::default()
                },
                SoundEntry {
                    file: "beep.wav".into(),
                    volume: 1.0,
                    triggers: vec!["ev_beep".into()],
                    only_one: true,
                    ..Default::default()
                },
            ],
            unknown_keywords: Vec::new(),
        };
        let engine = engine_with(&cfg);
        let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
        let events = vec![SoundEvent::trigger(EventSource::Player, 0, "ev_beep")];
        set.update_events(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &events, &|_| None);
        assert_eq!(engine.voice_count(), 1, "the second entry reuses the running voice");
    }

    /// `[viewpoint]` gating in the update path: an outside-only entry (`[viewpoint] 1`) is
    /// *not* cut by the listener sitting in the cab - `volume_side` builds the view from the
    /// entry's own bits. This locks the current (OPEN) behavior.
    #[test]
    fn viewpoint_gating_is_taken_from_the_entry_not_the_listener() {
        let cfg = SoundCfg {
            sounds: vec![SoundEntry {
                file: "rain.wav".into(),
                volume: 1.0,
                viewpoint: 1,
                ..Default::default()
            }],
            unknown_keywords: Vec::new(),
        };
        let engine = engine_with(&cfg);
        let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
        set.set_inside(true); // the listener is in the cab, the entry says "outside"
        set.update_events(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &[], &|_| None);
        assert_eq!(engine.voice_count(), 1, "the entry's own view bit is used (OPEN)");
    }

    /// A triggered entry reads its curve from the event's fire-time snapshot through the
    /// slot resolver, not from the frame-end variables.
    #[test]
    fn a_triggered_event_reads_its_fire_time_snapshot() {
        let cfg = SoundCfg {
            sounds: vec![SoundEntry {
                file: "hit.wav".into(),
                volume: 1.0,
                triggers: vec!["ev_doorhitclose_0".into()],
                vol_curves: vec![VolCurve {
                    variable: "doorSpeed_0".into(),
                    points: vec![(-1.0, 1.0), (-0.5, 0.0)],
                }],
                ..Default::default()
            }],
            unknown_keywords: Vec::new(),
        };
        let engine = engine_with(&cfg);
        let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
        // the door speed has turned round by the frame's end; the snapshot has -1
        let now = |n: &str| (n == "doorSpeed_0").then_some(0.8);
        let slots = |n: &str| (n == "doorSpeed_0").then_some(0);
        let events = vec![
            SoundEvent::trigger(EventSource::Player, 0, "ev_doorhitclose_0")
                .with_vars(vec![-1.0]),
        ];
        set.update_events(&engine, &now, &glam::Mat4::IDENTITY, &events, &slots);
        assert_eq!(engine.voice_count(), 1, "started at the fire-time volume");
    }

    /// An exterior (AI / other player) sound set heard from the player's cabin is damped by
    /// the player's bodywork: the wired `outside_gain` transmission, separate from the
    /// own bus's `Snd_OutsideVol` path.
    #[test]
    fn an_exterior_set_is_damped_by_the_bodywork() {
        let cfg = SoundCfg {
            sounds: vec![SoundEntry {
                file: "amb.wav".into(),
                volume: 1.0,
                ..Default::default()
            }],
            unknown_keywords: Vec::new(),
        };
        outside::set_outside_open(Some(0.0));
        let engine = engine_with(&cfg);
        let mut set = SoundSet::new_exterior(&engine, &cfg, Path::new(""));
        set.set_muffled(true);
        // let the inside/outside blend reach its target (it is eased over ~0.45 s)
        for _ in 0..6 {
            engine
                .clock()
                .advance(std::time::Duration::from_millis(100));
            set.update(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &[]);
        }
        let played = set.playing(&engine);
        assert_eq!(played.len(), 1);
        assert!(
            (played[0].1 - 0.25).abs() < 0.02,
            "a shut bodywork keeps a quarter: {}",
            played[0].1
        );
        outside::set_outside_open(None);
    }

    /// The runtime's admission budget is separate from the mixer's `MAX_VOICES`: a set with
    /// more entries than `[sound_maxcount]` starts at most that many.
    #[test]
    fn the_runtime_admits_at_most_sound_maxcount() {
        use super::level::SOUND_MAXCOUNT;
        let sounds: Vec<SoundEntry> = (0..(SOUND_MAXCOUNT + 20))
            .map(|i| SoundEntry {
                file: format!("s{i}.wav"),
                volume: 1.0,
                ..Default::default()
            })
            .collect();
        let cfg = SoundCfg {
            sounds,
            unknown_keywords: Vec::new(),
        };
        let engine = engine_with(&cfg);
        let mut set = SoundSet::new(&engine, &cfg, Path::new(""));
        set.update(&engine, &|_| Some(1.0), &glam::Mat4::IDENTITY, &[]);
        assert_eq!(
            engine.voice_count(),
            SOUND_MAXCOUNT,
            "the rest wait for a slot (their type decides the resume rule)"
        );
    }
}

#[cfg(test)]
mod listening_tests;
