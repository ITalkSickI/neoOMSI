//! Bus routing and event forwarding for attached parts. The same stream and snapshots
//! must reach trailer entries; stripping file events or fire-time variables loses sound.
use super::soundset::SoundSet;
use super::event::SoundEvent;
use crate::engine::Playback;
use crate::voice::bus::Bus;
use glam::Mat4;

impl SoundSet {
    pub fn set_bus(&mut self, bus: Bus) {
        self.bus = bus;
        for (_, part) in &mut self.parts { part.set_bus(bus); }
    }
    /// File triggers conventionally carry announcements; arbitrary script audio can
    /// explicitly choose another bus. This does not change trigger ordering or conditions.
    pub fn set_file_bus(&mut self, bus: Bus) {
        self.file_bus = bus;
        for (_, part) in &mut self.parts { part.set_file_bus(bus); }
    }
    pub fn update_parts_events(&mut self, engine: &dyn Playback,
        var: &dyn Fn(&str) -> Option<f32>, part_to_world: &dyn Fn(usize) -> Option<Mat4>,
        events: &[SoundEvent], slots: &dyn Fn(&str) -> Option<usize>) {
        for (index, part) in &mut self.parts {
            match part_to_world(*index) {
                Some(transform) => {
                    if part.hull.is_none() { part.hull_override = Some(self.hull_h); }
                    part.update_events(engine, var, &transform, events, slots);
                }
                None => part.stop_all(engine),
            }
        }
    }
}
