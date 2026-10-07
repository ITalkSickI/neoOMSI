//! The pages! (Backrooms for pages ig?)

use super::*;

mod map;
mod options;
mod vehicle;
mod world;

pub(super) type DrawFn = fn(&mut Ui, &Renderer, &mut Scene, &Frame, Metrics, f32, f32);

pub(super) struct Page {
    pub nav: &'static str,
    pub draw: DrawFn,
}

pub(super) const PAGES: [Page; 4] = [map::PAGE, options::PAGE, world::PAGE, vehicle::PAGE];

pub const PAGE_COUNT: usize = PAGES.len();

pub const VEHICLE_PAGE: usize = 3;

impl Ui {
    pub(in super::super) fn draw_pause_page(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        f: &Frame,
        m: Metrics,
        tab: usize,
        shown: usize,
    ) {
        let Metrics { w, h, u, .. } = m;
        let o = out(self.pause_open);
        self.text
            .rounded(r, scene, [0.0, 0.0, w, h], 0.0, fade([6, 8, 12, 232], o));
        let bar_bottom = self.draw_nav(r, scene, f, m, tab);
        let fade_in = self.page_fade;
        let pt = if fade_in { 1.0 } else { self.page_t };

        if shown != VEHICLE_PAGE {
            self.lab_groups.clear();
            self.lab_actions.clear();
        }
        (PAGES[shown].draw)(self, r, scene, f, m, bar_bottom + 32.0 * u, pt);

        if fade_in {
            let t = self.page_t;
            let cover = out(if t < 0.5 { t * 2.0 } else { (1.0 - t) * 2.0 });
            if cover > 0.0 {
                self.text.rounded(
                    r,
                    scene,
                    [0.0, bar_bottom + m.line, w, h],
                    0.0,
                    fade([6, 8, 12, 255], cover),
                );
            }
        }
    }

    pub(super) fn draw_page_head(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        m: Metrics,
        head: &str,
        note: &str,
        y: f32,
        pt: f32,
    ) -> f32 {
        let Metrics { u, mx, .. } = m;
        let hs = out(pt / 0.6);
        let hl = self.text.label(r, scene, &t(head), (34.0 * u) as u32, WHITE);
        hl.place(scene, mx - 40.0 * u * (1.0 - hs), y);
        let nl = self.text.label(r, scene, &t(note), (14.0 * u) as u32, MUTED);
        nl.place(scene, mx - 40.0 * u * (1.0 - hs), y + hl.h as f32);
        y + hl.h as f32 + nl.h as f32 + 18.0 * u
    }
}
