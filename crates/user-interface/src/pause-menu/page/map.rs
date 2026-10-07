//! The map page (Nothing yet)

use super::*;

pub(super) const PAGE: Page = Page {
    nav: "pause.page.map.nav",
    draw: Ui::draw_map_page,
};

impl Ui {
    pub(super) fn draw_map_page(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        _f: &Frame,
        m: Metrics,
        top: f32,
        pt: f32,
    ) {
        self.draw_page_head(r, scene, m, "pause.page.map.head", "pause.page.map.note", top, pt);
    }
}