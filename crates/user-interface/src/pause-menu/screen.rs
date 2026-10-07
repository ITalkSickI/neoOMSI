//! The full-screen pause screen

use super::*;

impl Ui {
    pub(super) fn draw_pause_screen(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        f: &Frame,
        m: Metrics,
        sel: usize,
    ) {
        let Metrics { w, h, u, mx, .. } = m;
        self.pause_items.clear();
        let o = out(self.pause_open);
        self.text
            .rounded(r, scene, [0.0, 0.0, w, h], 0.0, fade([6, 8, 12, 215], o));

        let mut y = (h * 0.12).round();
        let slide = -40.0 * u * (1.0 - o);
        let title = self.text.label(r, scene, &t("pause.title"), (72.0 * u) as u32, WHITE);
        title.place(scene, mx + slide, y);
        y += title.h as f32 + 6.0 * u;
        let rule = out(self.pause_open * 1.6 - 0.2);
        self.text.rounded(
            r,
            scene,
            [mx, y, mx + 80.0 * u * rule, y + 5.0 * u],
            0.0,
            ACCENT,
        );
        y += 48.0 * u;

        let px = (34.0 * u) as u32;
        let row_w = 420.0 * u;
        for (i, name) in PAUSE_ENTRIES.iter().enumerate() {
            let e = out((self.pause_open - 0.12 * i as f32 - 0.1) / 0.5);
            let text = t(name);
            let l = self.text.label(
                r,
                scene,
                &text,
                px,
                if i == sel { WHITE } else { SOFT },
            );
            let row_h = l.h as f32 + 16.0 * u;
            let rc = [mx - 24.0 * u, y, mx - 24.0 * u + row_w, y + row_h];
            let hot = inside(rc, f.cursor);
            let hv = self.ease((200, "hover", i), if hot { 1.0 } else { 0.0 }, 8.0);
            let sv = self.ease((200, "sel", i), if i == sel { 1.0 } else { 0.0 }, 6.0);
            let ox = -60.0 * u * (1.0 - e);
            if hv > 0.0 {
                let rr = [rc[0] + ox, rc[1], rc[2] + ox, rc[3]];
                self.text.rounded(r, scene, rr, 6.0 * u, fade(LIT, hv * e));
            }
            if sv > 0.0 {
                let mid = y + row_h * 0.5;
                let half = (row_h * 0.5 - 6.0 * u) * sv;
                self.text.rounded(
                    r,
                    scene,
                    [mx - 24.0 * u + ox, mid - half, mx - 20.0 * u + ox, mid + half],
                    0.0,
                    ACCENT,
                );
            }
            l.place(scene, mx + ox + 10.0 * u * sv, y + 8.0 * u);
            self.pause_items.push(rc);
            y += row_h + 6.0 * u;
        }
    }
}
