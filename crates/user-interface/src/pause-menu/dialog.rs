//! Dialogs

use super::*;

pub enum Dialog {
    Confirm { title: String, text: String, yes: String, no: String },
    Select { title: String, options: Vec<String>, sel: usize },
    Editor { title: String, value: String, ok: String, cancel: String },
}

impl Ui {
    fn dialog_button(&mut self, r: &Renderer, scene: &mut Scene, f: &Frame, m: Metrics, rc: [f32; 4], text: &str, i: usize, main: bool) {
        let hot = inside(rc, f.cursor);
        let hv = self.ease((205, "btn", i), if hot { 1.0 } else { 0.0 }, 8.0);
        let base = if main { [58, 62, 70, 255] } else { [30, 32, 37, 255] };
        self.text.rounded(r, scene, rc, 0.0, mix(base, [74, 78, 88, 255], hv));
        if main {
            self.text.rounded(r, scene, [rc[0], rc[3] - 3.0 * m.u, rc[2], rc[3]], 0.0, ACCENT);
        }
        let px = (16.0 * m.u) as u32;
        let l = self.text.label(r, scene, text, px, if hot || main { WHITE } else { SOFT });
        l.place(scene, rc[0] + (rc[2] - rc[0] - l.w as f32) * 0.5, rc[1] + (rc[3] - rc[1] - l.h as f32) * 0.5);
    }

    pub(super) fn draw_dialog(&mut self, r: &Renderer, scene: &mut Scene, f: &Frame, m: Metrics, d: &Dialog) {
        let Metrics { w, h, u, line, .. } = m;
        self.dialog_rects.clear();
        self.text.rounded(r, scene, [0.0, 0.0, w, h], 0.0, [0, 0, 0, 160]);

        let pad = 24.0 * u;
        let bw = (460.0 * u).min(w - 2.0 * pad);
        let inner = bw - 2.0 * pad;
        let btn_h = 44.0 * u;
        let row_h = 44.0 * u;
        let tpx = (22.0 * u) as u32;
        let bpx = 16.0 * u;

        let (title, body_h) = match d {
            Dialog::Confirm { title, text, .. } => {
                (title, wrap(&self.text, text, bpx, inner).len() as f32 * (bpx * 1.3 + 3.0 * u) + btn_h + pad)
            }
            Dialog::Select { title, options, .. } => (title, options.len() as f32 * (row_h + 4.0 * u)),
            Dialog::Editor { title, .. } => (title, 48.0 * u + pad + btn_h),
        };
        let bh = pad * 2.0 + tpx as f32 * 1.3 + 16.0 * u + body_h;
        let (x, y) = ((w - bw) * 0.5, ((h - bh) * 0.5).max(0.0));
        self.text.rounded(r, scene, [x - line, y - line, x + bw + line, y + bh + line], 0.0, BORDER);
        self.text.rounded(r, scene, [x, y, x + bw, y + bh], 0.0, [14, 16, 20, 255]);
        self.text.rounded(r, scene, [x, y, x + bw, y + 3.0 * u], 0.0, ACCENT);

        let tl = self.text.label(r, scene, title, tpx, WHITE);
        tl.place(scene, x + pad, y + pad);
        let mut cy = y + pad + tl.h as f32 + 16.0 * u;
        let (bx, bw2) = (x + pad, (inner - 12.0 * u) * 0.5);

        match d {
            Dialog::Confirm { text, yes, no, .. } => {
                for ln in wrap(&self.text, text, bpx, inner) {
                    let l = self.text.label(r, scene, &ln, bpx as u32, SOFT);
                    l.place(scene, bx, cy);
                    cy += l.h as f32 + 3.0 * u;
                }
                cy += pad;
                let a = [bx, cy, bx + bw2, cy + btn_h];
                let b = [bx + bw2 + 12.0 * u, cy, bx + inner, cy + btn_h];
                self.dialog_button(r, scene, f, m, a, yes, 0, true);
                self.dialog_button(r, scene, f, m, b, no, 1, false);
                self.dialog_rects.extend([a, b]);
            }
            Dialog::Select { options, sel, .. } => {
                for (i, o) in options.iter().enumerate() {
                    let rc = [bx, cy, bx + inner, cy + row_h];
                    let hot = inside(rc, f.cursor);
                    let hv = self.ease((205, "opt", i), if hot { 1.0 } else { 0.0 }, 8.0);
                    let on = i == *sel;
                    let bg = mix(if on { [46, 49, 57, 255] } else { [28, 30, 35, 255] }, [46, 49, 57, 255], hv);
                    self.text.rounded(r, scene, rc, 0.0, bg);
                    if on {
                        self.text.rounded(r, scene, [rc[0], rc[1], rc[0] + 4.0 * u, rc[3]], 0.0, ACCENT);
                    }
                    let name = clip_to(&self.text, o, bpx, inner - 40.0 * u);
                    let l = self.text.label(r, scene, &name, bpx as u32, if on || hot { WHITE } else { SOFT });
                    l.place(scene, rc[0] + 18.0 * u, rc[1] + (row_h - l.h as f32) * 0.5);
                    self.dialog_rects.push(rc);
                    cy += row_h + 4.0 * u;
                }
            }
            Dialog::Editor { value, ok, cancel, .. } => {
                let rc = [bx, cy, bx + inner, cy + 48.0 * u];
                self.text.rounded(r, scene, [rc[0] - line, rc[1] - line, rc[2] + line, rc[3] + line], 0.0, ACCENT);
                self.text.rounded(r, scene, rc, 0.0, [8, 10, 14, 255]);
                let shown = clip_left(&self.text, value, bpx, inner - 32.0 * u);
                let sw = if shown.is_empty() { 0.0 } else { self.text.width(&shown, bpx) };
                if !shown.is_empty() {
                    let l = self.text.label(r, scene, &shown, bpx as u32, WHITE);
                    l.place(scene, rc[0] + 16.0 * u, rc[1] + (48.0 * u - l.h as f32) * 0.5);
                }
                if (self.text.frame / 30) % 2 == 0 {
                    let cx = rc[0] + 16.0 * u + sw + 2.0 * u;
                    self.text.rounded(r, scene, [cx, rc[1] + 12.0 * u, cx + 2.0 * u, rc[3] - 12.0 * u], 0.0, WHITE);
                }
                cy = rc[3] + pad;
                let a = [bx, cy, bx + bw2, cy + btn_h];
                let b = [bx + bw2 + 12.0 * u, cy, bx + inner, cy + btn_h];
                self.dialog_button(r, scene, f, m, a, ok, 0, true);
                self.dialog_button(r, scene, f, m, b, cancel, 1, false);
                self.dialog_rects.extend([a, b]);
            }
        }
    }
}
