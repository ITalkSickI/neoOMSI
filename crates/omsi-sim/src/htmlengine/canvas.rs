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

    /// Put one picture pixel `s` (RGBA, straight alpha) at an in-bounds position, `cov`
    /// scaling its alpha (the rounded corners). Opaque pixels are plain copies.
    #[inline]
    pub(crate) fn put(&mut self, x: i32, y: i32, s: &[u8], cov: f32) {
        let a = s[3];
        if a == 0 || cov <= 0.0 {
            return;
        }
        let i = ((y as u32 * self.w + x as u32) * 4) as usize;
        if cov >= 0.999 && (a == 255 || self.px[i + 3] == 0) {
            self.px[i..i + 4].copy_from_slice(&s[..4]);
            return;
        }
        self.blend(x, y, [s[0], s[1], s[2], a], cov);
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

/// Coverage (0..1) of the pixel at (`x`, `y`) inside the rounded rectangle `r`.
fn corner_cov(r: [f32; 4], rad: f32, x: i32, y: i32) -> f32 {
    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
    let cx = px.max(r[0] + rad).min(r[0] + r[2] - rad);
    let cy = py.max(r[1] + rad).min(r[1] + r[3] - rad);
    let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    (rad - d + 0.5).clamp(0.0, 1.0)
}

/// Draw `img` inside `clip` (rounded by `radius`), its top-left corner at (`ox`, `oy`),
/// tiled along the axes that repeat. Only the pixels of the clip are visited.
fn draw_tiles(cv: &mut Canvas, img: &Img, clip: [f32; 4], radius: f32, ox: i32, oy: i32, rep: (bool, bool)) {
    if !clip.iter().all(|v| v.is_finite()) || !radius.is_finite() {
        return;
    }
    let (iw, ih) = (img.w as i32, img.h as i32);
    let mut x0 = (clip[0].round() as i32).max(0);
    let mut y0 = (clip[1].round() as i32).max(0);
    let mut x1 = ((clip[0] + clip[2]).round() as i32).min(cv.w as i32);
    let mut y1 = ((clip[1] + clip[3]).round() as i32).min(cv.h as i32);
    if !rep.0 {
        x0 = x0.max(ox);
        x1 = x1.min(ox.saturating_add(iw));
    }
    if !rep.1 {
        y0 = y0.max(oy);
        y1 = y1.min(oy.saturating_add(ih));
    }
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let rad = radius.min(clip[2] / 2.0).min(clip[3] / 2.0).max(0.0);
    let round = rad > 0.5;
    let (cl, cr) = (clip[0] + rad, clip[0] + clip[2] - rad);
    let (ct, cb) = (clip[1] + rad, clip[1] + clip[3] - rad);
    let tx0 = (x0 - ox).rem_euclid(iw);
    for y in y0..y1 {
        let ty = (y - oy).rem_euclid(ih);
        let row = ty as usize * iw as usize * 4;
        let ycorner = round && ((y as f32) < ct || (y as f32 + 1.0) > cb);
        let mut tx = tx0;
        for x in x0..x1 {
            let i = row + tx as usize * 4;
            let cov = if ycorner && ((x as f32) < cl || (x as f32 + 1.0) > cr) { corner_cov(clip, rad, x, y) } else { 1.0 };
            cv.put(x, y, &img.rgba[i..i + 4], cov);
            tx += 1;
            if tx == iw {
                tx = 0;
            }
        }
    }
}

/// The `background-image` of `st` over `rect` (size, repeat and position as CSS has them).
pub(crate) fn paint_bg(cv: &mut Canvas, imgs: &ImageStore, rect: [f32; 4], radius: f32, st: &Style) {
    let Some(src) = &st.bg_img else { return };
    let (rw, rh) = (rect[2], rect[3]);
    if !rect.iter().all(|v| v.is_finite()) || rw < 1.0 || rh < 1.0 {
        return;
    }
    let Some((iw, ih)) = imgs.dims(src) else { return };
    let (iw, ih) = (iw as f32, ih as f32);
    let (tw, th) = match st.bg_size {
        BgSize::Auto => (iw, ih),
        BgSize::Cover => {
            let k = (rw / iw).max(rh / ih);
            (iw * k, ih * k)
        }
        BgSize::Contain => {
            let k = (rw / iw).min(rh / ih);
            (iw * k, ih * k)
        }
        BgSize::Dims(w, h) => match (w.map(|l| l.px(rw)), h.map(|l| l.px(rh))) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => (w, w * ih / iw),
            (None, Some(h)) => (h * iw / ih, h),
            (None, None) => (iw, ih),
        },
    };
    if !tw.is_finite() || !th.is_finite() {
        return;
    }
    let (tw, th) = ((tw.round() as i64).clamp(1, 16384) as u32, (th.round() as i64).clamp(1, 16384) as u32);
    let Some(tile) = imgs.scaled(src, tw, th) else { return };
    let ox = rect[0] + st.bg_pos[0].px(rw - tw as f32);
    let oy = rect[1] + st.bg_pos[1].px(rh - th as f32);
    if !ox.is_finite() || !oy.is_finite() {
        return;
    }
    draw_tiles(cv, &tile, rect, radius, ox.round() as i32, oy.round() as i32, st.bg_repeat);
}

/// An `<img>`: the picture stretched over the content box of the element.
fn paint_img(cv: &mut Canvas, lay: &Layouter, b: &LBox) {
    let n = &lay.dom.nodes[b.node];
    if n.tag != "img" || n.src.is_empty() {
        return;
    }
    let [pt, pr, pb, pl] = b.st.padding;
    let area = [b.rect[0] + pl, b.rect[1] + pt, b.rect[2] - pl - pr, b.rect[3] - pt - pb];
    if !area.iter().all(|v| v.is_finite()) || area[2] < 0.5 || area[3] < 0.5 {
        return;
    }
    let w = (area[2].round() as i64).clamp(1, 16384) as u32;
    let h = (area[3].round() as i64).clamp(1, 16384) as u32;
    let Some(pic) = lay.imgs.scaled(&n.src, w, h) else { return };
    draw_tiles(cv, &pic, area, b.st.radius, area[0].round() as i32, area[1].round() as i32, (false, false));
}

pub(crate) fn paint(cv: &mut Canvas, lay: &Layouter, b: &LBox) {
    if !b.st.hidden {
        if b.st.bg[3] > 0 {
            cv.fill(b.rect, b.st.bg, b.st.radius);
        }
        if b.st.bg_img.is_some() {
            paint_bg(cv, lay.imgs, b.rect, b.st.radius, &b.st);
        }
        paint_img(cv, lay, b);
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
