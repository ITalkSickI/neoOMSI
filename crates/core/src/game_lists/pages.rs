//! Pages of rows for the options, vehicle and world windows.

use super::*;

fn tx(key: &str) -> String {
    ::i18n::translate(key, &[])
}

pub(crate) fn row(name: &str, kind: char, value: &str, desc: &str, frac: Option<f32>) -> String {
    format!(
        "{name}\u{1f}{kind}\u{1f}{value}\u{1f}{desc}\u{1f}{}",
        frac.map(|f| format!("{f:.3}")).unwrap_or_default()
    )
}

pub(crate) fn opens(name: &str, desc: &str, id: &str) -> (String, String) {
    (row(name, 'o', "", desc, None), id.to_string())
}

pub(crate) fn button(name: &str, text: &str, desc: &str, id: &str) -> (String, String) {
    (row(name, 'a', text, desc, None), id.to_string())
}

pub(crate) fn switch_row(app: &App, id: &str, name: &str, desc: &str) -> Option<(String, String)> {
    let on = toggle_now(app, id)?;
    Some((
        row(name, 's', if on { "on" } else { "off" }, desc, None),
        id.to_string(),
    ))
}

pub(crate) fn slider_row(
    app: &App,
    id: &str,
    name: &str,
    desc: &str,
    fmt: &dyn Fn(f32) -> String,
) -> Option<(String, String)> {
    let (verb, arg) = id.split_once(' ').unwrap_or((id, ""));
    let steps = steps_of(verb)?;
    let now = option_now(app, verb, arg)?;
    let i = nearest(&steps, now);
    let frac = if steps.len() > 1 {
        i as f32 / (steps.len() - 1) as f32
    } else {
        0.0
    };
    Some((row(name, 'v', &fmt(now), desc, Some(frac)), id.to_string()))
}

pub(crate) const MAP_TAB: usize = 99;
pub(crate) const KEYS_TAB: usize = 98;
pub(crate) const LOOK_TAB: usize = 97;

pub(crate) fn is_sub_tab(t: usize) -> bool {
    t == MAP_TAB || t == KEYS_TAB || t == LOOK_TAB
}
pub(super) fn look_options_page(app: &App) -> Page {
    let rows: Vec<(String, String)> = vec![
        switch_row(
            app,
            "free_look",
            &tx("pause.options.free_look.free_look.name"),
            &tx("pause.options.free_look.free_look.desc"),
        ),
        switch_row(
            app,
            "crosshair",
            &tx("pause.options.free_look.crosshair.name"),
            &tx("pause.options.free_look.crosshair.desc"),
        ),
        switch_row(
            app,
            "tooltips",
            &tx("pause.options.free_look.tooltips.name"),
            &tx("pause.options.free_look.tooltips.desc"),
        ),
    ]
        .into_iter()
        .flatten()
        .collect();
    (tx("pause.options.group.free_look"), rows)
}
pub(super) fn map_options_page(app: &App) -> Page {
    let file = settings_file();
    let rows: Vec<(String, String)> = vec![
        switch_row(app, "navigator", &tx("pause.options.group.map"), &tx("pause.page.text.enables_disables_the_minimap")),
        switch_row(
            app,
            "nav_topbar",
            &tx("pause.options.map.nav_topbar.name"),
            &tx("pause.options.map.nav_topbar.desc"),
        ),
        switch_row(
            app,
            "nav_turn",
            &tx("pause.options.map.nav_turn.name"),
            &tx("pause.options.map.nav_turn.desc"),
        ),
        switch_row(
            app,
            "nav_stoplist",
            &tx("pause.options.map.nav_stoplist.name"),
            &tx("pause.options.map.nav_stoplist.desc"),
        ),
        switch_row(
            app,
            "nav_stops_ext",
            &tx("pause.options.map.nav_stops_ext.name"),
            &tx("pause.options.map.nav_stops_ext.desc"),
        ),
        switch_row(
            app,
            "nav_ai",
            &tx("pause.options.map.nav_ai.name"),
            &tx("pause.options.map.nav_ai.desc"),
        ),
        select_row(
            &file,
            "navigator_corner",
            &tx("pause.options.map.navigator_corner.name"),
            &tx("pause.options.later"),
        ),
    ]
        .into_iter()
        .flatten()
        .collect();
    (tx("pause.options.group.map"), rows)
}

pub(super) fn options_pages(app: &App) -> Vec<Page> {
    let file = settings_file();
    let pick = |key: &str, name: &str, desc: &str| select_row(&file, key, name, desc);
    let pct = |v: f32| format!("{:.0} %", v * 100.0);
    let cm = |v: f32| format!("{:+.0} cm", v * 100.0);
    let later_text = tx("pause.options.later");
    let later = later_text.as_str();
    let game: Vec<(String, String)> = vec![
        switch_row(
            app,
            "auto_ibis",
            &tx("pause.options.gameplay.auto_ibis.name"),
            &tx("pause.options.gameplay.auto_ibis.desc"),
        ),
        switch_row(
            app,
            "exact_fare",
            &tx("pause.options.gameplay.exact_fare.name"),
            &tx("pause.options.gameplay.exact_fare.desc"),
        ),
        pick("pax_motion", &tx("pause.options.gameplay.pax_motion.name"), &tx("pause.options.gameplay.pax_motion.desc")),
        switch_row(
            app,
            "pax_ik",
            &tx("pause.options.gameplay.pax_ik.name"),
            &tx("pause.options.gameplay.pax_ik.desc"),
        ),
        pick("pax_models", &tx("pause.options.gameplay.pax_models.name"), &tx("pause.options.gameplay.pax_models.desc")),
        pick("boarding", &tx("pause.options.gameplay.boarding.name"), &tx("pause.options.gameplay.boarding.desc")),
        switch_row(
            app,
            "pax_prefer_seats",
            &tx("pause.options.gameplay.pax_prefer_seats.name"),
            &tx("pause.options.gameplay.pax_prefer_seats.desc"),
        ),
        switch_row(
            app,
            "pax_rear_entry",
            &tx("pause.page.text.boarding_at_the_rear_doors"),
            &tx("pause.page.text.passengers_who_need_no_ticket_from_the_driver_also_get_on_at_the_rear_doors"),
        ),
        pick("maintenance", &tx("pause.options.gameplay.maintenance.name"), later),
        switch_row(
            app,
            "coll_objects",
            &tx("pause.options.gameplay.coll_objects.name"),
            &tx("pause.options.gameplay.coll_objects.desc"),
        ),
        switch_row(
            app,
            "coll_vehicles",
            &tx("pause.options.gameplay.coll_vehicles.name"),
            &tx("pause.page.text.enables_disables_collisions_with_other_vehicles"),
        ),
        switch_row(
            app,
            "collision_pedestrians",
            &tx("pause.options.gameplay.collision_pedestrians.name"),
            &tx("pause.options.gameplay.collision_pedestrians.desc"),
        ),
        pick("ai_unsched_factor", &tx("pause.options.gameplay.ai_unsched_factor.name"), later),
        pick("ai_max_scheduled", &tx("pause.options.gameplay.ai_max_scheduled.name"), later),
        pick("ai_max_parked", &tx("pause.options.gameplay.ai_max_parked.name"), later),
    ]
        .into_iter()
        .flatten()
        .collect();
    let driving: Vec<(String, String)> = vec![
        switch_row(app, "auto_clutch", &tx("pause.options.driving.auto_clutch.name"), &tx("pause.options.driving.auto_clutch.desc")),
        switch_row(app, "auto_shift", &tx("pause.options.driving.auto_shift.name"), &tx("pause.options.driving.auto_shift.desc")),
        switch_row(app, "momentary_gears", &tx("pause.options.driving.momentary_gears.name"), later),
        switch_row(app, "brake_hold", &tx("pause.options.driving.brake_hold.name"), &tx("pause.options.driving.brake_hold.desc")),
        switch_row(app, "blinker_cancel", &tx("pause.options.driving.blinker_cancel.name"), &tx("pause.options.driving.blinker_cancel.desc")),
        switch_row(app, "steering_linear", &tx("pause.options.driving.steering_linear.name"), &tx("pause.options.driving.steering_linear.desc")),
        switch_row(app, "old_steering", &tx("pause.options.driving.old_steering.name"), &tx("pause.options.driving.old_steering.desc")),
        switch_row(app, "red_steer_spd", &tx("pause.options.driving.red_steer_spd.name"), &tx("pause.options.driving.red_steer_spd.desc")),
    ]
        .into_iter()
        .flatten()
        .collect();
    let controls: Vec<(String, String)> = vec![
        Some(opens(
            &tx("pause.options.controls.keybinds.name"),
            &tx("pause.options.controls.keybinds.desc"),
            "keysopts",
        )),
        switch_row(
            app,
            "mouse",
            &tx("pause.options.controls.mouse.name"),
            &tx("pause.options.controls.mouse.desc"),
        ),
        switch_row(
            app,
            "mouse_right",
            &tx("pause.options.controls.mouse_right.name"),
            &tx("pause.options.controls.mouse_right.desc"),
        ),
        slider_row(
            app,
            "mouse_sens",
            &tx("pause.options.controls.mouse_sens.name"),
            &tx("pause.options.controls.mouse_sens.desc"),
            &pct,
        ),
        slider_row(
            app,
            "stick_sens",
            &tx("pause.options.controls.stick_sens.name"),
            &tx("pause.options.controls.stick_sens.desc"),
            &pct,
        ),
        switch_row(
            app,
            "steer_center",
            &tx("pause.options.controls.steer_center.name"),
            &tx("pause.options.controls.steer_center.desc"),
        ),
        slider_row(
            app,
            "pedal_t",
            &tx("pause.options.controls.pedal_t.name"),
            &tx("pause.options.controls.pedal_t.desc"),
            &|v| format!("x{v}"),
        ),
        slider_row(
            app,
            "pedal_b",
            &tx("pause.options.controls.pedal_b.name"),
            &tx("pause.options.controls.pedal_b.desc"),
            &|v| format!("x{v}"),
        ),
        slider_row(
            app,
            "wheel_range",
            &tx("pause.options.controls.wheel_range.name"),
            &tx("pause.options.controls.wheel_range.desc"),
            &|v| format!("{v:.0}°"),
        ),
        slider_row(
            app,
            "wheel_lock",
            &tx("pause.options.controls.wheel_lock.name"),
            &tx("pause.options.controls.wheel_lock.desc"),
            &|v| {
                if v < 45.0 {
                    "OMSI".to_string()
                } else {
                    format!("{v:.0}°")
                }
            },
        ),
        switch_row(
            app,
            "ff",
            &tx("pause.options.controls.ff.name"),
            &tx("pause.options.controls.ff.desc"),
        ),
        switch_row(
            app,
            "ff_invert",
            &tx("pause.options.controls.ff_invert.name"),
            &tx("pause.options.controls.ff_invert.desc"),
        ),
    ]
        .into_iter()
        .flatten()
        .collect();
    let mut camera: Vec<(String, String)> = vec![
        slider_row(
            app,
            "fov",
            &tx("pause.options.camera.fov.name"),
            &tx("pause.options.camera.fov.desc"),
            &|v| {
                if v < 20.0 {
                    tx("pause.options.fmt.default")
                } else {
                    format!("{v:.0}°")
                }
            },
        ),
        switch_row(
            app,
            "head",
            &tx("pause.options.camera.head.name"),
            &tx("pause.options.camera.head.desc"),
        ),
        switch_row(
            app,
            "cam_smooth",
            &tx("pause.options.camera.cam_smooth.name"),
            &tx("pause.options.camera.cam_smooth.desc"),
        ),
        switch_row(
            app,
            "camcoll",
            &tx("pause.options.camera.camcoll.name"),
            &tx("pause.options.camera.camcoll.desc"),
        ),
        slider_row(
            app,
            "look_sens",
            &tx("pause.options.camera.look_sens.name"),
            &tx("pause.options.camera.look_sens.desc"),
            &pct,
        ),
        switch_row(
            app,
            "alt_view",
            &tx("pause.options.camera.alt_view.name"),
            &tx("pause.options.camera.alt_view.desc"),
        ),
        toggle_now(app, "free_look").map(|on| {
            (
                row(
                    &tx("pause.options.free_look.free_look.name"),
                    'm',
                    if on { "on" } else { "off" },
                    &tx("pause.options.camera.free_look.desc"),
                    None,
                ),
                "lookopts".to_string(),
            )
        }),
        switch_row(
            app,
            "steer_look",
            &tx("pause.options.camera.steer_look.name"),
            &tx("pause.options.camera.steer_look.desc"),
        ),
        slider_row(
            app,
            "steer_look_angle",
            &tx("pause.options.camera.steer_look_angle.name"),
            &tx("pause.options.camera.steer_look_angle.desc"),
            &|v| format!("{v:.0}°"),
        ),
        slider_row(
            app,
            "steer_look_response",
            &tx("pause.options.camera.steer_look_response.name"),
            &tx("pause.options.camera.steer_look_response.desc"),
            &|v| format!("{:.0} ms", v * 1000.0),
        ),
        switch_row(
            app,
            "headtrack",
            &tx("pause.options.camera.headtrack.name"),
            &::i18n::translate(
                "pause.options.camera.headtrack.desc_port",
                &[("port", &::config::get_int("camera", "head_tracking_port").and_then(|v| u16::try_from(v).ok()).unwrap_or(4242))],
            ),
        ),
        switch_row(
            app,
            "hands_in_cab",
            &tx("pause.options.camera.hands_in_cab.name"),
            &tx("pause.options.camera.hands_in_cab.desc"),
        ),
        switch_row(
            app,
            "driver",
            &tx("pause.options.camera.driver.name"),
            &tx("pause.options.camera.driver.desc"),
        ),
        slider_row(
            app,
            "seat 1",
            &tx("pause.options.camera.seat_1.name"),
            &tx("pause.options.camera.seat_1.desc"),
            &cm,
        ),
        slider_row(
            app,
            "seat 2",
            &tx("pause.options.camera.seat_2.name"),
            &tx("pause.options.camera.seat_2.desc"),
            &cm,
        ),
        slider_row(
            app,
            "seat 0",
            &tx("pause.options.camera.seat_0.name"),
            &tx("pause.options.camera.seat_0.desc"),
            &cm,
        ),
    ]
        .into_iter()
        .flatten()
        .collect();
    camera.push(button(
        &tx("pause.options.camera.seat_reset.name"),
        &tx("pause.options.button.reset"),
        &tx("pause.options.camera.seat_reset.desc"),
        "seat_reset",
    ));
    let graphics: Vec<(String, String)> = vec![
        preset_row(
            &tx("pause.options.graphics.preset.name"),
            &tx("pause.options.graphics.preset.desc"),
        ),
        (!::config::get_subs("graphics_profiles").is_empty()).then(|| {
            opens(
                &tx("pause.options.graphics.gfxprofile.name"),
                &tx("pause.options.graphics.gfxprofile.desc"),
                "gfxprofile",
            )
        }),
        pick("graphics", &tx("pause.options.graphics.graphics.name"), later),
        pick("msaa", &tx("pause.options.graphics.msaa.name"), later),
        pick("render_scale", &tx("pause.options.graphics.render_scale.name"), later),
        pick("anisotropy", &tx("pause.options.graphics.anisotropy.name"), later),
        switch_row(app, "shadows", &tx("pause.options.graphics.shadows.name"), &tx("pause.page.text.enables_disabled_shadows")),
        pick("shadow_size", &tx("pause.options.graphics.shadow_size.name"), later),
        pick("shadow_casters", &tx("pause.options.graphics.shadow_casters.name"), later),
        switch_row(app, "ssao", &tx("pause.options.graphics.ssao.name"), later),
        switch_row(
            app,
            "reflections",
            &tx("pause.options.graphics.reflections.name"),
            later,
        ),
        switch_row(app, "clouds", &tx("pause.options.graphics.clouds.name"), later),
        switch_row(
            app,
            "detail_textures",
            &tx("pause.options.graphics.detail_textures.name"),
            &tx("pause.options.graphics.detail_textures.desc"),
        ),
        pick("map_detail", &tx("pause.options.graphics.map_detail.name"), later),
        pick("view_distance", &tx("pause.options.graphics.view_distance.name"), later),
        pick("max_obj_dist", &tx("pause.options.graphics.max_obj_dist.name"), later),
        pick("min_obj_size", &tx("pause.options.graphics.min_obj_size.name"), later),
        pick("mirror_size", &tx("pause.options.graphics.mirror_size.name"), later),
        pick("texture_memory", &tx("pause.options.graphics.texture_memory.name"), later),
        switch_row(
            app,
            "texture_compression",
            &tx("pause.options.graphics.texture_compression.name"),
            later,
        ),
        slider_row(
            app,
            "led_glow",
            &tx("pause.options.graphics.led_glow.name"),
            &tx("pause.options.graphics.led_glow.desc"),
            &|v| format!("{}/15", v as i64),
        ),
        slider_row(
            app,
            "nightmap_glow",
            &tx("pause.options.graphics.nightmap_glow.name"),
            &tx("pause.options.graphics.nightmap_glow.desc"),
            &|v| format!("{}/15", v as i64),
        ),
        slider_row(
            app,
            "atmosphere_brightness",
            &tx("pause.options.graphics.atmosphere_brightness.name"),
            &tx("pause.options.graphics.atmosphere_brightness.desc"),
            &|v| format!("{v:.2}"),
        ),
        slider_row(
            app,
            "led_mips",
            &tx("pause.options.graphics.led_mips.name"),
            &tx("pause.options.graphics.led_mips.desc"),
            &|v| format!("{v:.2}"),
        ),
    ]
        .into_iter()
        .flatten()
        .collect();
    let display: Vec<(String, String)> = vec![
        pick("window_mode", &tx("pause.options.display.window_mode.name"), &tx("pause.options.display.window_mode.desc")),
        switch_row(app, "vsync", &tx("pause.options.display.vsync.name"), &tx("pause.options.display.vsync.desc")),
        pick("max_fps", &tx("pause.options.display.max_fps.name"), &tx("pause.options.display.max_fps.desc")),
        switch_row(
            app,
            "fps",
            &tx("pause.options.display.fps.name"),
            &tx("pause.options.display.fps.desc"),
        ),
    ]
        .into_iter()
        .flatten()
        .collect();
    let sound: Vec<(String, String)> = vec![
        slider_row(
            app,
            "volume",
            &tx("pause.options.sound.volume.name"),
            &tx("pause.options.sound.volume.desc"),
            &pct,
        ),
        slider_row(
            app,
            "vol_ai",
            &tx("pause.options.sound.vol_ai.name"),
            &tx("pause.options.sound.vol_ai.desc"),
            &pct,
        ),
        slider_row(
            app,
            "vol_scenery",
            &tx("pause.options.sound.vol_scenery.name"),
            &tx("pause.options.sound.vol_scenery.desc"),
            &pct,
        ),
        switch_row(
            app,
            "doppler",
            &tx("pause.options.sound.doppler.name"),
            &tx("pause.options.sound.doppler.desc"),
        ),
        pick("pax_voices", &tx("pause.options.sound.pax_voices.name"), &tx("pause.options.sound.pax_voices.desc")),
    ]
        .into_iter()
        .flatten()
        .collect();
    let interface: Vec<(String, String)> = vec![
        pick("language", &tx("pause.options.interface.language.name"), &tx("pause.options.interface.language.desc")),
        pick("units", &tx("pause.options.interface.units.name"), &tx("pause.options.interface.units.desc")),
        slider_row(app, "ui_scale", &tx("pause.options.interface.ui_scale.name"), &tx("pause.options.interface.ui_scale.desc"), &pct),
        switch_row(app, "ui_scale_window", &tx("pause.options.interface.ui_scale_window.name"), &tx("pause.options.interface.ui_scale_window.desc")),
        slider_row(app, "ui_opacity", &tx("pause.options.interface.ui_opacity.name"), &tx("pause.options.interface.ui_opacity.desc"), &pct),
        toggle_now(app, "navigator").map(|on| (row(&tx("pause.options.vr.navigator.name"), 'm', if on { "on" } else { "off" }, &tx("pause.options.interface.navigator.desc"), None), "mapopts".to_string())),
        switch_row(app, "nav_arrows", &tx("pause.options.interface.nav_arrows.name"), &tx("pause.options.interface.nav_arrows.desc")),
        switch_row(app, "info_bar", &tx("pause.options.interface.info_bar.name"), &tx("pause.options.interface.info_bar.desc")),
        switch_row(app, "timetable_win", &tx("pause.options.interface.timetable_win.name"), &tx("pause.options.interface.timetable_win.desc")),
        switch_row(app, "notes", &tx("pause.options.interface.notes.name"), &tx("pause.options.interface.notes.desc")),
        switch_row(app, "tooltips", &tx("pause.options.interface.tooltips.name"), &tx("pause.options.interface.tooltips.desc")),
        switch_row(app, "chat", &tx("pause.options.interface.chat.name"), &tx("pause.options.interface.chat.desc")),
        switch_row(app, "name_tags", &tx("pause.options.interface.name_tags.name"), &tx("pause.options.interface.name_tags.desc")),
        Some(opens(&tx("pause.options.interface.reset.name"), &tx("pause.options.interface.reset.desc"), "reset")),
    ]
        .into_iter()
        .flatten()
        .collect();
    let mut vr: Vec<(String, String)> = Vec::new();
    let vr_on = ::config::get_bool("vr", "enabled").unwrap_or(false);
    if cfg!(windows) {
        vr.extend(
            vec![
                switch_row(app, "vr", &tx("pause.options.vr.vr.name"), later),
                if vr_on {
                    pick("vr_scale", &tx("pause.options.vr.vr_scale.name"), later)
                } else {
                    None
                },
                if vr_on {
                    pick("vr_head_smoothing_ms", &tx("pause.options.vr.vr_head_smoothing_ms.name"), later)
                } else {
                    None
                },
                if vr_on {
                    pick("vr_mirror_rate", &tx("pause.options.vr.vr_mirror_rate.name"), later)
                } else {
                    None
                },
                if vr_on {
                    switch_row(
                        app,
                        "vr_desktop_mirror",
                        &tx("pause.options.vr.vr_desktop_mirror.name"),
                        later,
                    )
                } else {
                    None
                },
            ]
                .into_iter()
                .flatten(),
        );
    }
    if app.vr_active() && app.player.is_some() {
        let desc = &tx("pause.options.vr.navigator.desc");
        vr.extend(switch_row(app, "navigator", &tx("pause.options.vr.navigator.name"), desc));
        vr.push(button(
            &tx("pause.options.vr.vr_nav_edit.name"),
            &tx("pause.options.button.open"),
            desc,
            "vr_nav_edit",
        ));
        for (id, label) in [
            ("x", &tx("pause.options.vr.vr_nav_x.name")),
            ("y", &tx("pause.options.vr.vr_nav_y.name")),
            ("z", &tx("pause.options.vr.vr_nav_z.name")),
            ("width", &tx("pause.options.vr.vr_nav_width.name")),
        ] {
            vr.extend(slider_row(app, &format!("vr_nav_{id}"), label, desc, &cm));
        }
        for (id, label) in [
            ("yaw", &tx("pause.options.vr.vr_nav_yaw.name")),
            ("tilt", &tx("pause.options.vr.vr_nav_tilt.name")),
            ("roll", &tx("pause.options.vr.vr_nav_roll.name")),
        ] {
            vr.extend(slider_row(
                app,
                &format!("vr_nav_{id}"),
                label,
                desc,
                &|v| format!("{v:.0}°"),
            ));
        }
        vr.extend(slider_row(
            app,
            "vr_nav_opacity",
            &tx("pause.options.interface.ui_opacity.name"),
            desc,
            &pct,
        ));
        vr.push(button(
            &tx("pause.options.vr.vr_nav_reset.name"),
            &tx("pause.options.button.reset"),
            desc,
            "vr_nav_reset",
        ));
    }
    vec![
        (tx("pause.options.group.gameplay"), game),
        (tx("pause.options.group.driving"), driving),
        (tx("pause.options.group.controls"), controls),
        (tx("pause.options.group.camera"), camera),
        (tx("pause.options.group.graphics"), graphics),
        (tx("pause.options.group.display"), display),
        (tx("pause.options.group.sound"), sound),
        (tx("pause.options.group.interface"), interface),
        (tx("pause.options.group.vr"), vr),
    ]
}

pub(crate) fn key_rows(app: &App) -> Vec<(String, String)> {
    let Ok(v) = omsi_launcher_lib::get_keybindings() else {
        return vec![(
            row(&tx("pause.page.keys.load_error"), 'i', "", "", None),
            "noop".to_string(),
        )];
    };
    let names = crate::describe::names(&app.args.root, &::config::get_string("ui", "language").unwrap_or_else(|| "en".into()));
    let head = |t: &str, n: usize| {
        (
            row(&t.to_uppercase(), 'i', &n.to_string(), "", None),
            HEADING.to_string(),
        )
    };
    let q = app.key_filter.trim().to_lowercase();
    let mut out = vec![(
        row(
            &tx("pause.page.keys.find"),
            if app.key_search { 'E' } else { 'a' },
            &app.key_filter,
            "",
            None,
        ),
        "keysearch".to_string(),
    )];
    let mut all: Vec<(usize, usize, String, i64, i64)> = Vec::new();
    for (sec, key) in ["vehicles", "game"].iter().enumerate() {
        if let Some(a) = v.get(*key).and_then(|a| a.as_array()) {
            for (i, b) in a.iter().enumerate() {
                let action = b
                    .get("action")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                if !action.is_empty() {
                    all.push((
                        sec,
                        i,
                        action,
                        b.get("scan_code").and_then(|x| x.as_i64()).unwrap_or(0),
                        b.get("modifier").and_then(|x| x.as_i64()).unwrap_or(0),
                    ));
                }
            }
        }
    }
    let groups: [(String, Box<dyn Fn(&(usize, usize, String, i64, i64)) -> bool>); 3] = [
        (tx("pause.page.keys.group_driving"), Box::new(|b| b.0 == 0)),
        (
            tx("pause.page.text.the_game"),
            Box::new(|b| b.0 == 1 && !b.2.starts_with("vr_")),
        ),
        (
            tx("pause.page.keys.group_vr"),
            Box::new(|b| b.0 == 1 && b.2.starts_with("vr_")),
        ),
    ];
    let mut any = false;
    // scripted keybinds
    let scripted: Vec<(usize, String)> = app
        .scripted_names()
        .into_iter()
        .enumerate()
        .filter(|(_, n)| {
            q.is_empty()
                || app.key_capture.is_some_and(|c| c.0 == 2)
                || names.control(n).to_lowercase().contains(&q)
                || n.to_lowercase().contains(&q)
        })
        .collect();
    if !scripted.is_empty() {
        any = true;
        out.push(head(&tx("pause.page.keys.group_scripted"), scripted.len()));
        for (i, action) in scripted {
            let label = names.control(&action);
            let id = format!("keybind 2 {i} {action}");
            if app.key_capture == Some((2, i)) {
                out.push((
                    row(&label, 'E', &tx("pause.page.keys.press_key"), &tx("pause.page.keys.press_key_desc"), None),
                    id,
                ));
            } else {
                out.push((row(&label, 'k', &tx("pause.page.keys.not_set"), "", None), id));
            }
        }
    }
    for (title, pick) in groups.iter() {
        let mut members: Vec<&(usize, usize, String, i64, i64)> = all
            .iter()
            .filter(|b| pick(b))
            .filter(|b| {
                q.is_empty()
                    || app.key_capture == Some((b.0, b.1))
                    || names.control(&b.2).to_lowercase().contains(&q)
                    || b.2.to_lowercase().contains(&q)
                    || crate::keys::key_name(b.3, b.4).to_lowercase().contains(&q)
            })
            .collect();
        if members.is_empty() {
            continue;
        }
        // (alphabetical by the name shown)
        members.sort_by_cached_key(|b| names.control(&b.2).to_lowercase());
        any = true;
        out.push(head(title, members.len()));
        for b in members {
            let (sec, i, action, scan, m) = (b.0, b.1, &b.2, b.3, b.4);
            // (every other binding of the same key, in the game's keys and the vehicle's: both are live)
            let clash: Vec<String> = if scan == 0 {
                Vec::new()
            } else {
                all.iter()
                    .filter(|o| (o.0, o.1) != (sec, i) && o.3 == scan && (o.4 & 6) == (m & 6))
                    .map(|o| names.control(&o.2))
                    .collect()
            };
            let label = names.control(action);
            if app.key_capture == Some((sec, i)) {
                out.push((
                    row(
                        &label,
                        'E',
                        &tx("pause.page.keys.press_key"),
                        &tx("pause.page.keys.press_key_desc"),
                        None,
                    ),
                    format!("keybind {sec} {i} {action}"),
                ));
                continue;
            }
            let value = if scan == 0 {
                tx("pause.page.keys.not_set")
            } else {
                crate::keys::key_name(scan, m)
            };
            let desc = if clash.is_empty() {
                String::new()
            } else {
                ::i18n::translate("pause.msg.key_conflict", &[("keys", &clash.join(", "))])
            };
            out.push((
                row(&label, 'k', &value, &desc, None),
                format!("keybind {sec} {i} {action}"),
            ));
        }
    }
    if !any {
        out.push((
            row(&tx("pause.page.keys.no_match"), 'i', "", &tx("pause.page.keys.no_match_desc"), None),
            HEADING.to_string(),
        ));
    }
    out
}

pub(super) fn vehicle_pages(app: &App) -> Vec<Page> {
    let has = app.player.is_some();
    let server = crate::input_script::on_server(&app.args);
    let mut display: Vec<(String, String)> = Vec::new();
    if has {
        display.push(opens(
            &tx("pause.page.vehicle.action.dest.name"),
            &tx("pause.page.vehicle.action.dest.desc"),
            "dest",
        ));
        display.push(opens(
            &tx("pause.page.vehicle.action.hof.name"),
            &tx("pause.page.vehicle.action.hof.desc"),
            "hof",
        ));
        display.push(opens(
            &tx("pause.page.vehicle.action.number.name"),
            &tx("pause.page.vehicle.action.number.desc"),
            "number",
        ));
    }
    let mut fleet: Vec<(String, String)> = Vec::new();
    if has || !app.placed.is_empty() {
        fleet.push(button(
            &tx("pause.page.vehicle.action.switch.name"),
            &tx("pause.page.vehicle.action.switch.button"),
            &tx("pause.page.vehicle.action.switch.desc"),
            "switch",
        ));
    }
    fleet.push(opens(
        &tx("pause.page.vehicle.action.place.name"),
        &tx("pause.page.vehicle.action.place.desc"),
        "place",
    ));
    if has {
        // (#728: another bus in this one's place, or this one again with its files read
        // anew - a script or a .bus changed - without starting the game again)
        fleet.push(button(
            &tx("pause.page.vehicle.action.swap.name"),
            &tx("pause.page.vehicle.action.swap.button"),
            &tx("pause.page.vehicle.action.swap.desc"),
            "swap",
        ));
        fleet.push(button(
            &tx("pause.page.vehicle.action.couple.name"),
            &tx("pause.page.vehicle.action.couple.name"),
            &tx("pause.page.vehicle.action.couple.desc"),
            "couple",
        ));
        fleet.push(button(
            &tx("pause.page.vehicle.action.uncouple.name"),
            &tx("pause.page.vehicle.action.uncouple.name"),
            &tx("pause.page.vehicle.action.uncouple.desc"),
            "uncouple",
        ));
        fleet.push(button(
            &tx("pause.page.vehicle.action.remove.name"),
            &tx("pause.page.vehicle.action.remove.button"),
            &tx("pause.page.vehicle.action.remove.desc"),
            "remove",
        ));
    }
    if !app.placed.is_empty() {
        fleet.push(button(
            &tx("pause.page.vehicle.action.clearplaced.name"),
            &tx("pause.page.vehicle.action.remove.button"),
            &tx("pause.page.vehicle.action.clearplaced.desc"),
            "clearplaced",
        ));
    }
    let mut service: Vec<(String, String)> = Vec::new();
    if has {
        service.push(button(
            &tx("pause.page.vehicle.action.refuel.name"),
            &tx("pause.page.vehicle.action.refuel.name"),
            &tx("pause.page.vehicle.action.refuel.desc"),
            "refuel",
        ));
        service.push(button(&tx("pause.page.vehicle.action.wash.name"), &tx("pause.page.vehicle.action.wash.name"), &tx("pause.page.vehicle.action.wash.desc"), "wash"));
        service.push(button(
            &tx("pause.page.vehicle.action.repair.name"),
            &tx("pause.page.vehicle.action.repair.name"),
            &tx("pause.page.vehicle.action.repair.desc"),
            "repair",
        ));
        service.push(button(
            &tx("pause.page.vehicle.action.reset_vehicle.name"),
            &tx("pause.page.vehicle.action.reset_vehicle.button"),
            &tx("pause.page.vehicle.action.reset_vehicle.desc"),
            "reset_vehicle",
        ));
        service.push(button(&tx("pause.page.vehicle.action.reload.name"), &tx("pause.page.vehicle.action.reload.button"), &tx("pause.page.vehicle.action.reload.desc"), "reload"));
    }
    let mut driver: Vec<(String, String)> = Vec::new();
    if !server {
        driver.push(opens(
            &tx("pause.page.vehicle.group.driver.title"),
            &tx("pause.page.vehicle.action.driver.desc"),
            "driver",
        ));
    }
    if has && app.on_foot.is_none() {
        driver.push(button(
            &tx("pause.page.vehicle.action.getout.name"),
            &tx("pause.page.vehicle.action.getout.button"),
            &tx("pause.page.vehicle.action.getout.desc"),
            "getout",
        ));
    }
    let mut teleport: Vec<(String, String)> = Vec::new();
    if has && !server && app.navigator.is_some() {
        teleport.push(button(
            &tx("pause.page.vehicle.action.teleport.name"),
            &tx("pause.page.vehicle.action.teleport.button"),
            &tx("pause.page.vehicle.action.teleport.desc"),
            "teleport",
        ));
        teleport.push(opens(
            &tx("pause.page.vehicle.action.tplist.name"),
            &tx("pause.page.vehicle.action.tplist.desc"),
            "tplist",
        ));
    }
    vec![
        (tx("pause.page.vehicle.group.fleet.title"), fleet),
        (tx("pause.page.vehicle.group.display.title"), display),
        (tx("pause.page.vehicle.group.service.title"), service),
        (tx("pause.page.vehicle.group.driver.title"), driver),
        (tx("pause.page.vehicle.group.teleport.title"), teleport),
    ]
}

/// The vehicle page of the pause menu: group ids and the ids of their actions (the same ids
/// `page_action` takes). The texts live in the i18n files under `pause.page.vehicle`.
pub(crate) fn vehicle_menu(app: &App) -> Vec<(&'static str, Vec<(&'static str, bool)>)> {
    let has = app.player.is_some();
    let server = crate::input_script::on_server(&app.args);
    let mut fleet: Vec<(&'static str, bool)> = Vec::new();
    if has || !app.placed.is_empty() {
        fleet.push(("switch", false));
    }
    fleet.push(("place", true));
    if has {
        fleet.extend([("swap", false), ("couple", false), ("uncouple", false), ("remove", false)]);
    }
    if !app.placed.is_empty() {
        fleet.push(("clearplaced", false));
    }
    let mut display: Vec<(&'static str, bool)> = Vec::new();
    if has {
        display.extend([("dest", true), ("hof", true), ("number", true)]);
    }
    let mut service: Vec<(&'static str, bool)> = Vec::new();
    if has {
        service.extend([
            ("refuel", false),
            ("wash", false),
            ("repair", false),
            ("reset_vehicle", false),
            ("reload", false),
        ]);
    }
    let mut driver: Vec<(&'static str, bool)> = Vec::new();
    if !server {
        driver.push(("driver", true));
    }
    if has && app.on_foot.is_none() {
        driver.push(("getout", false));
    }
    let mut teleport: Vec<(&'static str, bool)> = Vec::new();
    if has && !server && app.navigator.is_some() {
        teleport.extend([("teleport", false), ("tplist", true)]);
    }
    vec![
        ("fleet", fleet),
        ("display", display),
        ("service", service),
        ("driver", driver),
        ("teleport", teleport),
    ]
        .into_iter()
        .filter(|g| !g.1.is_empty())
        .collect()
}

pub(crate) fn world_groups(app: &App) -> Vec<(String, Vec<(String, String)>, Vec<(String, Vec<(String, String)>)>)> {
    world_pages(app)
        .into_iter()
        .filter(|p| !p.1.is_empty())
        .map(|(title, rows)| {
            let (presets, rows): (Vec<_>, Vec<_>) = rows.into_iter().partition(|r| r.1.starts_with("clock_set "));
            let subs = if presets.is_empty() { Vec::new() } else { vec![(tx("pause.world.text.presets"), presets)] };
            (title, rows, subs)
        })
        .collect()
}

pub(super) fn world_pages(app: &App) -> Vec<Page> {
    let client = app
        .lan
        .as_ref()
        .is_some_and(|l| l.role == ::network::Role::Client);
    let pct = |v: f32| format!("{:.0} %", v * 100.0);
    let mut time: Vec<(String, String)> = Vec::new();
    let mut weather: Vec<(String, String)> = Vec::new();
    let mut climate: Vec<(String, String)> = Vec::new();
    let mut tools: Vec<(String, String)> = Vec::new();
    if !client {
        let t = app.clock.time;
        let now = format!(
            "{:02}:{:02}",
            ((t / 3600.0) as i64).rem_euclid(24),
            ((t / 60.0) as i64) % 60
        );
        time.extend(switch_row(
            app,
            "time_sync",
            &tx("pause.world.text.real_time_sync"),
            &tx("pause.world.text.the_game_follows_your_device_s_date_and_time"),
        ));
        if app.real_time_locked() {
            let (d, m) = app.clock.day_month();
            let text = format!(
                "{:04}-{m:02}-{d:02}  {}:{:02}",
                app.clock.year,
                now,
                (t as i64) % 60
            );
            time.push((
                row(
                    &tx("pause.world.text.date_and_time"),
                    'i',
                    &text,
                    &tx("pause.world.text.synchronized_with_the_real_time"),
                    None,
                ),
                "noop".to_string(),
            ));
        } else {
            match app.menu_edit.as_ref() {
                Some(d) => {
                    let mut c: Vec<char> = d.chars().collect();
                    c.resize(6, '_');
                    let typed = format!("{}{}:{}{}:{}{}", c[0], c[1], c[2], c[3], c[4], c[5]);
                    time.push((
                        row(
                            &tx("pause.world.text.exact_time"),
                            'E',
                            &typed,
                            &tx("pause.world.text.press_enter_to_change_esc_to_cancel"),
                            None,
                        ),
                        "time_edit".to_string(),
                    ));
                }
                None => {
                    let secs = format!("{}:{:02}", now, (t as i64) % 60);
                    time.push((
                        row(
                            &tx("pause.world.text.exact_time"),
                            'e',
                            &secs,
                            &tx("pause.world.text.change_the_current_time_press_enter_to_change"),
                            None,
                        ),
                        "time_edit".to_string(),
                    ));
                }
            }
            time.extend(slider_row(
                app,
                "hour",
                &tx("pause.world.text.hour"),
                &tx("pause.world.text.set_the_hour_of_the_day_directly"),
                &|v| format!("{:02}", v as i64),
            ));
            time.extend(slider_row(
                app,
                "minute",
                &tx("pause.world.text.minute"),
                &tx("pause.world.text.set_the_minute_directly"),
                &|v| format!("{:02}", v as i64),
            ));
            for (name, hm, secs) in [
                (&tx("pause.world.text.morning"), "06:00", 6 * 3600),
                (&tx("pause.world.text.noon"), "12:00", 12 * 3600),
                (&tx("pause.world.text.evening"), "18:00", 18 * 3600),
                (&tx("pause.world.text.night"), "23:00", 23 * 3600),
            ] {
                time.push(button(
                    name,
                    hm,
                    &tx("pause.world.text.jump_to_this_time_of_day"),
                    &format!("clock_set {secs}"),
                ));
            }
            if let (Some(_), Some(p)) = (app.duty.as_ref(), app.player.as_ref()) {
                let d = p.vehicle.host.tt_delay as f64;
                if d.abs() >= 1.0 {
                    let text = format!(
                        "{}{}:{:02}",
                        if d < 0.0 { "−" } else { "+" },
                        (d.abs() / 60.0) as i64,
                        d.abs() as i64 % 60
                    );
                    time.push(button(
                        &tx("pause.world.text.on_time_with_the_timetable"),
                        &text,
                        &tx("pause.world.text.move_the_clock_so_that_the_vehicle_is_on_time"),
                        "clock_ontime",
                    ));
                }
            }
            if app.lan.is_none() {
                time.extend(slider_row(
                    app,
                    "speed",
                    &tx("pause.world.text.time_speed"),
                    &tx("pause.world.text.how_fast_the_world_s_clock_runs"),
                    &|v| format!("x{v}"),
                ));
            }
        }
        weather.extend(switch_row(
            app,
            "metar_sync",
            &tx("pause.world.text.metar_sync"),
            &tx("pause.world.text.the_weather_follows_the_real_metar_report"),
        ));
        let src = if ::config::get_string("gameplay", "metar_station").unwrap_or_default().is_empty() {
            format!("{} ({})", app.metar_station(), ::user_interface::tr("automatic"))
        } else {
            app.metar_station()
        };
        weather.push((
            row(
                &tx("pause.world.text.metar_source"),
                'o',
                &src,
                &tx("pause.world.text.the_airport_used_for_real_weather"),
                None,
            ),
            "metar_src".to_string(),
        ));
        let typed = if app.menu_edit_icao {
            let mut s = app.menu_edit.clone().unwrap_or_default();
            while s.len() < 4 {
                s.push('_');
            }
            format!("{s}  (typing)")
        } else {
            app.metar_station()
        };
        weather.push((
            row(
                "ICAO",
                if app.menu_edit_icao { 'E' } else { 'e' },
                &typed,
                &tx("pause.world.text.enter_any_4_letter_icao_station"),
                None,
            ),
            "metar_icao_edit".to_string(),
        ));
        if app.metar_locked() {
            weather.push(button(
                &tx("pause.world.text.metar_report"),
                &tx("pause.world.text.refresh_now"),
                &tx("pause.world.text.fetch_the_selected_station_again_without_waiting_for_the_next_automatic_update"),
                "metar_refresh",
            ));
        } else {
            weather.push(button(
                &tx("pause.world.text.metar_report"),
                &tx("pause.world.text.load_once"),
                &tx("pause.world.text.load_the_selected_station_once_without_enabling_continuous_metar_sync"),
                "metar_once",
            ));
        }
        weather.push((
            row(
                &tx("pause.world.text.preset"),
                'o',
                &weather_name(app),
                &tx("pause.world.text.a_ready_made_weather"),
                None,
            ),
            "weather".to_string(),
        ));
        if !app.metar_locked() {
            weather.push(button(
                &tx("pause.world.text.custom_weather"),
                &tx("pause.world.text.edit_current"),
                &tx("pause.world.text.freeze_the_weather_currently_in_force_and_edit_it_as_a_custom_weather"),
                "weather_custom",
            ));
        }
        let cloud = app
            .weather
            .as_ref()
            .and_then(|w| cloud_index(&w.clouds.0))
            .map(|i| CLOUD_TYPES[i].1.to_string())
            .or_else(|| app.weather.as_ref().map(|w| w.clouds.0.trim().to_string()))
            .unwrap_or_default();
        weather.push((
            row(
                &tx("pause.world.text.clouds"),
                'o',
                &cloud,
                &tx("pause.world.text.the_kind_of_clouds_in_the_sky"),
                None,
            ),
            "cloudkind".to_string(),
        ));
        weather.extend(slider_row(
            app,
            "visibility",
            &tx("pause.world.text.visibility"),
            &tx("pause.world.text.how_far_one_can_see_less_is_fog"),
            &|v| {
                if v >= 1000.0 {
                    format!("{:.1} km", v / 1000.0)
                } else {
                    format!("{} m", v as i64)
                }
            },
        ));
        weather.extend(slider_row(
            app,
            "brightness",
            &tx("pause.world.text.brightness"),
            &tx("pause.world.text.brightness_of_the_custom_weather_lighting"),
            &|v| format!("{:.0} %", v * 100.0),
        ));
        let kind = app
            .weather
            .as_ref()
            .map(|w| {
                (w.precip.first().copied().unwrap_or(0.0).max(0.0) as usize)
                    .min(PRECIP_KINDS.len() - 1)
            })
            .unwrap_or(0);
        weather.push((
            row(
                &tx("pause.world.text.precipitation"),
                'o',
                PRECIP_KINDS[kind],
                &tx("pause.world.text.rain_or_snow"),
                None,
            ),
            "precipkind".to_string(),
        ));
        weather.extend(slider_row(
            app,
            "rain_amt",
            &tx("pause.world.text.precipitation_strength"),
            &tx("pause.world.text.how_hard_it_rains_or_snows"),
            &pct,
        ));
        weather.extend(slider_row(
            app,
            "wet",
            &tx("pause.world.text.wet_roads"),
            &tx("pause.world.text.how_wet_the_roads_are_now_they_dry_in_the_sun_wet_in_the_rain"),
            &pct,
        ));
        weather.extend(switch_row(
            app,
            "snow_cover",
            &tx("pause.world.text.snow_cover"),
            &tx("pause.world.text.snow_lying_on_the_world_and_ground"),
        ));
        weather.extend(switch_row(
            app,
            "snow_road",
            &tx("pause.world.text.snow_on_road"),
            &tx("pause.world.text.treat_the_road_surface_as_snow_covered"),
        ));
        climate.extend(slider_row(
            app,
            "temp",
            &tx("pause.world.text.temperature"),
            &tx("pause.world.text.the_air_temperature"),
            &|v| format!("{} °C", v as i64),
        ));
        let dew_temp = app.weather.as_ref().map(|w| w.temp.0).unwrap_or(15.0);
        climate.extend(slider_row(
            app,
            "humidity",
            &tx("pause.world.text.humidity"),
            &tx("pause.world.text.relative_humidity_of_the_air"),
            &|v| {
                format!(
                    "{:.0} % · {} {:.0} °C",
                    v,
                    tx("pause.world.dew"),
                    crate::weather_setup::dew_point_c(dew_temp, v)
                )
            },
        ));
        climate.extend(slider_row(
            app,
            "wind_speed",
            &tx("pause.world.text.wind_speed"),
            &tx("pause.world.text.how_fast_the_wind_blows_it_drives_the_clouds"),
            &|v| format!("{} m/s", v as i64),
        ));
        climate.extend(slider_row(
            app,
            "wind_dir",
            &tx("pause.world.text.wind_direction"),
            &tx("pause.world.text.the_direction_of_the_wind_in_degrees_0_is_north"),
            &|v| format!("{}°", v as i64),
        ));
        // the METAR sync on: only its own rows stay (the weather is the report's)
        if app.metar_locked() {
            weather.retain(|r| {
                matches!(
                    r.1.as_str(),
                    "metar_sync" | "metar_src" | "metar_icao_edit" | "metar_refresh"
                )
            });
            climate.clear();
        }
        tools.push(button(
            &tx("pause.world.text.object_editor"),
            &tx("pause.world.text.open"),
            &tx("pause.world.text.place_and_move_objects_in_the_world"),
            "editor",
        ));
    }
    let mut people: Vec<(String, String)> = Vec::new();
    people.extend(slider_row(
        app,
        "traffic",
        &tx("pause.world.text.traffic"),
        &tx("pause.world.text.how_many_vehicles_drive_around_the_map"),
        &|v| format!("{} vehicles", v as i64),
    ));
    people.extend(slider_row(
        app,
        "pax",
        &tx("pause.world.text.passengers"),
        &tx("pause.world.text.how_many_passengers_wait_at_the_stops_and_ride"),
        &pct,
    ));
    vec![
        (tx("pause.world.text.time"), time),
        (tx("pause.world.text.weather"), weather),
        (tx("pause.world.text.temperature_and_wind"), climate),
        (tx("pause.world.text.traffic_and_people"), people),
        (tx("pause.world.text.tools"), tools),
    ]
}

pub(super) fn pages_of(app: &App, kind: &ListKind) -> Option<(Vec<Page>, usize)> {
    let (pages, tab) = match kind {
        ListKind::Options(t) if is_sub_tab(*t) => (options_pages(app), app.map_return_tab),
        ListKind::Options(t) => (options_pages(app), *t),
        ListKind::Vehicle(t) => (vehicle_pages(app), *t),
        ListKind::World(t) => (world_pages(app), *t),
        _ => return None,
    };
    let pages: Vec<Page> = pages.into_iter().filter(|p| !p.1.is_empty()).collect();
    let tab = tab.min(pages.len().saturating_sub(1));
    Some((pages, tab))
}

pub(super) type TitlesCache = Option<(ListKind, bool, std::time::Instant, (Vec<String>, usize))>;

thread_local! {
    static TITLES: std::cell::RefCell<TitlesCache> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn forget_page_titles() {
    TITLES.with(|c| *c.borrow_mut() = None);
    *DRIVER_SCAN.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *WEATHER_SCAN.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

pub(crate) fn page_titles(app: &App, kind: &ListKind) -> Option<(Vec<String>, usize)> {
    let vr_nav_available = app.vr_active() && app.player.is_some();
    if let Some(hit) = TITLES.with(|c| {
        c.borrow()
            .as_ref()
            .filter(|(k, vr, t, _)| {
                k == kind && *vr == vr_nav_available && t.elapsed().as_millis() < 5000
            })
            .map(|(_, _, _, r)| r.clone())
    }) {
        return Some(hit);
    }
    let (pages, tab) = pages_of(app, kind)?;
    let r = (
        pages.iter().map(|p| p.0.clone()).collect::<Vec<_>>(),
        tab,
    );
    TITLES.with(|c| {
        *c.borrow_mut() = Some((
            kind.clone(),
            vr_nav_available,
            std::time::Instant::now(),
            r.clone(),
        ))
    });
    Some(r)
}
