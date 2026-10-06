//! The OMSI sound runtime: the rules that decide *what* is heard, kept apart from the mixer
//! that decides *how* it sounds.
//!
//! Everything here works on a parsed [`omsi_vehicle::SoundEntry`] and script variable
//! values. The rules are free functions so they can be read and tested without a device,
//! a thread or a running sound set.

pub(crate) mod conditions;
pub(crate) mod outside;
pub(crate) mod placement;
pub(crate) mod report;
pub(crate) mod sound;
pub mod soundset;

#[cfg(test)]
mod tests {
    use super::outside;
    use super::soundset::SoundSet;
    use super::conditions;
    use omsi_vehicle::{SoundCfg, SoundEntry};

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
}
