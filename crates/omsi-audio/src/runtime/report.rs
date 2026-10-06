//! Read-back from a sound set: what is playing, how many entries there are, which triggers
//! need their variable values at fire time, and the `OMSI_DEBUG_SOUND` report.

use crate::engine::Playback;
use crate::runtime::conditions;
use crate::runtime::outside;
use crate::runtime::placement;
use crate::runtime::soundset::SoundSet;

impl SoundSet {
    /// How many `[sound]`/`[loopsound]` entries the configuration has.
    pub fn len(&self) -> usize {
        self.sounds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// The triggers (lower case) whose entries read script variables in a volume curve:
    /// what [`SoundSet::update_fired`] wants the values of at the moment they fire.
    pub fn curve_triggers(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for s in &self.sounds {
            if !s
                .def
                .vol_curves
                .iter()
                .any(|vc| vc.variable.trim().parse::<i32>().is_err())
            {
                continue;
            }
            for t in &s.def.triggers {
                let t = t.trim().to_ascii_lowercase();
                if !out.contains(&t) {
                    out.push(t);
                }
            }
        }
        out
    }

    /// The sounds playing right now: (file, gain at the listener, pitch) - for the logs.
    pub fn playing(&self, engine: &dyn Playback) -> Vec<(String, f32, f32)> {
        self.sounds
            .iter()
            .filter_map(|s| {
                s.voice
                    .and_then(|id| engine.voice_state(id))
                    .map(|(p, heard)| (s.def.file.clone(), heard, p.pitch))
            })
            .collect()
    }

    /// One line per entry of the configuration, saying whether it is heard and why not
    /// (`OMSI_DEBUG_SOUND`): the only way to see which of a bus's hundred sounds the
    /// rewrite never reaches - a missing clip, a condition on a variable nobody feeds, a
    /// volume curve that stays at zero or a `[viewpoint]` the camera is not in.
    pub fn report(&self, engine: &dyn Playback, var: &dyn Fn(&str) -> Option<f32>) -> Vec<String> {
        let view = placement::view_mask(self.inside, self.ai);
        let now = self.clock.now();
        let mut out = Vec::new();
        for s in &self.sounds {
            let file = s.def.file.trim();
            let mut why = String::new();
            if s.clip.is_none() {
                why = if file.parse::<i32>().is_ok() {
                    "waits for a (T.F.) file".into()
                } else {
                    "no clip".into()
                };
            } else if s.def.viewpoint != 0
                && s.def.viewpoint & view == 0
                && !(view == 2
                && s.def.viewpoint & 2 == 0
                && outside::outside_open().is_some_and(|o| o > 0.01))
            {
                why = format!("viewpoint {} (listener {view})", s.def.viewpoint);
            } else if let Some(c) = s
                .def
                .conditions
                .iter()
                .find(|c| !c.holds(var(&c.variable).unwrap_or(0.0)))
            {
                why = format!("condition {} = {:?}", c.variable, var(&c.variable));
            } else if let Some(vc) = s.def.vol_curves.iter().find(|vc| {
                let active = s
                    .active_since
                    .map_or(0.0, |t| now.saturating_duration_since(t).as_secs_f32());
                conditions::curve_input(vc, var, active, 1.0)
                    .is_some_and(|x| conditions::curve(&vc.points, x) <= 0.001)
            }) {
                why = format!("volcurve {} = {:?}", vc.variable, var(&vc.variable));
            }
            let playing = s.voice.and_then(|id| engine.voice_state(id));
            match (playing, why.is_empty()) {
                (Some((p, heard)), _) => {
                    out.push(format!("{file}: {:.3} heard, pitch {:.2}", heard, p.pitch))
                }
                (None, true) => out.push(format!("{file}: ready, silent")),
                (None, false) => out.push(format!("{file}: off - {why}")),
            }
        }
        out
    }
}
