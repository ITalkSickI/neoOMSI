//! The OMSI sound runtime: the rules that decide *what* is heard, kept apart from the mixer
//! that decides *how* it sounds.
//!
//! Everything here works on a parsed [`omsi_vehicle::SoundEntry`] and script variable
//! values. The rules are free functions so they can be read and tested without a device,
//! a thread or a running sound set.

pub(crate) mod conditions;
pub(crate) mod event;
pub(crate) mod outside;
pub(crate) mod placement;
pub(crate) mod report;
pub(crate) mod sound;
pub mod soundset;

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
        outside::set_outside_open(None);
        assert_eq!(
            outside::outside_gain(1.0, true),
            1.0,
            "no variable: as before"
        );
        outside::set_outside_open(Some(0.0));
        assert_eq!(outside::outside_gain(1.0, true), 0.25, "shut: a quarter");
        assert_eq!(
            outside::outside_gain(1.0, false),
            1.0,
            "the own bus's sounds are its own"
        );
        assert_eq!(outside::lowpass_of(1.0, false), 0.0);
        assert_eq!(outside::outside_gain(0.0, true), 1.0, "standing outside");
        outside::set_outside_open(Some(0.5));
        assert_eq!(
            outside::outside_gain(1.0, true),
            1.0,
            "doors open: all of it"
        );
        assert!(outside::lowpass_of(1.0, true) > 5000.0);
        // (in the same test: the variable is one for all) the own bus's outside-only
        // entry, `[viewpoint] 5`, heard from the cab at `Snd_OutsideVol` (TSound update
        // 0x750340), not at all with everything shut
        let engine = SoundEntry {
            volume: 0.8,
            viewpoint: 5,
            ..Default::default()
        };
        let none = |_: &str| None;
        let vol = |def: &SoundEntry, view: i32| {
            conditions::volume(def, &none, view, 0.0, 1.0, outside::outside_open())
        };
        outside::set_outside_open(Some(0.0));
        assert_eq!(vol(&engine, 2), None);
        outside::set_outside_open(Some(0.5));
        assert_eq!(vol(&engine, 2), Some(0.4));
        assert_eq!(vol(&engine, 1), Some(0.8), "outside: as it is");
        let out_entry = SoundEntry {
            volume: 0.8,
            viewpoint: 1,
            ..Default::default()
        };
        assert_eq!(vol(&out_entry, 2), Some(0.4));
        assert_eq!(vol(&out_entry, 2 | 4), None, "not an AI bus's");
        let cab = SoundEntry {
            volume: 0.8,
            viewpoint: 2,
            ..Default::default()
        };
        assert_eq!(vol(&cab, 1), None, "a cab sound stays in");
        outside::set_outside_open(None);
        assert_eq!(vol(&engine, 2), None);
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
        outside::set_outside_open(None);
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
}
