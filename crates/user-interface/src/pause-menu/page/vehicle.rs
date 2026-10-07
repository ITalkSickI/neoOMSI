//! Vehicle related things

use super::*;

pub(super) const PAGE: Page = Page {
    nav: "pause.page.vehicle.nav",
    draw: Ui::draw_vehicle_page,
};

fn group_title(g: &VehicleGroup) -> String {
    t(&format!("pause.page.vehicle.group.{}.title", g.id)).to_uppercase()
}

fn action_key(a: &VehicleAction, field: &str) -> String {
    format!("pause.page.vehicle.action.{}.{field}", a.id)
}

impl Ui {
    pub(super) fn draw_vehicle_page(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        f: &Frame,
        m: Metrics,
        top: f32,
        pt: f32,
    ) {
        let Metrics { w, h, u, mx, line } = m;
        let top = self.draw_page_head(
            r,
            scene,
            m,
            "pause.page.vehicle.head",
            "pause.page.vehicle.note",
            top,
            pt,
        );
        self.lab_groups.clear();
        self.lab_actions.clear();
        let groups = f.vehicle_menu;
        if groups.is_empty() {
            let l = self.text.label(
                r,
                scene,
                &t("pause.page.vehicle.empty"),
                (16.0 * u) as u32,
                MUTED,
            );
            l.place(scene, mx, top + 8.0 * u);
            return;
        }
        let gi = self.lab_group.min(groups.len() - 1);
        self.lab_group = gi;
        let foot_h = 34.0 * u;
        let bottom = h - foot_h;
        let gap = 28.0 * u;
        let inner_w = w - mx * 2.0;

        // left: the groups
        let side_w = (inner_w * 0.2).floor();
        let btn_h = 48.0 * u;
        let bpx = (16.0 * u) as u32;
        for (i, g) in groups.iter().enumerate() {
            let e = out((pt - 0.06 * i as f32 - 0.05) / 0.5);
            let y = top + (btn_h + 6.0 * u) * i as f32;
            if y + btn_h > bottom {
                break;
            }
            let rc = [
                mx - 20.0 * u * (1.0 - e),
                y,
                mx + side_w - 20.0 * u * (1.0 - e),
                y + btn_h,
            ];
            let hot = inside(rc, f.cursor) && self.dialog.is_none();
            let hv = self.ease((203, "grp", i), if hot { 1.0 } else { 0.0 }, 8.0);
            let sv = self.easeq((203, "grpsel", i), if i == gi { 1.0 } else { 0.0 }, 7.0);
            let bg = mix(
                mix([30, 32, 37, 255], [42, 45, 52, 255], hv),
                [58, 62, 70, 255],
                sv,
            );
            self.text.rounded(r, scene, rc, 0.0, fade(bg, e));
            if sv > 0.0 {
                self.text.rounded(
                    r,
                    scene,
                    [rc[0], rc[1], rc[0] + 4.0 * u, rc[3]],
                    0.0,
                    fade(ACCENT, e * sv),
                );
            }
            let name = clip_to(&self.text, &group_title(g), bpx as f32, side_w - 32.0 * u);
            let l = self.text.label(
                r,
                scene,
                &name,
                bpx,
                mix(if hot { WHITE } else { SOFT }, WHITE, sv),
            );
            l.place(scene, rc[0] + 16.0 * u, rc[1] + (btn_h - l.h as f32) * 0.5);
            self.lab_groups.push([mx, y, mx + side_w, y + btn_h]);
        }

        // middle: the rows
        let mid_x = mx + side_w + gap;
        let mid_w = ((inner_w - side_w - gap * 2.0) * 0.56).floor();
        let g = &groups[gi];
        let hpx = (22.0 * u) as u32;
        let hl = self.text.label(r, scene, &group_title(g), hpx, WHITE);
        hl.place(scene, mid_x, top);
        let y0 = top + hl.h as f32 + 12.0 * u;
        let row_h = 56.0 * u;
        let fit = (((bottom - y0) / row_h).floor().max(0.0)) as usize;
        let mut hovered: Option<usize> = None;
        for (i, a) in g.actions.iter().enumerate().take(fit) {
            let e = out((pt - 0.06 * i as f32 - 0.15) / 0.5);
            let ry = y0 + row_h * i as f32;
            let ox = 24.0 * u * (1.0 - e);
            let rc = [mid_x, ry, mid_x + mid_w, ry + row_h];
            let hot = inside(rc, f.cursor) && self.dialog.is_none();
            if hot {
                hovered = Some(i);
            }
            let hv = self.ease((204, "row", i), if hot { 1.0 } else { 0.0 }, 8.0);
            let base = if i % 2 == 0 {
                [28, 30, 35, 255]
            } else {
                [22, 24, 28, 255]
            };
            let rr = [rc[0] + ox, rc[1], rc[2] + ox, rc[3]];
            self.text
                .rounded(r, scene, rr, 0.0, fade(mix(base, [46, 49, 57, 255], hv), e));
            if hv > 0.0 {
                self.text.rounded(
                    r,
                    scene,
                    [rr[0] - line, rr[1], rr[2] + line, rr[1] + line],
                    0.0,
                    fade(ACCENT, e * hv),
                );
                self.text.rounded(
                    r,
                    scene,
                    [rr[0] - line, rr[3] - line, rr[2] + line, rr[3]],
                    0.0,
                    fade(ACCENT, e * hv),
                );
            }
            let vpx = (15.0 * u) as u32;
            let val = if a.opens {
                format!("{}  >", t("pause.page.vehicle.choose"))
            } else {
                t(&action_key(a, "button"))
            };
            let vw = self.text.width(&val, vpx as f32);
            let vl = self.text.label(r, scene, &val, vpx, mix(AMBER, WHITE, hv));
            vl.place(
                scene,
                rr[2] - 18.0 * u - vw,
                rr[1] + (row_h - vl.h as f32) * 0.5,
            );
            let npx = (18.0 * u) as u32;
            let name = clip_to(
                &self.text,
                &t(&action_key(a, "name")),
                npx as f32,
                mid_w - vw - 56.0 * u,
            );
            let nl = self
                .text
                .label(r, scene, &name, npx, if hot { WHITE } else { SOFT });
            nl.place(scene, rr[0] + 18.0 * u, rr[1] + (row_h - nl.h as f32) * 0.5);
            self.lab_actions.push(rc);
        }

        // right: what the row under the mouse does
        let px0 = mid_x + mid_w + gap;
        let rc = [px0, top, w - mx, (top + 300.0 * u).min(bottom)];
        if rc[2] - rc[0] > 120.0 * u && rc[3] - rc[1] > 100.0 * u {
            let e = out((pt - 0.3) / 0.5);
            self.text.rounded(
                r,
                scene,
                [rc[0] - line, rc[1] - line, rc[2] + line, rc[3] + line],
                0.0,
                fade(BORDER, e),
            );
            self.text
                .rounded(r, scene, rc, 0.0, fade([8, 10, 14, 255], e));
            let a = hovered.and_then(|i| g.actions.get(i));
            let (title, body) = match a {
                Some(a) => (
                    t(&action_key(a, "name")).to_uppercase(),
                    t(&action_key(a, "desc")),
                ),
                None => (group_title(g), t("pause.page.vehicle.hint")),
            };
            let inner = rc[2] - rc[0] - 40.0 * u;
            let tpx = (22.0 * u) as u32;
            let tl = self.text.label(
                r,
                scene,
                &clip_to(&self.text, &title, tpx as f32, inner),
                tpx,
                WHITE,
            );
            tl.place(scene, rc[0] + 20.0 * u, rc[1] + 20.0 * u);
            let mut ty = rc[1] + 20.0 * u + tl.h as f32 + 18.0 * u;
            let bpx = 15.0 * u;
            for ln in wrap(&self.text, &body, bpx, inner) {
                if ty + bpx * 1.3 > rc[3] - 16.0 * u {
                    break;
                }
                let l = self.text.label(r, scene, &ln, bpx as u32, SOFT);
                l.place(scene, rc[0] + 20.0 * u, ty);
                ty += l.h as f32 + 3.0 * u;
            }
            // dashed rule at the bottom of the panel
            let ry = rc[3] - 14.0 * u;
            let mut dx = rc[0] + 20.0 * u;
            while dx + 6.0 * u < rc[2] - 20.0 * u {
                self.text.rounded(
                    r,
                    scene,
                    [dx, ry, dx + 6.0 * u, ry + line],
                    0.0,
                    fade([255, 255, 255, 60], e),
                );
                dx += 10.0 * u;
            }
        }
    }
}
