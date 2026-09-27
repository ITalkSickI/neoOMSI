//! Game controllers — steering wheels, pedals, joysticks, gamepads — as OMSI drives them
//! from `Inputs/gamectrler.cfg` (Omsi.exe sub_648764). Each `[ctrl]` block names a device
//! and says what its eight DirectInput axes (X, Y, Z, Rx, Ry, Rz and the two sliders) do:
//! a pair per axis of the function (-1 none, 0 steering, 1 throttle, 2 brake, 3 clutch,
//! 4 throttle and brake on one axis — the options dialog's "<none>@Steering@Throttle@
//! Brake@Clutch@Throttle/Brake" less its first entry) and flags (bit 0: the axis runs the
//! other way, as the G25's pedals do). `[buttons]` lists per button the key action it
//! presses. A device the file does not know is taken as a gamepad: the left stick steers,
//! the right trigger is the throttle, the left one the brake. K switches the controller on
//! and off (OMSI's `toggel_ctrler`).

use gilrs::{Axis, EventType, Gilrs};
use std::path::Path;

/// What one axis of a device does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Func {
    Steering,
    Throttle,
    Brake,
    Clutch,
    ThrottleBrake,
}

impl Func {
    /// The file's number of the function (-1 none).
    pub(crate) fn code(f: Option<Func>) -> i32 {
        match f {
            None => -1,
            Some(Func::Steering) => 0,
            Some(Func::Throttle) => 1,
            Some(Func::Brake) => 2,
            Some(Func::Clutch) => 3,
            Some(Func::ThrottleBrake) => 4,
        }
    }

    pub(crate) fn from_code(c: i32) -> Option<Func> {
        match c {
            0 => Some(Func::Steering),
            1 => Some(Func::Throttle),
            2 => Some(Func::Brake),
            3 => Some(Func::Clutch),
            4 => Some(Func::ThrottleBrake),
            _ => None,
        }
    }

    /// As the options dialog lists them.
    pub(crate) const LABELS: [&'static str; 6] = ["<none>", "Steering", "Throttle", "Brake", "Clutch", "Throttle/Brake"];
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DeviceCfg {
    pub(crate) name: String,
    /// The line after the name (kept as the file has it).
    pub(crate) second: String,
    /// Per DirectInput axis: the function and whether it runs the other way.
    pub(crate) axes: [Option<(Func, bool)>; 8],
    /// Per axis the file's flags beyond bit 0 (kept as they are).
    pub(crate) axis_flags: [i32; 8],
    /// Per button: the key action (empty: none) and the number after it.
    pub(crate) buttons: Vec<(String, String)>,
    /// `[FFScale]`: the force feedback's strength (two factors).
    pub(crate) ff_scale: Option<(f32, f32)>,
}

/// The `gamectrler.cfg` in use: the content folder's (written by the launcher) before
/// OMSI 2's own.
pub(crate) fn cfg_path(root: &Path) -> std::path::PathBuf {
    omsi_cfg::find_in_roots("Inputs/gamectrler.cfg").map(|(_, p)| p).unwrap_or_else(|| root.join("Inputs").join("gamectrler.cfg"))
}

/// `Inputs/gamectrler.cfg`: the configured devices.
fn read_cfg(root: &Path) -> Vec<DeviceCfg> {
    let Ok(text) = std::fs::read(cfg_path(root)) else { return Vec::new() };
    parse_cfg(&omsi_cfg::codepage::decode(&text))
}

pub(crate) fn parse_cfg(text: &str) -> Vec<DeviceCfg> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out: Vec<DeviceCfg> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        match lines[i] {
            "[ctrl]" => {
                out.push(DeviceCfg { name: lines.get(i + 1).unwrap_or(&"").to_string(), second: lines.get(i + 2).unwrap_or(&"0").to_string(), ..Default::default() });
                i += 3;
            }
            "[axis]" => {
                if let Some(d) = out.last_mut() {
                    for a in 0..8 {
                        let f: i32 = lines.get(i + 1 + a * 2).and_then(|v| v.parse().ok()).unwrap_or(-1);
                        let flags: i32 = lines.get(i + 2 + a * 2).and_then(|v| v.parse().ok()).unwrap_or(0);
                        d.axes[a] = Func::from_code(f).map(|f| (f, flags & 1 != 0));
                        d.axis_flags[a] = flags & !1;
                    }
                }
                i += 17;
            }
            "[buttons]" => {
                let n: usize = lines.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(0).min(512);
                if let Some(d) = out.last_mut() {
                    for b in 0..n {
                        d.buttons.push((lines.get(i + 2 + b * 2).unwrap_or(&"").to_string(), lines.get(i + 3 + b * 2).unwrap_or(&"0").to_string()));
                    }
                }
                i += 2 + n * 2;
            }
            "[ffscale]" | "[FFScale]" => {
                if let Some(d) = out.last_mut() {
                    let f = |k: usize| lines.get(i + k).map(|v| omsi_cfg::parse_f64(v)).unwrap_or(1.0) as f32;
                    d.ff_scale = Some((f(1), f(2)));
                }
                i += 3;
            }
            _ => i += 1,
        }
    }
    out
}

/// The file's text for `devices`, as OMSI writes it (CR LF).
pub(crate) fn cfg_text(devices: &[DeviceCfg]) -> String {
    let mut t = String::new();
    for d in devices {
        t.push_str(&format!("\r\n[ctrl]\r\n{}\r\n{}\r\n\r\n[axis]\r\n", d.name, if d.second.is_empty() { "0" } else { &d.second }));
        for a in 0..8 {
            let (f, inv) = match d.axes[a] {
                Some((f, inv)) => (Func::code(Some(f)), inv),
                None => (-1, false),
            };
            t.push_str(&format!("{f}\r\n{}\r\n", d.axis_flags[a] | inv as i32));
        }
        t.push_str(&format!("\r\n[buttons]\r\n{}\r\n", d.buttons.len()));
        for (action, n) in &d.buttons {
            t.push_str(&format!("{action}\r\n{}\r\n", if n.is_empty() { "0" } else { n }));
        }
        let (a, b) = d.ff_scale.unwrap_or((1.0, 1.0));
        t.push_str(&format!("\r\n[FFScale]\r\n{a:.3}\r\n{b:.3}\r\n\r\n"));
    }
    t
}

/// The analog controls a controller gives this frame (None: that one is not on it).
#[derive(Debug, Clone, Copy, Default)]
pub struct Analog {
    pub steering: Option<f32>,
    pub throttle: Option<f32>,
    pub brake: Option<f32>,
    pub clutch: Option<f32>,
}

pub struct Controllers {
    gilrs: Option<Gilrs>,
    cfg: Vec<DeviceCfg>,
    pub enabled: bool,
    /// The settings' dead zone round the centre of a set-up device's axes (0..0.3).
    pub deadzone: f32,
    /// Key actions of buttons pressed (true) and released (false) since the last poll.
    pub actions: Vec<(String, bool)>,
    announced: Vec<String>,
    /// The rumble playing (`FF_Vib_Amp` and `FF_Vib_Period` of the bus), rebuilt when
    /// either changes.
    rumble: Option<(gilrs::ff::Effect, f32, f32)>,
}

impl Controllers {
    pub fn new(root: &Path) -> Controllers {
        let gilrs = match Gilrs::new() {
            Ok(g) => Some(g),
            Err(e) => {
                log::info!("game controllers: {e}");
                None
            }
        };
        let cfg = read_cfg(root);
        if let Some(g) = gilrs.as_ref() {
            for (_, pad) in g.gamepads() {
                log::info!("game controller: {} ({})", pad.name(), if cfg.iter().any(|d| names_match(&d.name, pad.name())) { "set up in gamectrler.cfg" } else { "as a gamepad" });
            }
        }
        Controllers { gilrs, cfg, deadzone: 0.0, enabled: true, actions: Vec::new(), announced: Vec::new(), rumble: None }
    }

    /// Read the devices: the analog controls, and the button actions into `actions`.
    pub fn poll(&mut self) -> Analog {
        let mut out = Analog::default();
        let Some(g) = self.gilrs.as_mut() else { return out };
        while let Some(ev) = g.next_event() {
            let pad = g.gamepad(ev.id);
            let cfg = self.cfg.iter().find(|d| names_match(&d.name, pad.name()));
            match ev.event {
                EventType::Connected => {
                    let name = pad.name().to_string();
                    if !self.announced.contains(&name) {
                        log::info!("game controller connected: {name}");
                        self.announced.push(name);
                    }
                }
                EventType::ButtonPressed(_, code) | EventType::ButtonReleased(_, code) => {
                    let down = matches!(ev.event, EventType::ButtonPressed(..));
                    if let Some(action) = cfg.and_then(|d| d.buttons.get(button_index(&pad, code))).filter(|a| !a.0.is_empty()) {
                        self.actions.push((action.0.clone(), down));
                    }
                }
                _ => {}
            }
        }
        if !self.enabled {
            return out;
        }
        // the devices set up in gamectrler.cfg first; a device the file does not know (a
        // gamepad) only gives what none of them does — a pad lying beside a set-up wheel
        // held the steering at its own centre, whichever the system listed first
        let mut pads: Vec<(Option<&DeviceCfg>, gilrs::Gamepad)> = g.gamepads().map(|(_, pad)| (self.cfg.iter().find(|d| names_match(&d.name, pad.name())), pad)).collect();
        pads.sort_by_key(|(cfg, _)| cfg.is_none());
        for (cfg, pad) in pads {
            match cfg {
                Some(d) => {
                    let axes: Vec<(u32, f32)> = pad.state().axes().map(|(c, d)| (c.into_u32(), d.value())).collect();
                    for (k, v) in di_slots(&axes) {
                        let Some((f, inverted)) = d.axes[k] else { continue };
                        let v = if inverted { -v } else { v };
                        // the dead zone: round the wheel's centre, or at a pedal's rest
                        let dz = self.deadzone.clamp(0.0, 0.3);
                        let v = match f {
                            Func::Steering | Func::ThrottleBrake => v.signum() * ((v.abs() - dz).max(0.0) / (1.0 - dz)),
                            _ => ((v + 1.0 - 2.0 * dz).max(0.0) / (1.0 - dz)) - 1.0,
                        };
                        // a pedal travels the whole range, -1 up to 1 down
                        let pedal = ((v + 1.0) * 0.5).clamp(0.0, 1.0);
                        match f {
                            Func::Steering => set(&mut out.steering, v.clamp(-1.0, 1.0)),
                            Func::Throttle => set(&mut out.throttle, pedal),
                            Func::Brake => set(&mut out.brake, pedal),
                            Func::Clutch => set(&mut out.clutch, pedal),
                            Func::ThrottleBrake => {
                                set(&mut out.throttle, (-v).max(0.0));
                                set(&mut out.brake, v.max(0.0));
                            }
                        }
                    }
                }
                None => {
                    // a gamepad: the left stick steers, the triggers are the pedals (only
                    // one the system knows as a gamepad: other devices' axes mean nothing
                    // here)
                    if pad.mapping_source() == gilrs::MappingSource::None {
                        continue;
                    }
                    let x = pad.value(Axis::LeftStickX);
                    let dead = |v: f32| if v.abs() < 0.08 { 0.0 } else { v };
                    let rt = pad.button_data(gilrs::Button::RightTrigger2).map(|d| d.value()).unwrap_or(0.0);
                    let lt = pad.button_data(gilrs::Button::LeftTrigger2).map(|d| d.value()).unwrap_or(0.0);
                    out.steering.get_or_insert(dead(x));
                    out.throttle.get_or_insert(rt.clamp(0.0, 1.0));
                    out.brake.get_or_insert(lt.clamp(0.0, 1.0));
                }
            }
        }
        out
    }

    /// The bus's force feedback: the scripts write `FF_Vib_Amp` (0..1, the engine's shaking,
    /// a rough road) and OMSI shakes the wheel or pad with it — as a rumble on every device
    /// that can (gilrs has no steering spring).
    /// `period`: `FF_Vib_Period` (Omsi.exe hands DirectInput Round(period × 10000) µs, so
    /// a hundredth of a second per unit): the shaking comes in pulses that long, on for
    /// half of it; 0 is a steady rumble.
    pub fn feedback(&mut self, amp: f32, period: f32) {
        let amp = if self.enabled { amp.clamp(0.0, 1.0) } else { 0.0 };
        let period = if period.is_finite() { period.clamp(0.0, 100.0) } else { 0.0 };
        let Some(g) = self.gilrs.as_mut() else { return };
        if let Some((_, was, was_period)) = &self.rumble {
            if (was - amp).abs() < 0.02 && (was_period - period).abs() < 0.05 {
                return;
            }
        }
        self.rumble = None;
        if amp < 0.01 {
            return;
        }
        let pads: Vec<gilrs::GamepadId> = g.gamepads().filter(|(_, p)| p.is_ff_supported()).map(|(id, _)| id).collect();
        if pads.is_empty() {
            return;
        }
        // (the file's [FFScale] of a set-up device scales it)
        let scale = self.cfg.iter().find_map(|d| d.ff_scale).map(|s| s.0).unwrap_or(1.0).clamp(0.0, 2.0);
        let m = ((amp * scale).min(1.0) * u16::MAX as f32) as u16;
        let ms = (period * 10.0).round() as u32;
        let scheduling = if ms >= 20 {
            gilrs::ff::Replay { after: gilrs::ff::Ticks::from_ms(0), play_for: gilrs::ff::Ticks::from_ms(ms / 2), with_delay: gilrs::ff::Ticks::from_ms(ms - ms / 2) }
        } else {
            Default::default()
        };
        let effect = gilrs::ff::EffectBuilder::new()
            .add_effect(gilrs::ff::BaseEffect { kind: gilrs::ff::BaseEffectType::Strong { magnitude: m }, scheduling, ..Default::default() })
            .add_effect(gilrs::ff::BaseEffect { kind: gilrs::ff::BaseEffectType::Weak { magnitude: m / 2 }, scheduling, ..Default::default() })
            .repeat(gilrs::ff::Repeat::Infinitely)
            .gamepads(&pads)
            .finish(g);
        if let Ok(e) = effect {
            let _ = e.play();
            self.rumble = Some((e, amp, period));
        }
    }

    /// Any controller there at all.
    pub fn any(&self) -> bool {
        self.gilrs.as_ref().map(|g| g.gamepads().next().is_some()).unwrap_or(false)
    }
}

/// A control several set-up devices give: the first one set wins, unless a later one is
/// moved further (two wheels, or pedals on their own device).
fn set(slot: &mut Option<f32>, v: f32) {
    match slot {
        Some(old) if old.abs() >= v.abs() => {}
        _ => *slot = Some(v),
    }
}

/// The DirectInput axis (0-5 X, Y, Z, Rx, Ry, Rz; 6, 7 the sliders) each of a device's axes
/// is, from the system's code for it: OMSI's gamectrler.cfg numbers them so. The HID usages
/// on macOS (generic desktop page 1: 0x30-0x35, slider 0x36, dial 0x37; the simulation page
/// 2: steering, accelerator, brake, clutch as Windows' HID driver places them), the evdev
/// ABS codes on Linux (ABS_X..ABS_RZ, then THROTTLE and RUDDER as the sliders), the axis
/// index on Windows. Axes of no known slot take the free ones in their order.
pub(crate) fn di_slots(axes: &[(u32, f32)]) -> Vec<(usize, f32)> {
    let known = |code: u32| -> Option<usize> {
        let (hi, lo) = (code >> 16, code & 0xFFFF);
        if cfg!(target_os = "macos") {
            match (hi, lo) {
                (1, 0x30..=0x35) => Some((lo - 0x30) as usize),
                (1, 0x36) => Some(6),
                (1, 0x37) => Some(7),
                (2, 0xC8) => Some(0), // steering
                (2, 0xC4) => Some(1), // accelerator
                (2, 0xC5) => Some(5), // brake
                (2, 0xC6) => Some(6), // clutch
                (2, 0xBB) => Some(2), // throttle
                (2, 0xBA) => Some(5), // rudder
                _ => None,
            }
        } else if cfg!(target_os = "linux") {
            match lo {
                0..=5 => Some(lo as usize),
                6 | 9 => Some(6), // ABS_THROTTLE, ABS_GAS
                7 | 10 => Some(7), // ABS_RUDDER, ABS_BRAKE
                _ => None,
            }
        } else {
            (lo < 8).then_some(lo as usize)
        }
    };
    let mut sorted = axes.to_vec();
    sorted.sort_by_key(|(c, _)| *c);
    let mut used = [false; 8];
    let mut out: Vec<(usize, f32)> = Vec::new();
    let mut rest = Vec::new();
    for (c, v) in sorted {
        match known(c).filter(|k| !used[*k]) {
            Some(k) => {
                used[k] = true;
                out.push((k, v));
            }
            None => rest.push(v),
        }
    }
    for v in rest {
        if let Some(k) = used.iter().position(|u| !u) {
            used[k] = true;
            out.push((k, v));
        }
    }
    out
}

/// OMSI stores DirectInput's product name; the system's may differ in spacing and case.
pub(crate) fn names_match(a: &str, b: &str) -> bool {
    let n = |s: &str| s.to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>();
    let (a, b) = (n(a), n(b));
    !a.is_empty() && (a == b || a.contains(&b) || b.contains(&a))
}

/// The button's number on its device (DirectInput counts them in the order the device
/// lists them, as the HID codes run).
pub(crate) fn button_number(pad: &gilrs::Gamepad, code: gilrs::ev::Code) -> usize {
    button_index(pad, code)
}

fn button_index(pad: &gilrs::Gamepad, code: gilrs::ev::Code) -> usize {
    let mut codes: Vec<u32> = pad.state().buttons().map(|(c, _)| c.into_u32()).collect();
    codes.sort_unstable();
    codes.iter().position(|c| *c == code.into_u32()).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    #[test]
    fn names() {
        assert!(super::names_match("Logitech G25 Racing Wheel USB", "Logitech G25 Racing Wheel"));
        assert!(!super::names_match("", "x"));
    }
}

#[cfg(test)]
mod slot_tests {
    use super::*;

    #[test]
    fn a_missing_axis_does_not_shift_the_rest() {
        // X and Rz only (a wheel with one pedal axis): Rz stays slot 5
        let axes = if cfg!(target_os = "macos") {
            vec![(0x1_0030, 0.5), (0x1_0035, -1.0)]
        } else {
            vec![(0x3_0000, 0.5), (0x3_0005, -1.0)]
        };
        let got = di_slots(&axes);
        assert!(got.contains(&(0, 0.5)), "{got:?}");
        if !cfg!(windows) {
            assert!(got.contains(&(5, -1.0)), "{got:?}");
        }
    }

    #[test]
    fn two_devices_merge_by_the_larger_movement() {
        let mut s = None;
        set(&mut s, 0.0);
        set(&mut s, 0.4);
        set(&mut s, -0.1);
        assert_eq!(s, Some(0.4));
    }
}

#[cfg(test)]
mod cfg_tests {
    #[test]
    fn the_stock_file_round_trips() {
        let Ok(bytes) = std::fs::read("../../OMSI 2 Original/Inputs/gamectrler.cfg") else { return };
        let text = omsi_cfg::codepage::decode(&bytes);
        let devs = super::parse_cfg(&text);
        assert!(devs.iter().any(|d| d.name.contains("G25")), "{:?}", devs.iter().map(|d| &d.name).collect::<Vec<_>>());
        let again = super::parse_cfg(&super::cfg_text(&devs));
        assert_eq!(again, devs);
    }
}
