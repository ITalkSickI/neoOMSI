//! The pixel canvas and the painter that draws laid-out boxes and text into it.

use super::*;

pub(crate) struct Canvas {
    pub(crate) w: u32,
    pub(crate) h: u32,
    pub(crate) px: Vec<u8>,
}

impl Canvas {
    pub(crate) fn blend(&mut self, x: i32, y: i32, c: [u8; 4], cov: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = ((y as u32 * self.w + x as u32) * 4) as usize;
        let sa = c[3] as f32 / 255.0 * cov.clamp(0.0, 1.0);
        if sa <= 0.0 {
            return;
        }
        let da = self.px[i + 3] as f32 / 255.0;
        let oa = sa + da * (1.0 - sa);
        for k in 0..3 {
            let v = (c[k] as f32 * sa + self.px[i + k] as f32 * da * (1.0 - sa)) / oa;
            self.px[i + k] = v.round().clamp(0.0, 255.0) as u8;
        }
        self.px[i + 3] = (oa * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    pub(crate) fn fill(&mut self, r: [f32; 4], c: [u8; 4], radius: f32) {
        if !r.iter().all(|v| v.is_finite()) || !radius.is_finite() {
            return;
        }
        let (x0, y0) = (r[0].round() as i32, r[1].round() as i32);
        let (x1, y1) = ((r[0] + r[2]).round() as i32, (r[1] + r[3]).round() as i32);
        let rad = radius.min(r[2] / 2.0).min(r[3] / 2.0).max(0.0);
        for y in y0.max(0)..y1.min(self.h as i32) {
            for x in x0.max(0)..x1.min(self.w as i32) {
                let mut cov = 1.0;
                if rad > 0.5 {
                    let px = x as f32 + 0.5;
                    let py = y as f32 + 0.5;
                    let (cx_lo, cx_hi) = {
                        let a = r[0] + rad;
                        let b = r[0] + r[2] - rad;
                        (a.min(b), a.max(b))
                    };
                    let (cy_lo, cy_hi) = {
                        let a = r[1] + rad;
                        let b = r[1] + r[3] - rad;
                        (a.min(b), a.max(b))
                    };
                    let cx = px.clamp(cx_lo, cx_hi);
                    let cy = py.clamp(cy_lo, cy_hi);
                    let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                    cov = (rad - d + 0.5).clamp(0.0, 1.0);
                }
                self.blend(x, y, c, cov);
            }
        }
    }
}

pub(crate) fn draw_text(cv: &mut Canvas, font: &FontRef<'static>, px: f32, x: f32, base_y: f32, color: [u8; 4], s: &str) {
    let sc = PxScale::from(px);
    let sf = font.as_scaled(sc);
    let mut cx = x;
    let mut prev = None;
    for ch in s.chars() {
        if ch == '\n' {
            continue;
        }
        let id = font.glyph_id(ch);
        if let Some(p) = prev {
            cx += sf.kern(p, id);
        }
        let g = id.with_scale_and_position(sc, point(cx, base_y));
        if let Some(o) = font.outline_glyph(g) {
            let b = o.px_bounds();
            let (bx, by) = (b.min.x as i32, b.min.y as i32);
            o.draw(|gx, gy, cov| cv.blend(bx + gx as i32, by + gy as i32, color, cov));
        }
        cx += sf.h_advance(id);
        prev = Some(id);
    }
}

pub(crate) fn paint(cv: &mut Canvas, lay: &Layouter, b: &LBox) {
    if !b.st.hidden && b.st.bg[3] > 0 {
        cv.fill(b.rect, b.st.bg, b.st.radius);
    }
    for it in &b.items {
        match it {
            Item::Block(c) => paint(cv, lay, c),
            Item::Line(l) => {
                if b.st.hidden {
                    continue;
                }
                for li in &l.items {
                    let font = if li.bold { lay.bold } else { lay.reg };
                    let sf = font.as_scaled(PxScale::from(li.px));
                    let base = l.y + (l.h - (sf.ascent() - sf.descent())) / 2.0 + sf.ascent();
                    draw_text(cv, font, li.px, l.x + li.dx, base, li.color, &li.text);
                }
            }
        }
    }
}
