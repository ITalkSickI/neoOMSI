//! Game controllers on Windows through DirectInput 8, as Omsi.exe reads them: every device
//! Windows lists as a game controller (wheels with their makers' drivers, pedals, joysticks,
//! button boxes - the system's newer Windows.Gaming.Input misses many of them), the eight
//! axes in the slots `gamectrler.cfg` numbers (X, Y, Z, Rx, Ry, Rz, the two sliders), up to
//! 128 buttons, and force feedback: one constant force on the wheel's axis the game sets
//! every frame (its centring spring, the drag of the steering, the shaking of the bus).
//!
//! The list of devices is looked up on a thread of its own when Windows reports a device
//! change. Some drivers take a tenth of a second to enumerate and can stall input polling
//! even when the enumeration is on another thread.

use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use windows::core::{Interface, GUID};
use windows::Win32::Devices::HumanInterfaceDevice::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

/// What a device gives: the axes (-1..1) in their DirectInput slots and 128 buttons. Laid out
/// as the data format below says.
#[repr(C)]
#[derive(Clone, Copy)]
struct RawState {
    axes: [i32; 8],
    pov: [u32; 4],
    buttons: [u8; 128],
}

impl Default for RawState {
    fn default() -> Self {
        RawState { axes: [0; 8], pov: [u32::MAX; 4], buttons: [0; 128] }
    }
}

const RANGE: i32 = 10_000;

/// One device opened.
pub(crate) struct Device {
    pub name: String,
    guid: GUID,
    dev: IDirectInputDevice8W,
    /// The slots the device has (an axis it lacks is not in the list).
    has_axis: [bool; 8],
    state: RawState,
    ff: Option<IDirectInputEffect>,
    pub buttons: usize,
}

/// Take the device again (after the window left the front). DirectInput resets a force
/// feedback wheel when it is taken: its own centring came back on, and a G29 pulled itself
/// to the middle after the pause until mouse steering was switched on and off.
fn reacquire(dev: &IDirectInputDevice8W, ff: bool) -> bool {
    unsafe {
        let off = || {
            let mut ac = DIPROPDWORD { diph: DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPDWORD>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: 0, dwHow: DIPH_DEVICE }, dwData: DIPROPAUTOCENTER_OFF };
            let _ = dev.SetProperty(prop(9), &mut ac.diph);
        };
        if ff {
            off();
        }
        let ok = dev.Acquire().is_ok();
        if ff {
            off();
        }
        ok
    }
}

impl Device {
    /// The axes the device has: (slot, value -1..1).
    pub fn axes(&self) -> Vec<(usize, f32)> {
        (0..8).filter(|k| self.has_axis[*k]).map(|k| (k, self.state.axes[k] as f32 / RANGE as f32)).collect()
    }

    pub fn has_ff(&self) -> bool {
        self.ff.is_some()
    }
}

/// The devices, and the thread that finds them.
pub(crate) struct DirectInput {
    di: IDirectInput8W,
    hwnd: HWND,
    /// The game wants force feedback (the window is the game's, not the launcher's).
    ff: bool,
    pub devices: Vec<Device>,
    found: Arc<Mutex<Option<Vec<(GUID, String)>>>>,
    scan: mpsc::Sender<()>,
    /// Button changes since the last `poll`: (device, button, down).
    pub events: Vec<(String, usize, bool)>,
    last_force: Instant,
}

unsafe extern "system" fn collect(inst: *mut DIDEVICEINSTANCEW, out: *mut core::ffi::c_void) -> windows::core::BOOL {
    let v = &mut *(out as *mut Vec<(GUID, String)>);
    let inst = &*inst;
    let end = inst.tszProductName.iter().position(|c| *c == 0).unwrap_or(inst.tszProductName.len());
    v.push((inst.guidInstance, String::from_utf16_lossy(&inst.tszProductName[..end]).trim().to_string()));
    windows::core::BOOL(DIENUM_CONTINUE as i32)
}

fn create() -> Option<IDirectInput8W> {
    unsafe {
        let hinst: HINSTANCE = GetModuleHandleW(None).ok()?.into();
        let mut p: *mut core::ffi::c_void = std::ptr::null_mut();
        DirectInput8Create(hinst, DIRECTINPUT_VERSION, &IDirectInput8W::IID, &mut p, None).ok()?;
        (!p.is_null()).then(|| IDirectInput8W::from_raw(p))
    }
}

fn list(di: &IDirectInput8W) -> Vec<(GUID, String)> {
    let mut v: Vec<(GUID, String)> = Vec::new();
    unsafe {
        let _ = di.EnumDevices(DI8DEVCLASS_GAMECTRL, Some(collect), &mut v as *mut _ as *mut core::ffi::c_void, DIEDFL_ATTACHEDONLY);
    }
    v
}

/// The data format: `RawState`, every object optional (as the SDK's c_dfDIJoystick2 has it
/// for the parts used here).
fn data_format() -> (Vec<DIOBJECTDATAFORMAT>, DIDATAFORMAT) {
    static AXES: [GUID; 8] = [GUID_XAxis, GUID_YAxis, GUID_ZAxis, GUID_RxAxis, GUID_RyAxis, GUID_RzAxis, GUID_Slider, GUID_Slider];
    let mut objs = Vec::new();
    for (k, g) in AXES.iter().enumerate() {
        objs.push(DIOBJECTDATAFORMAT { pguid: g, dwOfs: (k * 4) as u32, dwType: DIDFT_AXIS | DIDFT_OPTIONAL | DIDFT_ANYINSTANCE, dwFlags: DIDOI_ASPECTPOSITION });
    }
    for k in 0..4 {
        objs.push(DIOBJECTDATAFORMAT { pguid: &GUID_POV, dwOfs: (32 + k * 4) as u32, dwType: DIDFT_POV | DIDFT_OPTIONAL | DIDFT_ANYINSTANCE, dwFlags: 0 });
    }
    for k in 0..128 {
        objs.push(DIOBJECTDATAFORMAT { pguid: std::ptr::null(), dwOfs: (48 + k) as u32, dwType: DIDFT_BUTTON | DIDFT_OPTIONAL | DIDFT_ANYINSTANCE, dwFlags: 0 });
    }
    let f = DIDATAFORMAT {
        dwSize: std::mem::size_of::<DIDATAFORMAT>() as u32,
        dwObjSize: std::mem::size_of::<DIOBJECTDATAFORMAT>() as u32,
        dwFlags: DIDF_ABSAXIS,
        dwDataSize: std::mem::size_of::<RawState>() as u32,
        dwNumObjs: objs.len() as u32,
        rgodf: std::ptr::null_mut(),
    };
    (objs, f)
}

/// (dinput.h's DIDFT_OPTIONAL: a device without the object is still taken)
const DIDFT_OPTIONAL: u32 = 0x8000_0000;

/// `MAKEDIPROP(n)`: DirectInput's own properties are numbers passed where a GUID's address
/// goes.
fn prop(n: usize) -> *const GUID {
    n as *const GUID
}

impl DirectInput {
    /// `hwnd`: the window the devices belong to; `ff`: take the devices for force feedback
    /// (the game's window: they then answer only while it is in front, as in OMSI).
    pub fn new(hwnd: isize, ff: bool) -> Option<DirectInput> {
        let di = create()?;
        let found = Arc::new(Mutex::new(Some(list(&di))));
        let f2 = found.clone();
        let (scan, requests) = mpsc::channel();
        let _ = std::thread::Builder::new().name("game controllers".into()).spawn(move || {
            let Some(di) = create() else { return };
            while requests.recv().is_ok() {
                // One plug/unplug can produce several events. Enumerate only once for them.
                while requests.try_recv().is_ok() {}
                let v = list(&di);
                *f2.lock().unwrap() = Some(v);
            }
        });
        Some(DirectInput { di, hwnd: HWND(hwnd as *mut _), ff, devices: Vec::new(), found, scan, events: Vec::new(), last_force: Instant::now() })
    }

    pub fn refresh(&self) {
        let _ = self.scan.send(());
    }

    fn open(&self, guid: &GUID, name: &str) -> Option<Device> {
        unsafe {
            let mut dev: Option<IDirectInputDevice8W> = None;
            self.di.CreateDevice(guid, &mut dev, None).ok()?;
            let dev = dev?;
            let (mut objs, mut fmt) = data_format();
            fmt.rgodf = objs.as_mut_ptr();
            dev.SetDataFormat(&mut fmt).ok()?;
            let mut caps = DIDEVCAPS { dwSize: std::mem::size_of::<DIDEVCAPS>() as u32, ..Default::default() };
            let _ = dev.GetCapabilities(&mut caps);
            let wants_ff = self.ff && caps.dwFlags & DIDC_FORCEFEEDBACK != 0;
            let level = if wants_ff { DISCL_EXCLUSIVE | DISCL_FOREGROUND } else { DISCL_NONEXCLUSIVE | DISCL_BACKGROUND };
            if dev.SetCooperativeLevel(self.hwnd, level).is_err() {
                dev.SetCooperativeLevel(self.hwnd, DISCL_NONEXCLUSIVE | DISCL_BACKGROUND).ok()?;
            }
            // every axis from -RANGE to RANGE
            let mut range = DIPROPRANGE { diph: DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPRANGE>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: 0, dwHow: DIPH_DEVICE }, lMin: -RANGE, lMax: RANGE };
            let _ = dev.SetProperty(prop(4), &mut range.diph);
            // no dead zone and no saturation of the driver's own (DIPROP_DEADZONE 5,
            // DIPROP_SATURATION 6): some wheels' drivers set one - a PXN V99 lost 30 % of
            // its turn round the middle - and OMSI clears them as well; the settings'
            // dead zone is the only one
            for (p_id, v) in [(5usize, 0u32), (6, 10_000)] {
                let mut d = DIPROPDWORD { diph: DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPDWORD>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: 0, dwHow: DIPH_DEVICE }, dwData: v };
                let _ = dev.SetProperty(prop(p_id), &mut d.diph);
            }
            // which slots the device has (the data of an absent one reads as the middle)
            let mut has_axis = [false; 8];
            for (k, h) in has_axis.iter_mut().enumerate() {
                let mut info = DIDEVICEOBJECTINSTANCEW { dwSize: std::mem::size_of::<DIDEVICEOBJECTINSTANCEW>() as u32, ..Default::default() };
                *h = dev.GetObjectInfo(&mut info, (k * 4) as u32, DIPH_BYOFFSET).is_ok();
            }
            let _ = dev.Acquire();
            let mut ff = None;
            if wants_ff {
                // the wheel's own centring off: the game's forces take its place
                let mut ac = DIPROPDWORD { diph: DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPDWORD>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: 0, dwHow: DIPH_DEVICE }, dwData: DIPROPAUTOCENTER_OFF };
                if dev.SetProperty(prop(9), &mut ac.diph).is_err() {
                    log::warn!("{name}: force feedback could not be activated (autocenter deactivation impossible)");
                }
                let mut axes = [0u32; 1];
                let mut dirs = [0i32; 1];
                let mut cf = DICONSTANTFORCE { lMagnitude: 0 };
                let mut eff = DIEFFECT {
                    dwSize: std::mem::size_of::<DIEFFECT>() as u32,
                    dwFlags: DIEFF_CARTESIAN | DIEFF_OBJECTOFFSETS,
                    dwDuration: u32::MAX, // INFINITE
                    dwGain: DI_FFNOMINALMAX,
                    dwTriggerButton: DIEB_NOTRIGGER,
                    cAxes: 1,
                    rgdwAxes: axes.as_mut_ptr(),
                    rglDirection: dirs.as_mut_ptr(),
                    cbTypeSpecificParams: std::mem::size_of::<DICONSTANTFORCE>() as u32,
                    lpvTypeSpecificParams: &mut cf as *mut _ as *mut core::ffi::c_void,
                    ..Default::default()
                };
                let mut e: Option<IDirectInputEffect> = None;
                if dev.CreateEffect(&GUID_ConstantForce, &mut eff, &mut e, None).is_ok() {
                    if let Some(e) = e.as_ref() {
                        let _ = e.Start(1, 0);
                    }
                    ff = e;
                }
            }
            log::info!("game controller (DirectInput): {name}, {} axes, {} buttons{}", has_axis.iter().filter(|a| **a).count(), caps.dwButtons, if ff.is_some() { ", force feedback" } else { "" });
            Some(Device { name: name.to_string(), guid: *guid, dev, has_axis, state: RawState::default(), ff, buttons: caps.dwButtons as usize })
        }
    }

    /// Read every device; devices plugged in or out since the last list are opened or let go.
    pub fn poll(&mut self) {
        if let Some(list) = self.found.lock().unwrap().take() {
            self.devices.retain(|d| list.iter().any(|(g, _)| *g == d.guid));
            for (g, name) in list {
                if !self.devices.iter().any(|d| d.guid == g) {
                    if let Some(d) = self.open(&g, &name) {
                        self.devices.push(d);
                    }
                }
            }
        }
        for d in &mut self.devices {
            let mut s = RawState { axes: d.state.axes, ..Default::default() };
            let read = |s: &mut RawState| unsafe {
                let _ = d.dev.Poll();
                d.dev.GetDeviceState(std::mem::size_of::<RawState>() as u32, s as *mut _ as *mut core::ffi::c_void)
            };
            let ok = match read(&mut s) {
                Ok(()) => true,
                Err(_) => reacquire(&d.dev, d.ff.is_some()) && read(&mut s).is_ok(),
            };
            if !ok {
                continue;
            }
            for b in 0..128 {
                let (was, now) = (d.state.buttons[b] & 0x80 != 0, s.buttons[b] & 0x80 != 0);
                if was != now {
                    self.events.push((d.name.clone(), b, now));
                }
            }
            // the hat switches as buttons after the 128 (up, right, down, left of each): the
            // D-pad of a wheel rim - Moza's among others - is a hat, and could not be given a key
            for k in 0..4 {
                let dirs = |pov: u32| -> [bool; 4] {
                    if pov == u32::MAX || pov & 0xFFFF == 0xFFFF {
                        return [false; 4];
                    }
                    let a = (pov % 36000) as i32;
                    let near = |c: i32| {
                        let d = (a - c).rem_euclid(36000);
                        d.min(36000 - d) < 6750
                    };
                    [near(0), near(9000), near(18000), near(27000)]
                };
                let (was, now) = (dirs(d.state.pov[k]), dirs(s.pov[k]));
                for dir in 0..4 {
                    if was[dir] != now[dir] {
                        self.events.push((d.name.clone(), crate::controllers::HAT_BUTTONS + k * 4 + dir, now[dir]));
                    }
                }
            }
            d.state = s;
        }
    }

    /// The force on the wheel of device `name`: -1 (full to the left) .. 1. Set at most 100
    /// times a second (each is a message to the device).
    pub fn set_force(&mut self, name: &str, f: f32) {
        if self.last_force.elapsed() < Duration::from_millis(10) {
            return;
        }
        self.last_force = Instant::now();
        for d in self.devices.iter().filter(|d| d.name == name) {
            let Some(e) = d.ff.as_ref() else { continue };
            let mut axes = [0u32; 1];
            let mut dirs = [0i32; 1];
            let mut cf = DICONSTANTFORCE { lMagnitude: (f.clamp(-1.0, 1.0) * DI_FFNOMINALMAX as f32) as i32 };
            let mut eff = DIEFFECT {
                dwSize: std::mem::size_of::<DIEFFECT>() as u32,
                dwFlags: DIEFF_CARTESIAN | DIEFF_OBJECTOFFSETS,
                cAxes: 1,
                rgdwAxes: axes.as_mut_ptr(),
                rglDirection: dirs.as_mut_ptr(),
                cbTypeSpecificParams: std::mem::size_of::<DICONSTANTFORCE>() as u32,
                lpvTypeSpecificParams: &mut cf as *mut _ as *mut core::ffi::c_void,
                ..Default::default()
            };
            unsafe {
                // (a device taken away - the window left the front - is taken again)
                if e.SetParameters(&mut eff, DIEP_TYPESPECIFICPARAMS | DIEP_START).is_err() {
                    reacquire(&d.dev, true);
                    let _ = e.SetParameters(&mut eff, DIEP_TYPESPECIFICPARAMS | DIEP_START);
                }
            }
        }
    }
}

impl Drop for DirectInput {
    fn drop(&mut self) {
        for d in &self.devices {
            unsafe {
                if let Some(e) = d.ff.as_ref() {
                    let _ = e.Stop();
                }
                let _ = d.dev.Unacquire();
            }
        }
    }
}
