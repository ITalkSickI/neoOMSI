use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct CamCfg {
    pub direct: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub pos: [f32; 3],
}

impl CamCfg {
    pub const DEFAULT: CamCfg = CamCfg {
        direct: false,
        yaw: 0.0,
        pitch: 0.0,
        fov: 0.0,
        pos: [0.0; 3],
    };
}

#[derive(Clone, Default)]
pub(crate) struct CamInfo {
    pub part: usize,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub radius: f32,
    pub aimed_yaw: f32,
    pub aimed_pitch: f32,
    pub eye: [f64; 3],
    pub seen: bool,
    pub direct: bool,
}

static CFG: Mutex<Vec<CamCfg>> = Mutex::new(Vec::new());
static INFO: Mutex<Vec<CamInfo>> = Mutex::new(Vec::new());
static WANTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn cfg(i: usize) -> CamCfg {
    CFG.lock().get(i).copied().unwrap_or(CamCfg::DEFAULT)
}

pub(crate) fn set_cfg(i: usize, c: CamCfg) {
    let mut g = CFG.lock();
    if g.len() <= i {
        if c == CamCfg::DEFAULT {
            return;
        }
        g.resize(i + 1, CamCfg::DEFAULT);
    }
    g[i] = c;
}

pub(crate) fn reset_all() {
    CFG.lock().clear();
}

pub(crate) fn want(on: bool) {
    WANTED.store(on, Ordering::Relaxed);
}

pub(crate) fn wants() -> bool {
    WANTED.load(Ordering::Relaxed)
}

pub(crate) fn publish(info: Vec<CamInfo>) {
    *INFO.lock() = info;
}

pub(crate) fn info() -> Vec<CamInfo> {
    INFO.lock().clone()
}