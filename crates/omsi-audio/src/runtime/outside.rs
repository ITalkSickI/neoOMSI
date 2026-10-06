//! `Snd_OutsideVol`, the one global of the runtime: how open the listener's bus is to the
//! outside (doors, driver's window), written by the player's scripts every frame and read at
//! the sound-set level. [`outside_gain`] and [`lowpass_of`] are the stock
//! `sound_volume.osc` shaping, wired into an exterior sound set's transmission while the
//! listener sits in a cabin (see `runtime::sound`). The own bus's outside-only entries go
//! through `conditions::split` instead; the two paths are disjoint.

/// `Snd_OutsideVol` of the bus the listener sits in (bits of an f32; NaN = none).
static OUTSIDE_OPEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x7fc0_0000);

/// Set how open the listener's bus is to the outside (`Snd_OutsideVol`), or `None`.
pub fn set_outside_open(v: Option<f32>) {
    OUTSIDE_OPEN.store(
        v.filter(|x| x.is_finite()).unwrap_or(f32::NAN).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
}

pub(super) fn outside_open() -> Option<f32> {
    let v = f32::from_bits(OUTSIDE_OPEN.load(std::sync::atomic::Ordering::Relaxed));
    (!v.is_nan()).then_some(v)
}

/// A cutoff `t` of the way from `a` to `b` (0 = no filter, treated as wide open).
pub(super) fn lp_between(a: f32, b: f32, t: f32) -> f32 {
    if a <= 0.0 && b <= 0.0 {
        return 0.0;
    }
    let open = 20_000.0f32;
    let (a, b) = (if a > 0.0 { a } else { open }, if b > 0.0 { b } else { open });
    let c = (a.ln() + (b.ln() - a.ln()) * t).exp();
    if c >= 0.95 * open { 0.0 } else { c }
}

/// The lower of two low-pass cutoffs, 0 meaning none.
#[allow(dead_code)]
pub(super) fn merge_lowpass(a: f32, b: f32) -> f32 {
    match (a > 0.0, b > 0.0) {
        (true, true) => a.min(b),
        (true, false) => a,
        (false, true) => b,
        _ => 0.0,
    }
}

/// How muffled an entry should sound, once the listener sits in *some* cabin (`muffled`):
/// a foreign vehicle's sound set (`exterior`, an AI bus or another player's) is muffled
/// then - its sounds are on the far side of the player's own bodywork and glass. The
/// player's own bus is not: OMSI plays its entries as the `[viewpoint]` lets them through
/// and leaves the rest to the scripts' volume curves (`Snd_OutsideVol` and the like). We
/// muffled every entry of it not tagged as a cabin sound alone - a blinker relay tagged
/// for inside and out was cut to a quarter below 450 Hz, heard only with a door open.
pub(super) fn lowpass_of(muffled: f32, exterior: bool, opening: Option<f32>) -> f32 {
    if muffled > 0.0 && exterior {
        // doors or the driver's window open let the outside in unfiltered
        let shut = match opening {
            Some(o) => 450.0 * (1.0 + 30.0 * o.clamp(0.0, 0.5)),
            None => 450.0,
        };
        lp_between(0.0, shut, muffled)
    } else {
        0.0
    }
}

/// The share of an outside sound heard in the cab: the stock `sound_volume.osc` writes
/// `Snd_OutsideVol` (0 with everything shut, up to 0.5 with doors or the driver's window
/// open: "when doors are open, you can hear outside sounds louder"); a shut bus keeps a
/// quarter, an open one all of it. Without the variable the level stays as it was.
pub(super) fn outside_gain(muffled: f32, exterior: bool) -> f32 {
    outside_gain_at(muffled, exterior, outside_open())
}

pub(super) fn outside_gain_at(muffled: f32, exterior: bool, opening: Option<f32>) -> f32 {
    if muffled > 0.0 && exterior {
        let shut = match opening {
            Some(o) => (0.25 + 1.5 * o.clamp(0.0, 0.5)).min(1.0),
            None => 1.0,
        };
        1.0 + (shut - 1.0) * muffled
    } else {
        1.0
    }
}

/// Continuous timbre/transfer for the own vehicle's inside/outside recordings. This is
/// listening-tuned bodywork mixing, not a change to OPEN viewpoint admission semantics.
/// Apply transmission once, after script/peak handling. If a curve explicitly already
/// reads Snd_OutsideVol, leave its gain to that curve (filtering still models the body).
pub(super) fn entry_transfer(def: &omsi_vehicle::SoundEntry, exterior: bool,
    inside: f32, muffled: f32) -> (f32, f32) {
    let opening = outside_open();
    if exterior { return (outside_gain_at(muffled, true, opening), lowpass_of(muffled, true, opening)); }
    entry_transfer_at(def, inside, opening)
}

pub(super) fn entry_transfer_at(def: &omsi_vehicle::SoundEntry,
    inside: f32, opening: Option<f32>) -> (f32, f32) {
    match def.viewpoint & 3 {
        1 => {
            let gain = if def.vol_curves.iter().any(|c| c.variable.eq_ignore_ascii_case("Snd_OutsideVol")) {
                1.0
            } else {
                let shut = 0.25 + 1.5 * opening.unwrap_or(0.0).clamp(0.0, 0.5);
                1.0 + (shut - 1.0) * inside.clamp(0.0, 1.0)
            };
            let shut_hz = 450.0 * (1.0 + 30.0 * opening.unwrap_or(0.0).clamp(0.0, 0.5));
            (gain, lp_between(0.0, shut_hz, inside))
        }
        2 => {
            let outside = 1.0 - inside.clamp(0.0, 1.0);
            (1.0 - 0.75 * outside, lp_between(0.0, 1200.0, outside))
        }
        _ => (1.0, 0.0),
    }
}
