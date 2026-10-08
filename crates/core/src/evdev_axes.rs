use gilrs::{GamepadId, Gilrs, LinuxGamepadExt};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Default)]
struct AbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

impl AbsInfo {
    fn normalized(&self) -> f32 {
        let span = i64::from(self.maximum) - i64::from(self.minimum);
        if span <= 0 {
            return 0.0;
        }
        let offset = i64::from(self.value) - i64::from(self.minimum);
        (offset as f64 / span as f64 * 2.0 - 1.0).clamp(-1.0, 1.0) as f32
    }
}

fn read_axis(file: &File, code: u32) -> Option<f32> {
    let mut info = AbsInfo::default();
    let request = 0x8000_4540 | (std::mem::size_of::<AbsInfo>() as u32) << 16 | code;
    let result = unsafe { libc::ioctl(file.as_raw_fd(), request as _, &mut info) };
    (result >= 0).then(|| info.normalized())
}

fn capabilities(file: &File, kind: u32, bits: &mut [u8]) -> bool {
    let request = 0x8000_4500 | (0x20 + kind) | (bits.len() as u32) << 16;
    unsafe { libc::ioctl(file.as_raw_fd(), request as _, bits.as_mut_ptr()) >= 0 }
}

fn has_bit(bits: &[u8], bit: usize) -> bool {
    bits.get(bit / 8)
        .is_some_and(|byte| byte & (1 << (bit % 8)) != 0)
}

fn axis_codes(bits: &[u8]) -> impl Iterator<Item = u32> + '_ {
    (0..=10).filter(|code| has_bit(bits, *code as usize))
}

struct Wheel {
    file: File,
    axes: Vec<(u32, f32)>,
}

impl Wheel {
    fn open(path: &Path) -> Option<Self> {
        let file = File::open(path).ok()?;
        let mut effects = [0u8; 16];
        if !capabilities(&file, 0x15, &mut effects) || !has_bit(&effects, 0x52) {
            return None;
        }
        let mut absolute = [0u8; 8];
        if !capabilities(&file, 3, &mut absolute) {
            return None;
        }
        let axes: Vec<_> = axis_codes(&absolute)
            .filter_map(|code| read_axis(&file, code).map(|value| (0x3_0000 | code, value)))
            .collect();
        (!axes.is_empty()).then_some(Self { file, axes })
    }

    fn poll(&mut self) -> bool {
        for (code, value) in &mut self.axes {
            let Some(current) = read_axis(&self.file, *code & 0xFFFF) else {
                return false;
            };
            *value = current;
        }
        true
    }
}

pub(crate) struct Wheels {
    devices: Vec<(GamepadId, Option<Wheel>)>,
    retry_at: Instant,
}

impl Wheels {
    pub(crate) fn new() -> Self {
        Self {
            devices: Vec::new(),
            retry_at: Instant::now(),
        }
    }

    pub(crate) fn poll(&mut self, gilrs: Option<&Gilrs>) {
        let Some(gilrs) = gilrs else {
            self.devices.clear();
            return;
        };
        self.devices
            .retain(|(id, _)| gilrs.gamepad(*id).is_connected());
        if self.retry_at.elapsed() >= Duration::from_secs(3) {
            self.devices.retain(|(_, wheel)| wheel.is_some());
            self.retry_at = Instant::now();
        }
        for (id, pad) in gilrs.gamepads() {
            if !self.devices.iter().any(|(known, _)| *known == id) {
                self.devices.push((id, Wheel::open(pad.devpath())));
            }
        }
        for (_, wheel) in &mut self.devices {
            if wheel.as_mut().is_some_and(|wheel| !wheel.poll()) {
                *wheel = None;
            }
        }
    }

    pub(crate) fn axes(&self, id: GamepadId) -> Option<&[(u32, f32)]> {
        self.devices
            .iter()
            .find(|(known, _)| *known == id)?
            .1
            .as_ref()
            .map(|wheel| wheel.axes.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::AbsInfo;

    #[test]
    fn wheel_capabilities_include_unmoved_z_but_exclude_absent_axes_hats_and_misc() {
        let capabilities = [0x27, 0, 0x03, 0, 0, 0x01, 0, 0];
        assert_eq!(
            super::axis_codes(&capabilities).collect::<Vec<_>>(),
            [0, 1, 2, 5]
        );
        assert!(super::axis_codes(&[0; 8]).next().is_none());
    }

    #[test]
    fn unsigned_wheel_and_pedal_ranges_keep_their_native_direction() {
        let value = |value| {
            AbsInfo {
                value,
                minimum: 0,
                maximum: 65535,
                ..Default::default()
            }
            .normalized()
        };
        assert_eq!(value(0), -1.0);
        assert!(value(32768).abs() < 0.0001);
        assert_eq!(value(65535), 1.0);
    }

    #[test]
    fn signed_and_short_pedal_ranges_use_the_reported_limits() {
        for (minimum, maximum) in [(-32768, 32767), (0, 1023), (10, 110)] {
            for (value, expected) in [(minimum, -1.0), (maximum, 1.0)] {
                assert_eq!(
                    AbsInfo {
                        value,
                        minimum,
                        maximum,
                        ..Default::default()
                    }
                    .normalized(),
                    expected
                );
            }
        }
    }

    #[test]
    fn invalid_ranges_and_out_of_range_values_are_bounded() {
        assert_eq!(AbsInfo::default().normalized(), 0.0);
        assert_eq!(
            AbsInfo {
                value: i32::MAX,
                minimum: i32::MIN,
                maximum: i32::MAX,
                ..Default::default()
            }
            .normalized(),
            1.0
        );
        assert_eq!(
            AbsInfo {
                value: -10,
                minimum: 0,
                maximum: 100,
                ..Default::default()
            }
            .normalized(),
            -1.0
        );
    }

    #[test]
    fn abs_info_matches_the_kernel_layout() {
        assert_eq!(std::mem::size_of::<AbsInfo>(), 24);
    }
}
