//! The pause menu

use super::*;

mod dialog;
mod nav;
mod page;
mod screen;

pub use self::dialog::Dialog;
pub use self::page::{PAGE_COUNT, VEHICLE_PAGE};

pub const PAUSE_ENTRIES: [&str; 6] = [
    "pause.entry.resume",
    "pause.entry.save",
    "pause.entry.options",
    "pause.entry.world",
    "pause.entry.vehicle",
    "pause.entry.quit",
];

pub(super) fn t(key: &str) -> String {
    ::i18n::translate(key, &[])
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PauseState {
    pub page: Option<usize>,
    pub sel: usize,
}

#[derive(Clone, Copy)]
pub(super) struct Metrics {
    pub w: f32,
    pub h: f32,
    pub u: f32,
    pub mx: f32,
    pub line: f32,
}

impl Metrics {
    pub(super) fn new(f: &Frame) -> Metrics {
        let (w, h) = (f.width, f.height);
        let k = (h / (900.0 * f.scale.max(0.5) * f.ui_scale)).clamp(0.55, 1.0);
        let u = f.scale.max(0.5) * f.ui_scale * k;
        Metrics {
            w,
            h,
            u,
            mx: (w * 0.06).max(24.0 * u).round(),
            line: 1.0_f32.max(u).round(),
        }
    }
}

pub(super) fn out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
}

pub(super) fn fade(c: [u8; 4], k: f32) -> [u8; 4] {
    [c[0], c[1], c[2], (c[3] as f32 * k.clamp(0.0, 1.0)).round() as u8]
}

pub(super) fn mix(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2]), l(a[3], b[3])]
}

pub(super) const SCREEN: usize = usize::MAX;

pub(super) fn inside(rc: [f32; 4], p: (f32, f32)) -> bool {
    p.0 >= rc[0] && p.0 < rc[2] && p.1 >= rc[1] && p.1 < rc[3]
}

impl Ui {

    pub(super) fn pause_reset(&mut self) {
        self.pause_open = 0.0;
        self.page_t = 0.0;
        self.pause_last = SCREEN - 1;
    }

    pub(super) fn draw_pause(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        f: &Frame,
        st: PauseState,
    ) {
        let m = Metrics::new(f);
        let key = st.page.map_or(SCREEN, |t| t.min(PAGE_COUNT - 1));
        if key == SCREEN || self.pause_last >= PAGE_COUNT {
            if self.pause_last != key {
                self.page_fade = false;
                self.pause_last = key;
                self.page_t = 0.0;
            }
        } else if self.pause_last != key {
            if !self.page_fade || self.page_t >= 1.0 {
                self.page_fade = true;
                self.page_t = 0.0;
            } else if self.page_t > 0.5 {
                self.page_t = 1.0 - self.page_t;
            }
        }
        self.pause_open = (self.pause_open + self.anim_dt / 0.3).min(1.0);
        self.page_t = (self.page_t + self.anim_dt / 0.36).min(1.0);
        if self.page_fade && key != SCREEN && self.pause_last < PAGE_COUNT && self.page_t >= 0.5 {
            self.pause_last = key;
        }
        match st.page {
            None => {
                self.lab_tabs.clear();
                self.lab_groups.clear();
                self.lab_actions.clear();
                self.draw_pause_screen(r, scene, f, m, st.sel);
            }
            Some(tab) => {
                self.pause_items.clear();
                let shown = self.pause_last.min(PAGE_COUNT - 1);
                self.draw_pause_page(r, scene, f, m, tab.min(PAGE_COUNT - 1), shown);
            }
        }
        
        match self.dialog.take() {
            Some(d) => {
                self.draw_dialog(r, scene, f, m, &d);
                self.dialog = Some(d);
            }
            None => self.dialog_rects.clear(),
        }
    }
}