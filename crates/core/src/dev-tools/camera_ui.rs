#![allow(unused_imports)]
use crate::camera_tool::{self, CamCfg};
use imgui::Condition;

pub(super) fn window(ui: &imgui::Ui, open: &mut bool) {
    camera_tool::want(*open);
    if !*open {
        return;
    }
    let info = camera_tool::info();
    ui.window("Cameras")
        .opened(open)
        .size([440.0, 420.0], Condition::FirstUseEver)
        .position([12.0, 32.0], Condition::FirstUseEver)
        .build(|| {
            if info.is_empty() {
                ui.text_disabled("No reflection cameras (drive a vehicle with [add_camera_reflexion])");
                return;
            }
            ui.text_disabled(format!("{} reflection cameras (front, then coupled parts)", info.len()));
            let mut ov = camera_tool::overlay_on();
            if ui.checkbox("Show in world (point and camera axis)##camov", &mut ov) {
                camera_tool::set_overlay(ov);
            }
            let mut ovv = camera_tool::overlay_view();
            if ui.checkbox("Also the drawn mirror ray (follows your eye)##camovv", &mut ovv) {
                camera_tool::set_overlay_view(ovv);
            }
            ui.text_disabled("cyan: in view, grey: not, magenta: direct, yellow: mirror ray");
            if ui.button("Reset all##camall") {
                camera_tool::reset_all();
            }
            ui.separator();
            for (i, inf) in info.iter().enumerate() {
                let mut c = camera_tool::cfg(i);
                let part = if inf.part == 0 {
                    "front".to_string()
                } else {
                    format!("part {}", inf.part)
                };
                let label = format!(
                    "#{i} {part}{}{}##cam{i}",
                    if inf.seen { "" } else { " (not in view)" },
                    if c == CamCfg::DEFAULT { "" } else { " *" },
                );
                if !ui.collapsing_header(label, imgui::TreeNodeFlags::empty()) {
                    continue;
                }
                ui.text_disabled(format!(
                    "file: pos {:.2} {:.2} {:.2}  yaw {:.1}  pitch {:.1}  fov {:.0}  radius {:.2}",
                    inf.pos[0], inf.pos[1], inf.pos[2], inf.yaw, inf.pitch, inf.fov, inf.radius
                ));
                ui.text_disabled(format!(
                    "view: yaw {:.1}  pitch {:.1}  ({})",
                    inf.aimed_yaw,
                    inf.aimed_pitch,
                    if inf.direct { "direct" } else { "mirror" }
                ));
                ui.text_disabled(format!(
                    "eye (world): {:.2} {:.2} {:.2}",
                    inf.eye[0], inf.eye[1], inf.eye[2]
                ));
                if ui.radio_button_bool(format!("Mirror (yaw/pitch = face)##cm{i}"), !c.direct) {
                    c.direct = false;
                }
                if ui.radio_button_bool(format!("Direct (yaw/pitch = view)##cd{i}"), c.direct) {
                    c.direct = true;
                }
                ui.slider(format!("Yaw +##cy{i}"), -180.0, 180.0, &mut c.yaw);
                ui.slider(format!("Pitch +##cp{i}"), -90.0, 90.0, &mut c.pitch);
                ui.slider(format!("FOV (0 = file)##cf{i}"), 0.0, 120.0, &mut c.fov);
                ui.slider(format!("Right (m)##cx{i}"), -3.0, 3.0, &mut c.pos[0]);
                ui.slider(format!("Forward (m)##cz{i}"), -3.0, 3.0, &mut c.pos[1]);
                ui.slider(format!("Height (m)##ch{i}"), -3.0, 3.0, &mut c.pos[2]);
                ui.text_disabled(format!(
                    "result: pos {:.2} {:.2} {:.2}  yaw {:.1}  pitch {:.1}  fov {:.0}",
                    inf.pos[0] + c.pos[0],
                    inf.pos[1] + c.pos[1],
                    inf.pos[2] + c.pos[2],
                    inf.yaw + c.yaw,
                    inf.pitch + c.pitch,
                    if c.fov > 0.0 { c.fov } else { inf.fov }
                ));
                if ui.button(format!("Reset##cr{i}")) {
                    c = CamCfg::DEFAULT;
                }
                camera_tool::set_cfg(i, c);
            }
        });
}