//! One entry of a sound configuration: its loaded clip, its running voice and the per-frame
//! decision of how it plays. The pure rules it calls live in [`crate::runtime::conditions`]
//! and [`crate::runtime::placement`]; the engine only starts and stops voices.

use crate::assets::Clip;
use crate::engine::Playback;
use crate::runtime::{conditions, outside, placement};
use crate::voice::{Level, MixParams, VoiceId};
use glam::{Mat4, Vec3};
use omsi_vehicle::SoundEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// The logical state of one entry, apart from whether its asset is in the cache. The
/// renderer may keep a running voice virtualized (or a spatializer may change how it
/// sounds), but that never changes these states: the runtime owns them, so a renderer swap
/// cannot alter conditions, selection or trigger lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundState {
    /// No asset and none requested (a `[sound] N` waiting for its `(T.F.)` file).
    NotLoaded,
    /// A start was accepted while the fixed file is still being read.
    Loading,
    /// The asset is present and the entry is idle.
    Ready,
    /// A voice is running logically.
    Running,
    /// The conditions or the `[viewpoint]` silence it this frame.
    Suppressed,
    /// Its conditions no longer hold / it was stopped.
    Stopped,
    /// A one-shot voice reached its end.
    Ended,
}

/// What asset an entry wants, kept apart from the loaded [`Clip`]: a fixed file is read
/// lazily (and retried) from the cache, a numeric `[sound] N` gets its file from a
/// `(T.F.trigger)`. This split lets an asynchronous load deliver a clip after an event was
/// already accepted without leaving a contradictory playing state.
pub(super) enum Asset {
    Dynamic,
    Fixed(PathBuf),
}

/// The `[onlyone]` registry of one sound set: the voice currently running for a file. Kept
/// on the sound set (not a global) so the scope is explicit and testable. The exact OMSI
/// scope (same file / same vehicle / global) is OPEN - see the behavior doc, section 6.
pub(super) type OnlyOne = HashMap<String, VoiceId>;

pub struct RuntimeSound {
    pub(super) def: SoundEntry,
    asset: Asset,
    /// The resolved clip, if the asset is loaded (see [`Asset`]).
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
    /// A one-shot start accepted while the clip was still loading: started as soon as the
    /// clip arrives, so an asynchronous load does not swallow the event.
    pending: bool,
    pub(super) state: SoundState,
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
    /// How muffled a foreign bus's sound is by the listener's own bodywork (0..1): the
    /// inside/outside transmission for an `exterior` set, see [`crate::runtime::outside`].
    pub muffled: f32,
    pub master: f32,
    pub doppler: bool,
    pub now: Instant,
}

impl RuntimeSound {
    pub(super) fn new(def: SoundEntry, path: Option<PathBuf>) -> RuntimeSound {
        RuntimeSound {
            def,
            asset: match path {
                Some(p) => Asset::Fixed(p),
                None => Asset::Dynamic,
            },
            clip: None,
            voice: None,
            held: false,
            active_since: None,
            peak: 0.0,
            pending: false,
            state: SoundState::NotLoaded,
        }
    }

    /// The logical state of the entry (see [`SoundState`]).
    pub fn state(&self) -> SoundState {
        self.state
    }

    /// Resolve the fixed asset lazily. A cached hit is cheap; a missing file stays cached
    /// as missing, so the underlying read happens at most once and a retry only re-checks
    /// the cache. A `[sound] N` waits for its `(T.F.)` file.
    fn resolve_asset(&mut self, engine: &dyn Playback) -> Option<Arc<Clip>> {
        if let Some(c) = &self.clip {
            return Some(c.clone());
        }
        if let Asset::Fixed(path) = &self.asset {
            if let Some(c) = engine.load_clip(path) {
                self.clip = Some(c.clone());
                return Some(c);
            }
        }
        None
    }

    /// The per-frame update of this entry (see `SoundSet::update_fired` for the semantics
    /// of trigger, loop and one-shot entries). `only_one` is the sound set's central
    /// `[onlyone]` registry, keyed by file. `admit` is the runtime's OMSI admission
    /// decision (see [`crate::runtime::level::SOUND_MAXCOUNT`]): when false the entry may
    /// keep an already-running voice but must not start a new one this frame. That is
    /// separate from the mixer's technical `MAX_VOICES` virtualization, which never changes
    /// what the runtime believes is playing.
    pub(super) fn update(
        &mut self,
        engine: &dyn Playback,
        cx: &EntryCtx,
        admit: bool,
        only_one: &mut OnlyOne,
    ) {
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
        // The separate legacy values (recording level, script volume, inside/outside). A
        // triggered entry reads its curve at the instant it fired and holds the peak of the
        // combined volume (TSound +0x2c); the renderer clamps the product only at the end, so
        // a level is never cut before the global master (see `runtime::level`).
        let split = match fired_by {
            Some(t) => volume_side(
                &self.def,
                &|n| (cx.at_fire)(t, n).or_else(|| (cx.var)(n)),
                cx.ai,
                active,
                facing,
            ),
            None => volume_side(&self.def, cx.var, cx.ai, active, facing),
        };
        // the conditions or the `[viewpoint]` silence it this frame
        let suppressed = split.is_none();
        let mut level = Level::Omsi {
            record: 0.0,
            script: 1.0,
            transmission: 1.0,
            master: cx.master,
        };
        let mut pre_master = 0.0;
        if let Some((record, script, through)) = split {
            // the inside/outside transmission: the own bus's `Snd_OutsideVol` share (folded
            // into `through`) times the bodywork the listener's own cabin puts between them
            let (record, script, through) = if triggered {
                let combined = conditions::peak_hold(
                    &mut self.peak,
                    Some(record * script * through),
                    fired,
                )
                .unwrap_or(0.0);
                (combined, 1.0, 1.0)
            } else {
                (record, script, through)
            };
            let transmission = through * outside::outside_gain(cx.muffled, cx.exterior);
            pre_master = record * script * transmission;
            level = Level::Omsi {
                record,
                script,
                transmission,
                master: cx.master,
            };
        }
        let Some(clip) = self.resolve_asset(engine) else {
            // no asset yet: an accepted one-shot start waits for the clip instead of being
            // swallowed by the asynchronous load
            let accepted = (fired || rising) && !suppressed;
            if accepted && (triggered || self.def.no_loop) {
                self.pending = true;
            }
            if let Some(id) = self.voice.take() {
                engine.stop(id);
            }
            self.state = if suppressed {
                SoundState::Suppressed
            } else if self.pending || matches!(self.asset, Asset::Fixed(_)) {
                SoundState::Loading
            } else {
                SoundState::NotLoaded
            };
            return;
        };
        let (pitch, fast_enough) = conditions::pitch_of(&self.def, cx.var, clip.sample_rate);
        let audible = pre_master > 0.001 && fast_enough;
        let (position, reach, pan) =
            placement::place(self.def.pos, self.def.range, cx.exterior, cx.object_to_world);
        // the inside/outside timbre: a foreign bus heard from the cabin loses its edge as
        // the bodywork closes (the transmission level is separate, above)
        let lowpass_hz = outside::lowpass_of(cx.muffled, cx.exterior);
        let params = |looping: bool| MixParams {
            level,
            pitch: pitch.max(0.001),
            looping,
            position,
            doppler: cx.doppler,
            range: reach,
            lowpass_hz,
            important: self.def.important,
            pan,
        };
        if !triggered && !self.def.no_loop {
            let params = params(true);
            match (self.voice, audible) {
                (Some(id), true) => {
                    if engine.is_playing(id) {
                        engine.set_mix_params(id, params);
                    } else if admit {
                        self.voice = Some(engine.play_mix(clip, params));
                    } else {
                        self.voice = None;
                        self.state = SoundState::Ready;
                        return;
                    }
                    self.state = SoundState::Running;
                }
                (Some(id), false) => {
                    engine.stop(id);
                    self.voice = None;
                    self.state = if suppressed {
                        SoundState::Suppressed
                    } else {
                        SoundState::Stopped
                    };
                }
                (None, true) => {
                    if admit {
                        self.voice = Some(engine.play_mix(clip, params));
                        self.state = SoundState::Running;
                    } else {
                        self.state = SoundState::Ready;
                    }
                }
                (None, false) => {
                    self.state = if suppressed {
                        SoundState::Suppressed
                    } else {
                        SoundState::Ready
                    };
                }
            }
            return;
        }
        // one-shot: started by its trigger or by its conditions starting to hold; the
        // volume follows the curves while it plays (a triggered one only gets louder)
        let params = params(false);
        let start = fired || rising || self.pending;
        self.pending = false;
        if start && audible && admit {
            if self.def.only_one {
                let key = only_one_key(&self.def);
                if let Some(existing) = only_one.get(&key).copied() {
                    if engine.is_playing(existing) {
                        engine.set_mix_params(existing, params);
                        self.voice = Some(existing);
                        self.state = SoundState::Running;
                        return;
                    }
                    only_one.remove(&key);
                }
            }
            if let Some(id) = self.voice {
                engine.stop(id);
            }
            let id = engine.play_mix(clip, params);
            self.voice = Some(id);
            if self.def.only_one {
                only_one.insert(only_one_key(&self.def), id);
            }
            self.state = SoundState::Running;
        } else if let Some(id) = self.voice {
            if engine.is_playing(id) {
                engine.set_mix_params(id, params);
                self.state = SoundState::Running;
            } else {
                self.voice = None;
                self.state = SoundState::Ended;
            }
        } else {
            self.state = if suppressed {
                SoundState::Suppressed
            } else {
                SoundState::Ready
            };
        }
    }
}

/// The key of an entry in the `[onlyone]` registry: its file, case-insensitive. The
/// reference manages `[onlyone]` centrally per file; the exact scope is OPEN.
fn only_one_key(def: &SoundEntry) -> String {
    def.file.trim().to_ascii_lowercase()
}

/// The separate level values of an entry for a listener on either side of the bodywork:
/// `(record, script, transmission)` before the 0 dB clamp. The entry's `[viewpoint]` says
/// where its recording was made (inside the cabin / outside), not where it may be heard -
/// see [`crate::runtime::conditions::split`]. Only the AI bit still decides (an entry for AI
/// vehicles only).
pub(super) fn volume_side(
    def: &SoundEntry,
    var: &dyn Fn(&str) -> Option<f32>,
    ai: bool,
    active: f32,
    facing: f32,
) -> Option<(f32, f32, f32)> {
    let view = (def.viewpoint & 3) | if ai { 4 } else { 0 };
    conditions::split(def, var, view, active, facing, outside::outside_open())
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
