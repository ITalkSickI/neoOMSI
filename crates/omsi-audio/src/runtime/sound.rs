//! One entry of a sound configuration: its loaded clip, its running voice and the per-frame
//! decision of how it plays. The pure rules it calls live in [`crate::runtime::conditions`]
//! and [`crate::runtime::placement`]; the engine only starts and stops voices.

use crate::assets::Clip;
use crate::engine::Playback;
use crate::runtime::{conditions, outside, placement};
use crate::voice::{VoiceId, VoiceParams};
use glam::{Mat4, Vec3};
use omsi_vehicle::SoundEntry;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

pub struct RuntimeSound {
    pub(super) def: SoundEntry,
    pub(super) clip: Option<Arc<Clip>>,
    pub(super) voice: Option<VoiceId>,
    /// The conditions held last frame (a `[noloop]` entry without a trigger plays once
    /// when they start to hold).
    pub(super) held: bool,
    /// Since when the conditions hold (triggered entries: since the trigger last fired) -
    /// what a `[volcurve] -1` reads, see [`crate::runtime::conditions::curve_input`].
    pub(super) active_since: Option<Instant>,
    /// The loudest a triggered entry has been since its trigger fired: Omsi.exe never lets
    /// such a sound get quieter while it plays (`TSound` +0x2c, reset when the trigger fires,
    /// peak hold @0x7507bc) - a door sound whose curve follows the door kept its tail.
    pub(super) peak: f32,
}

/// Everything one entry's update reads that is the same for every entry of a frame.
pub(super) struct EntryCtx<'a> {
    pub var: &'a dyn Fn(&str) -> Option<f32>,
    pub at_fire: &'a dyn Fn(&str, &str) -> Option<f32>,
    pub object_to_world: &'a Mat4,
    pub listener: Vec3,
    pub triggers: &'a [String],
    pub ai: bool,
    pub exterior: bool,
    pub master: f32,
    pub doppler: bool,
    pub now: Instant,
}

impl RuntimeSound {
    pub(super) fn new(def: SoundEntry, clip: Option<Arc<Clip>>) -> RuntimeSound {
        RuntimeSound {
            def,
            clip,
            voice: None,
            held: false,
            active_since: None,
            peak: 0.0,
        }
    }

    /// The per-frame update of this entry (see `SoundSet::update_fired` for the semantics
    /// of trigger, loop and one-shot entries).
    pub(super) fn update(&mut self, engine: &dyn Playback, cx: &EntryCtx) {
        // How the exe plays an entry (`TSound` update, 2.2.032):
        // * with a `[trigger]`: once each time the trigger fires, from the start, never
        //   looped (a `[loopsound]` too) and without looking at its conditions;
        // * without one: looped for as long as its conditions hold and it can be heard -
        //   `[sound]` and `[loopsound]` both loop by default;
        // * without one but with `[noloop]`: once, when its conditions start to hold.
        //   The mod buses' air sounds (ECAS kneeling, the parking brake valve, the
        //   start-up chime) are written like that; looped, they hissed for ever.
        let triggered = !self.def.triggers.is_empty();
        let holds = triggered || conditions::conditions_hold(&self.def, cx.var);
        let rising = !triggered && holds && !self.held;
        self.held = holds;
        if !triggered {
            if !holds {
                self.active_since = None;
            } else if self.active_since.is_none() {
                self.active_since = Some(cx.now);
            }
        }
        let Some(clip) = self.clip.clone() else { return };
        let active = self
            .active_since
            .map_or(0.0, |t| cx.now.saturating_duration_since(t).as_secs_f32());
        let facing = match (self.def.pos, self.def.dir) {
            (Some(p), Some(d)) => {
                let at = cx.object_to_world.transform_point3(Vec3::from_array(p));
                let dir = cx
                    .object_to_world
                    .transform_vector3(Vec3::from_array(d))
                    .normalize_or_zero();
                dir.dot((cx.listener - at).normalize_or_zero())
            }
            _ => 1.0,
        };
        let fired_by = if triggered {
            cx.triggers.iter().find(|t| {
                self.def
                    .triggers
                    .iter()
                    .any(|d| d.trim().eq_ignore_ascii_case(t))
            })
        } else {
            None
        };
        let fired = fired_by.is_some();
        let mut vol = match fired_by {
            Some(t) => volume_side(
                &self.def,
                &|n| (cx.at_fire)(t, n).or_else(|| (cx.var)(n)),
                cx.ai,
                active,
                facing,
            ),
            None => volume_side(&self.def, cx.var, cx.ai, active, facing),
        };
        if triggered {
            vol = conditions::peak_hold(&mut self.peak, vol, fired);
        }
        let (pitch, fast_enough) = conditions::pitch_of(&self.def, cx.var, clip.sample_rate);
        let audible = vol.map(|v| v > 0.001).unwrap_or(false) && fast_enough;
        let (position, reach, pan) =
            placement::place(self.def.pos, self.def.range, cx.exterior, cx.object_to_world);
        let params = |looping: bool| VoiceParams {
            gain: vol.unwrap_or(0.0) * cx.master,
            pitch: pitch.max(0.001),
            looping,
            position,
            doppler: cx.doppler,
            range: reach,
            lowpass_hz: 0.0,
            important: self.def.important,
            pan,
        };
        if !triggered && !self.def.no_loop {
            let params = params(true);
            match (self.voice, audible) {
                (Some(id), true) => {
                    if engine.is_playing(id) {
                        engine.set_params(id, params);
                    } else {
                        self.voice = Some(engine.play(clip, params));
                    }
                }
                (Some(id), false) => {
                    engine.stop(id);
                    self.voice = None;
                }
                (None, true) => self.voice = Some(engine.play(clip, params)),
                (None, false) => {}
            }
            return;
        }
        // one-shot: started by its trigger or by its conditions starting to hold; the
        // volume follows the curves while it plays (a triggered one only gets louder)
        let params = params(false);
        if (fired || rising) && audible {
            if let Some(id) = self.voice {
                if self.def.only_one && engine.is_playing(id) {
                    engine.set_params(id, params);
                    return;
                }
                engine.stop(id);
            }
            self.voice = Some(engine.play(clip, params));
        } else if let Some(id) = self.voice {
            if engine.is_playing(id) {
                engine.set_params(id, params);
            } else {
                self.voice = None;
            }
        }
    }
}

/// The volume of an entry for a listener on either side of the bodywork: the entry's
/// `[viewpoint]` says where its recording was made (inside the cabin / outside), not where
/// it may be heard - see [`crate::runtime::conditions::volume`]. Only the AI bit still
/// decides (an entry for AI vehicles only).
pub(super) fn volume_side(
    def: &SoundEntry,
    var: &dyn Fn(&str) -> Option<f32>,
    ai: bool,
    active: f32,
    facing: f32,
) -> Option<f32> {
    let view = (def.viewpoint & 3) | if ai { 4 } else { 0 };
    conditions::volume(def, var, view, active, facing, outside::outside_open())
}

/// Say once per file that a `(T.F.)` sound cannot be found: every AI bus of a type asks for
/// the same missing announcement at every stop.
pub(super) fn warn_missing_once(trigger: &str, path: &Path) {
    static SEEN: std::sync::Mutex<Option<std::collections::HashSet<std::path::PathBuf>>> =
        std::sync::Mutex::new(None);
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if seen
        .get_or_insert_with(Default::default)
        .insert(path.to_path_buf())
    {
        log::warn!(
            "sound {} for (T.F.{trigger}) not found (further requests for it are not logged)",
            path.display()
        );
    }
}
