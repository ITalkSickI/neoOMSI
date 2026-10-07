//! Game settings menu

use super::*;

pub(super) const PAGE: Page = Page {
    nav: "pause.page.options.nav",
    draw: Ui::draw_options_page,
};

impl Ui {
    pub(super) fn draw_options_page(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        _f: &Frame,
        m: Metrics,
        top: f32,
        pt: f32,
    ) {
        self.draw_page_head(r, scene, m, "pause.page.options.head", "pause.page.options.note", top, pt);
    }
}