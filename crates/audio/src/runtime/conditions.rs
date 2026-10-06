//! Pure OMSI sound rules: conditions, volume curves and what they read, playback rate and
//! the triggered-sound peak hold.
//!
//! Nothing here touches a device, a lock or a sound set: each function works only on the
//! parsed [`SoundEntry`] and a lookup for script variables, so the rules can be reasoned
//! about and tested on their own.

use ::legacy_vehicle::{SoundEntry, VolCurve};

/// Linear interpolation of a volume curve's points; 1 without points, clamped to the first
/// and last point outside their range.
pub(crate) fn curve(points: &[(f32, f32)], x: f32) -> f32 {
    if points.is_empty() {
        return 1.0;
    }
    if x <= points[0].0 {
        return points[0].1;
    }
    let last = points[points.len() - 1];
    if x >= last.0 {
        return last.1;
    }
    for w in points.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if x >= x0 && x <= x1 {
            return if x1 == x0 {
                y1
            } else {
                y0 + (y1 - y0) * (x - x0) / (x1 - x0)
            };
        }
    }
    last.1
}

/// What a `[volcurve]` reads. The exe (`TSound` load, 0x74e408) looks the name up in the
/// vehicle's variables and, when it is none of them, stores `StrToInt(name)` as the index;
/// the update (0x750584) then reads a negative index from the sound's own values: -1 =
/// seconds since the sound became active (its conditions started to hold,
/// GetTickCount/1000), -2 = how much a `[3d]` sound with a direction faces the listener
/// (1 without one), anything below is skipped with "Volume Variable not valid!". The LiAZ
/// 5292's engine loops fade in with `[volcurve] -1` (0 at 1 s, 1 at 1.3 s); read as an
/// unknown variable = 0 they were silent for ever.
pub(crate) fn curve_input(
    vc: &VolCurve,
    var: &dyn Fn(&str) -> Option<f32>,
    active: f32,
    facing: f32,
) -> Option<f32> {
    match vc.variable.trim().parse::<i32>() {
        Ok(-1) => Some(active),
        Ok(-2) => Some(facing),
        Ok(n) if n < 0 => None,
        _ => Some(var(&vc.variable).unwrap_or(0.0)),
    }
}

pub(crate) fn conditions_hold(def: &SoundEntry, var: &dyn Fn(&str) -> Option<f32>) -> bool {
    def.conditions
        .iter()
        .all(|c| c.holds(var(&c.variable).unwrap_or(0.0)))
}

/// The separate legacy level values of an entry, before the 0 dB clamp:
/// `(record, script, transmission)`. `record` is the recording level (`def.volume`),
/// `script` the product of the script volume curves, `transmission` the inside/outside
/// share (1.0, or `Snd_OutsideVol`). `None` when a condition or the `[viewpoint]` silences
/// the sound; `view` is [`placement::view_mask`]. `outside_open` is how far the listener's
/// bus is open to the outside (`Snd_OutsideVol`), or `None` when the scripts give none.
///
/// The exe (`TSound` update) evaluates the conditions only for entries without a
/// `[trigger]`: a triggered entry plays whenever its trigger fires, whatever its conditions
/// say. It multiplies the recording and script factors and the transmission and clamps only
/// once, at the very end (0 dB max); see [`crate::runtime::level`].
pub(crate) fn split(
    def: &SoundEntry,
    var: &dyn Fn(&str) -> Option<f32>,
    view: i32,
    active: f32,
    facing: f32,
    outside_open: Option<f32>,
) -> Option<(f32, f32, f32)> {
    // (an outside sound of the bus the camera sits in - no bit 2, the SD200's exterior
    // engine at `[viewpoint] 5` - comes into the cab through what is open, at
    // `Snd_OutsideVol`: TSound update 0x750340, played when that is over 0.01 and its
    // volume multiplied by it)
    let mut through = 1.0;
    if def.viewpoint != 0 && def.viewpoint & view == 0 {
        match outside_open {
            Some(o) if view == 2 && def.viewpoint & 2 == 0 && o > 0.01 => through = o,
            _ => return None,
        }
    }
    if def.triggers.is_empty() && !conditions_hold(def, var) {
        return None;
    }
    let mut script = 1.0;
    for vc in &def.vol_curves {
        if let Some(x) = curve_input(vc, var, active, facing) {
            script *= curve(&vc.points, x);
        }
    }
    Some((def.volume, script, through))
}

/// [`split`] as the single clamped factor the pure rules and tests use: the renderer itself
/// keeps the parts separate and clamps after the global master (see
/// [`crate::runtime::level`]). DirectSound has no gain over 0 dB, so a factor over 1 plays
/// at 1 - the MB 412D's `[sound] start2.wav` carries a loop sound's lines ("44100" read as
/// its volume) and its start-up roared 44 100 times too loud.
#[cfg(test)]
pub(crate) fn volume(
    def: &SoundEntry,
    var: &dyn Fn(&str) -> Option<f32>,
    view: i32,
    active: f32,
    facing: f32,
    outside_open: Option<f32>,
) -> Option<f32> {
    split(def, var, view, active, facing, outside_open)
        .map(|(record, script, through)| (record * script * through).clamp(0.0, 1.0))
}

/// The playback rate of a `[loopsound]` relative to its clip, and whether it is fast enough
/// to be played at all: the exe sets the buffer's frequency to
/// `|pitch variable| * sample rate / reference` and silences it below DirectSound's 100 Hz
/// minimum. 1 for a plain `[sound]`.
pub(crate) fn pitch_of(
    def: &SoundEntry,
    var: &dyn Fn(&str) -> Option<f32>,
    clip_sample_rate: u32,
) -> (f32, bool) {
    if def.is_loop && def.pitch_ref != 0.0 && !def.pitch_variable.is_empty() {
        let rate = if def.sample_rate > 0.0 {
            def.sample_rate
        } else {
            clip_sample_rate as f32
        };
        let hz = var(&def.pitch_variable).unwrap_or(0.0).abs() * rate / def.pitch_ref;
        (hz / clip_sample_rate.max(1) as f32, hz >= 100.0)
    } else {
        (1.0, true)
    }
}

/// A triggered entry's volume this frame: never below what it has been since its trigger
/// fired (`fired`: this frame). `None` (heard from the wrong view) stays `None`.
pub(crate) fn peak_hold(peak: &mut f32, vol: Option<f32>, fired: bool) -> Option<f32> {
    if fired {
        *peak = 0.0;
    }
    let v = vol?.max(*peak);
    *peak = v;
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<f32> {
        None
    }

    #[test]
    fn curve_clamps_and_interpolates() {
        assert_eq!(curve(&[], 5.0), 1.0);
        let pts = [(-1.0, 1.0), (-0.5, 0.0)];
        assert_eq!(curve(&pts, -2.0), 1.0);
        assert_eq!(curve(&pts, -0.75), 0.5);
        assert_eq!(curve(&pts, 0.0), 0.0);
    }

    #[test]
    fn negative_curve_variables_are_the_sounds_own() {
        let vc = |name: &str| VolCurve {
            variable: name.into(),
            points: vec![(0.0, 0.0), (1.0, 1.0)],
        };
        assert_eq!(curve_input(&vc("-1"), &none, 0.7, 1.0), Some(0.7));
        assert_eq!(curve_input(&vc("-2"), &none, 0.0, 0.25), Some(0.25));
        assert_eq!(curve_input(&vc("-3"), &none, 0.0, 1.0), None);
        assert_eq!(
            curve_input(&vc("engine_n"), &|n| (n == "engine_n").then_some(42.0), 0.0, 1.0),
            Some(42.0)
        );
    }

    /// A triggered sound reads its curve at the moment it fires, not at the frame's end.
    #[test]
    fn triggered_volume_reads_at_fire_time() {
        let hit = SoundEntry {
            volume: 1.0,
            triggers: vec!["ev_doorhitclose_0".into()],
            vol_curves: vec![VolCurve {
                variable: "doorSpeed_0".into(),
                points: vec![(-1.0, 1.0), (-0.5, 0.0)],
            }],
            ..Default::default()
        };
        // read at the frame's end the door speed has turned round
        let now = |n: &str| (n == "doorSpeed_0").then_some(0.8);
        assert_eq!(volume(&hit, &now, 2, 0.0, 1.0, None), Some(0.0));
        let fired = |n: &str| (n == "doorSpeed_0").then_some(-1.0);
        assert_eq!(volume(&hit, &fired, 2, 0.0, 1.0, None), Some(1.0));
    }

    #[test]
    fn a_triggered_sound_keeps_its_peak_while_it_plays() {
        // a door sound whose volcurve follows the door: fired with the door at 1, the door
        // closed the next frame - Omsi.exe holds the peak (0x7507bc)
        let mut peak = 0.7;
        assert_eq!(peak_hold(&mut peak, Some(1.0), true), Some(1.0));
        assert_eq!(peak_hold(&mut peak, Some(0.0), false), Some(1.0));
        assert_eq!(peak_hold(&mut peak, None, false), None, "wrong view: silent");
        // fired again quieter: the old peak is forgotten
        assert_eq!(peak_hold(&mut peak, Some(0.3), true), Some(0.3));
    }

    #[test]
    fn a_loop_pitch_is_read_from_its_variable() {
        let def = SoundEntry {
            is_loop: true,
            sample_rate: 44_100.0,
            pitch_variable: "engine_n".into(),
            pitch_ref: 600.0,
            ..Default::default()
        };
        let var = |n: &str| (n == "engine_n").then_some(1200.0);
        let (pitch, ok) = pitch_of(&def, &var, 44_100);
        assert!((pitch - 2.0).abs() < 1e-6 && ok);
        // below DirectSound's 100 Hz the buffer is silent
        let slow = |n: &str| (n == "engine_n").then_some(1.0);
        assert!(!pitch_of(&def, &slow, 44_100).1);
        // a plain sound plays at its own rate
        assert_eq!(
            pitch_of(&SoundEntry::default(), &none, 44_100),
            (1.0, true)
        );
    }
}
