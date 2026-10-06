//! Source of "now" for the sound runtime and mixer.
//!
//! Normal playback uses the wall clock. Tests and the offline renderer install a manual
//! clock instead, whose time only moves when [`Clock::advance`] is called: sound activity,
//! the inside/outside blend and the Doppler shift then depend on the number of mixed frames
//! alone, so a fixture can be played back and compared bit for bit.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A cheap, cloneable handle to a time source (see the module docs).
#[derive(Clone)]
pub struct Clock(Arc<Inner>);

enum Inner {
    Real,
    Manual(Mutex<Instant>),
}

impl Default for Clock {
    fn default() -> Self {
        Self::real()
    }
}

impl Clock {
    /// A clock reading the system time.
    pub fn real() -> Clock {
        Clock(Arc::new(Inner::Real))
    }

    /// A clock frozen at `start`; move it with [`Clock::advance`].
    pub fn manual(start: Instant) -> Clock {
        Clock(Arc::new(Inner::Manual(Mutex::new(start))))
    }

    /// The current time.
    pub fn now(&self) -> Instant {
        match &*self.0 {
            Inner::Real => Instant::now(),
            Inner::Manual(t) => *t.lock().unwrap_or_else(|e| e.into_inner()),
        }
    }

    /// Move a manual clock forward. Does nothing on a real clock.
    pub fn advance(&self, by: Duration) {
        if let Inner::Manual(t) = &*self.0 {
            let mut t = t.lock().unwrap_or_else(|e| e.into_inner());
            *t += by;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manual_clock_only_moves_when_advanced() {
        let start = Instant::now();
        let c = Clock::manual(start);
        assert_eq!(c.now(), start);
        c.advance(Duration::from_millis(20));
        assert_eq!(c.now(), start + Duration::from_millis(20));
        // clones share the same time
        let d = c.clone();
        d.advance(Duration::from_millis(5));
        assert_eq!(c.now(), start + Duration::from_millis(25));
    }

    #[test]
    fn a_real_clock_advances_itself() {
        let c = Clock::real();
        let a = c.now();
        c.advance(Duration::from_secs(3600));
        assert!(c.now() >= a, "advancing a real clock does nothing");
    }
}
