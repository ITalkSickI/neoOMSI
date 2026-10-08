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
        let Metrics { w, h, u, mx, line } = m;
        self.pause_items.clear();
        let o = out(self.pause_open);
        self.text.alpha = o;
        let entries = self.pause_entries.clone();
        let n = entries.len().max(1);

        // the picture dims, a panel on the left carries the menu
        self.text.rounded(r, scene, [0.0, 0.0, w, h], 0.0, [6, 8, 12, 150]);
        let mx = (mx * 0.7).round();
        let row_w = 300.0 * u;
        let pw = mx + row_w + 8.0 * u;
        // it slides in from the left (and out again)
        let dx = -pw * (1.0 - o);
        self.text.rounded(r, scene, [dx, 0.0, pw + dx, h], 0.0, [8, 10, 14, 238]);
        self.text.rounded(r, scene, [pw - line + dx, 0.0, pw + dx, h], 0.0, BORDER);

        // the logo, then a short accent rule
        let mut y = (h * 0.06).round();
        let slide = dx;
        self.ensure_logo();
        if let Some((tex, iw, ih)) = self.logo_at(r, scene, (38.0 * u).round()) {
            if o > 0.05 {
                let x = (mx + slide).round();
                scene.overlays.push((tex, [x, y, x + iw as f32, y + ih as f32]));
            }
            y += ih as f32 + 12.0 * u;
        } else {
            let title = self.text.label(r, scene, "neoOMSI", (30.0 * u) as u32, WHITE);
            title.place(scene, mx + slide, y);
            y += title.h as f32 + 6.0 * u;
        }
        self.text.rounded(
            r,
            scene,
            [mx + dx, y, mx + 48.0 * u + dx, y + 3.0 * u],
            0.0,
            ACCENT,
        );
        y += 22.0 * u;

        // the entries: as large as the screen leaves room for
        let gap = 2.0 * u;
        let avail = (h - y - 28.0 * u).max(100.0 * u);
        let px = (((avail / n as f32) - gap - 10.0 * u) / 1.3).clamp(13.0 * u, 21.0 * u);
        for (i, name) in entries.iter().enumerate() {
            let e = 1.0;
            let l = self.text.label(
                r,
                scene,
                name,
                px as u32,
                if i == sel { WHITE } else { SOFT },
            );
            let row_h = l.h as f32 + 10.0 * u;
            let rc = [mx - 24.0 * u, y, mx - 24.0 * u + row_w, y + row_h];
            let hot = inside(rc, f.cursor);
            let hv = self.ease((200, "hover", i), if hot { 1.0 } else { 0.0 }, 8.0);
            let sv = self.ease((200, "sel", i), if i == sel { 1.0 } else { 0.0 }, 6.0);
            let ox = dx * e;
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
            l.place(scene, mx + ox + 10.0 * u * sv, y + 5.0 * u);
            self.pause_items.push(rc);
            y += row_h + gap;
        }
        self.text.alpha = 1.0;
    }
}
