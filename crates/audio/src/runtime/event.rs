//! The ordered sound-event stream of one frame.
//!
//! OMSI fires script triggers, `(T.F.)` file triggers and loudspeaker announcements in the
//! order the scripts run. neoOMSI used to keep normal triggers and file triggers in two
//! separate lists and to replay them subsystem by subsystem, so the relative order was an
//! implicit consequence of the redraw order. [`SoundEvent`] makes that explicit: every
//! event carries the subsystem it came from ([`EventSource`]), a sequence number within
//! that subsystem, the moment it fired, an optional file and the variable snapshot needed
//! to evaluate its volume curves. Repeated firings survive as separate events.
//!
//! The subsystems run in a fixed order (traffic -> player -> LAN -> scenery, see the
//! redraw loop), which is the ordering rank of [`EventSource`]; within one subsystem the
//! sequence number keeps the script's firing order. `(source, seq)` is therefore a total,
//! reproducible order for the frame.
//!
//! The exact rule for two identical triggers in one frame is not settled by the static
//! reference: the runtime keeps the current behavior (an entry restarts once per frame) and
//! the stream records both firings. See `docs/audio/SOUND_ENGINE_BEHAVIOR.md`, section 4.

use std::time::Instant;

/// Which subsystem produced an event. The variant order is the redraw order and the
/// ordering rank used to merge events of one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventSource {
    /// AI traffic (`Traffic::update_audio`, redrawn first).
    Traffic,
    /// The player's own vehicle.
    Player,
    /// Other players / LAN vehicles.
    Lan,
    /// Scenery objects and lamps.
    Scenery,
}

/// One sound trigger of one frame. `file` is set for a `(T.F.trigger)` event (the entry
/// listening for `trigger` plays `file`); `vars` is the script variable snapshot of the
/// moment it fired, for the volume curves a triggered entry reads (`doorSpeed_<n>` and the
/// like, which the script changes again right after firing).
#[derive(Clone, Debug)]
pub struct SoundEvent {
    pub source: EventSource,
    /// Firing order within the source (the index in the subsystem's event list).
    pub seq: u32,
    /// When it fired; `None` means "the frame's clock now".
    pub at: Option<Instant>,
    pub trigger: String,
    /// `Some` for a `(T.F.trigger)` file trigger.
    pub file: Option<String>,
    /// The variables as they stood when the trigger fired.
    pub vars: Option<Vec<f32>>,
}

impl SoundEvent {
    /// A normal `(T.trigger)` event.
    pub fn trigger(source: EventSource, seq: u32, trigger: impl Into<String>) -> SoundEvent {
        SoundEvent {
            source,
            seq,
            at: None,
            trigger: trigger.into(),
            file: None,
            vars: None,
        }
    }

    /// A `(T.F.trigger)` file event: `trigger` names the entry, `file` is what it plays.
    pub fn file(
        source: EventSource,
        seq: u32,
        trigger: impl Into<String>,
        file: impl Into<String>,
    ) -> SoundEvent {
        SoundEvent {
            source,
            seq,
            at: None,
            trigger: trigger.into(),
            file: Some(file.into()),
            vars: None,
        }
    }

    /// The same event with a firing time.
    pub fn at(mut self, at: Instant) -> SoundEvent {
        self.at = Some(at);
        self
    }

    /// The same event with the variable snapshot of the moment it fired.
    pub fn with_vars(mut self, vars: Vec<f32>) -> SoundEvent {
        self.vars = Some(vars);
        self
    }

    pub fn is_file(&self) -> bool {
        self.file.is_some()
    }
}

/// Sort a frame's events by their explicit order `(source, seq)`. The runtime keeps the
/// events in the order the subsystems produced them; this makes the total order available
/// to the read-back and future global sinks. A stable sort keeps two events with the same
/// order as produced.
pub fn ordered(events: &mut [SoundEvent]) {
    events.sort_by_key(|e| (e.source, e.seq));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_order_is_the_redraw_order() {
        assert!(EventSource::Traffic < EventSource::Player);
        assert!(EventSource::Player < EventSource::Lan);
        assert!(EventSource::Lan < EventSource::Scenery);
    }

    #[test]
    fn merging_uses_source_then_sequence() {
        let mut events = vec![
            SoundEvent::trigger(EventSource::Scenery, 0, "a"),
            SoundEvent::trigger(EventSource::Player, 1, "b"),
            SoundEvent::trigger(EventSource::Traffic, 2, "c"),
            SoundEvent::trigger(EventSource::Player, 0, "d"),
        ];
        ordered(&mut events);
        let names: Vec<&str> = events.iter().map(|e| e.trigger.as_str()).collect();
        assert_eq!(names, ["c", "d", "b", "a"]);
    }

    #[test]
    fn a_file_event_carries_its_file() {
        let e = SoundEvent::file(EventSource::Player, 0, "ev_nbs", "announce.wav");
        assert!(e.is_file());
        assert_eq!(e.file.as_deref(), Some("announce.wav"));
        assert!(!SoundEvent::trigger(EventSource::Player, 0, "t").is_file());
    }
}
