//! Turn the script host's fired sound triggers into the audio runtime's ordered events.
//!
//! The host keeps normal and `(T.F.)` triggers in one ordered list ([`FiredSound`]); this
//! attaches the variable snapshot of each firing (the host records them per trigger name,
//! in order) and the subsystem they came from, so `SoundSet::update_events` can evaluate a
//! triggered entry's volume curves at the moment it fired.

use omsi_audio::{EventSource, SoundEvent};
use omsi_sim::host::FiredSound;
use std::collections::{HashMap, VecDeque};

/// Build a subsystem's events from the triggers it fired this frame, in order.
pub(crate) fn events_from(
    source: EventSource,
    fired: &[FiredSound],
    fired_vars: &[(String, Vec<f32>)],
) -> Vec<SoundEvent> {
    let mut by_name: HashMap<String, VecDeque<&Vec<f32>>> = HashMap::new();
    for (name, vars) in fired_vars {
        by_name
            .entry(name.to_ascii_lowercase())
            .or_default()
            .push_back(vars);
    }
    fired
        .iter()
        .enumerate()
        .map(|(i, s)| match s {
            FiredSound::Trigger { name } => {
                let mut e = SoundEvent::trigger(source, i as u32, name.clone());
                if let Some(q) = by_name.get_mut(&name.to_ascii_lowercase()) {
                    if let Some(v) = q.pop_front() {
                        e = e.with_vars(v.clone());
                    }
                }
                e
            }
            FiredSound::File { name, file } => {
                SoundEvent::file(source, i as u32, name.clone(), file.clone())
            }
        })
        .collect()
}
