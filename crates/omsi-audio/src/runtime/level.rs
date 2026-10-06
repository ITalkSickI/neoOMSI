//! Legacy level arithmetic shared by the runtime: OMSI's conversion of a linear level to
//! DirectSound's hundredths of a decibel, and the admission budget that is *not* the
//! renderer's voice limit.
//!
//! The reference's `TSound` update (`00750444`) multiplies the recording level, the script
//! volume curves, the inside/outside transmission and the global masters, then clamps the
//! result to 0 dB and sends it as hundredths of dB (`SetVolume`). The renderer order is:
//!
//! ```text
//! buffer = clamp01(record × script × transmission × set_master)   // recording/script/transmission
//! final  = clamp01(buffer × listener_master)                      // global volume/pause
//! sample = sample × final × distance_gain × pan × low-pass        // distance/pan/frequency separate
//! ```
//!
//! `listener_master` (the game's volume setting) is folded into the same clamp as the set
//! master, so a loud source is not cut before the global volume has been applied. Distance
//! and inside/outside enter exactly once each: distance in the spatializer, transmission in
//! [`crate::runtime::conditions`] or [`crate::runtime::outside`].

/// OMSI's `[sound_maxcount]`: how many voices the runtime admits before it stops starting
/// new ones. This is an *OMSI* decision (a sound stopped here has the lifetime rules of its
/// type); the renderer's own [`crate::engine::mixer::MAX_VOICES`] is a separate,
/// performance-only virtualization that never changes what the runtime believes is playing.
pub(crate) const SOUND_MAXCOUNT: usize = 200;

/// A linear level as DirectSound's hundredths of a decibel: at most 0 (0 dB, no gain over
/// the recording) and at least -100 dB. `00750444` clamps to `-10000` and converts with the
/// logarithm before `SetVolume` (vtable +0x3c).
#[cfg(test)]
pub(crate) fn hundredths_db(level: f32) -> i32 {
    if level <= 0.0 || !level.is_finite() {
        return -10_000;
    }
    let db = 20.0 * level.log10();
    (db * 100.0).round().clamp(-10_000.0, 0.0) as i32
}

/// The inverse of [`hundredths_db`], for fixtures and read-back.
#[cfg(test)]
pub(crate) fn from_hundredths_db(value: i32) -> f32 {
    10f32.powf(value as f32 / 2_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_scale_is_zero_db_and_over_it_is_cut() {
        assert_eq!(hundredths_db(1.0), 0);
        assert_eq!(hundredths_db(2.0), 0, "no gain over 0 dB");
        assert_eq!(hundredths_db(44_100.0), 0, "the MB 412D case: cut to 0 dB");
    }

    #[test]
    fn half_scale_is_about_minus_six_db() {
        assert_eq!(hundredths_db(0.5), -602);
        assert_eq!(hundredths_db(0.0), -10_000);
        assert_eq!(hundredths_db(-1.0), -10_000);
    }

    #[test]
    fn the_conversion_round_trips() {
        for level in [1.0f32, 0.5, 0.25, 0.1] {
            let back = from_hundredths_db(hundredths_db(level));
            assert!((back - level).abs() < 0.02, "{level} -> {back}");
        }
    }
}
