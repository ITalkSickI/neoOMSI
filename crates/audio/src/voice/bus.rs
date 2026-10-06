#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Bus { Vehicle, Ambience, Passenger, Announcement, Radio, Interface }
pub const BUS_COUNT: usize = 6;
pub const DEFAULT_GAINS: [f32; BUS_COUNT] = [1.0, 0.8, 1.0, 1.0, 0.7, 1.0];
pub const HEADROOM: f32 = 0.5;

