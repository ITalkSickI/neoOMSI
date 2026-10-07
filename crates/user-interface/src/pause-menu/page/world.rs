//! World related stuff, like time, weather etc.

use super::*;

pub(super) const PAGE: Page = Page {
    nav: "pause.page.world.nav",
    draw: Ui::draw_world_page,
};

impl Ui {
    pub(super) fn draw_world_page(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        _f: &Frame,
        m: Metrics,
        top: f32,
        pt: f32,
    ) {
        self.draw_page_head(r, scene, m, "pause.page.world.head", "pause.page.world.note", top, pt);
    }
}