//! The game menu's own windows besides the administration (see `admin`), as OMSI has them
//! in its menus: the options that can change while driving, the line and tour to drive,
//! the driver whose personnel file the run goes into, and the bus's fleet number. Each is a
//! list of (label, action) lines in the menu's chooser; choosing a line does it and shows
//! the list again (or the next one: a line's tours).

use crate::App;

/// Which list the chooser shows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ListKind {
    Admin,
    Options,
    Lines,
    Tours(String),
    Drivers,
    Numbers,
}

/// The time speeds, traffic amounts and passenger shares the options step through.
const SPEEDS: [f64; 5] = [1.0, 2.0, 4.0, 8.0, 15.0];
pub(crate) const TRAFFIC: [usize; 7] = [0, 10, 20, 30, 50, 80, 120];
const PAX: [f32; 6] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0];
const VOLUME: [f32; 6] = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];

fn on_off(b: bool) -> &'static str {
    if b {
        "on"
    } else {
        "off"
    }
}

/// The next of `steps` after `now` (round to the first).
pub(crate) fn next_step<T: PartialOrd + Copy>(steps: &[T], now: T) -> T {
    steps.iter().copied().find(|s| *s > now).unwrap_or(steps[0])
}

pub(crate) fn items(app: &App, kind: &ListKind) -> Vec<(String, String)> {
    let tr = |t: &str| omsi_ui::tr(t).into_owned();
    let mut out: Vec<(String, String)> = Vec::new();
    match kind {
        ListKind::Admin => return crate::admin::items(app),
        ListKind::Options => {
            let s = &app.settings;
            if app.lan.is_none() {
                out.push((format!("{}: x{}", tr("Time speed"), s.time_speed), "speed".into()));
            }
            if let Some(t) = app.traffic.as_ref() {
                out.push((format!("{}: {}", tr("Traffic"), t.target), "traffic".into()));
            }
            out.push((format!("{}: {:.0} %", tr("Passengers"), s.pax_density * 100.0), "pax".into()));
            out.push((format!("{}: {:.0} %", tr("Volume"), s.volume * 100.0), "volume".into()));
            out.push((format!("{}: {}", tr("Navigator"), tr(on_off(app.navigator.as_ref().is_some_and(|n| n.enabled)))), "navigator".into()));
            out.push((format!("{}: {}", tr("Sun shadows"), tr(on_off(s.shadows))), "shadows".into()));
            out.push((format!("{}: {}", tr("Head movement"), tr(on_off(s.head_movement))), "head".into()));
            out.push((format!("{}: {}", tr("Steering with the mouse"), tr(on_off(app.mouse_drive))), "mouse".into()));
            out.push((format!("{}: {}", tr("Frame rate"), tr(on_off(s.show_fps))), "fps".into()));
        }
        ListKind::Lines => {
            if let Some(sch) = app.schedule.as_ref() {
                let mut lines: Vec<&omsi_timetable::Line> = sch.data.lines.iter().filter(|l| l.user_allowed && !l.tours.is_empty()).collect();
                lines.sort_by(|a, b| natural(&a.name, &b.name));
                for l in lines {
                    out.push((format!("{} {}  ({} {})", tr("Line"), l.name, l.tours.len(), tr("tours")), format!("line {}", l.name)));
                }
            }
            if app.duty.is_some() {
                out.push((tr("Free drive (no duty)"), "free".into()));
            }
            if out.is_empty() {
                out.push((tr("No timetable on this map"), "back".into()));
            }
        }
        ListKind::Tours(line) => {
            if let Some(l) = app.schedule.as_ref().and_then(|s| s.data.lines.iter().find(|l| l.name == *line)) {
                for t in &l.tours {
                    let first = t.trips.first().map(|x| format!("  {:02}:{:02}", (x.departure / 60.0) as i32 % 24, (x.departure % 60.0) as i32)).unwrap_or_default();
                    out.push((format!("{} {}{first}", tr("Tour"), t.number.trim()), format!("tour {}\u{1}{}", line, t.number)));
                }
            }
        }
        ListKind::Drivers => {
            for name in driver_names(app) {
                let mark = if app.career.path.as_ref().and_then(|p| p.file_stem()).is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(&name)) { format!("  {}", tr("(now)")) } else { String::new() };
                out.push((format!("{name}{mark}"), format!("driver {name}")));
            }
        }
        ListKind::Numbers => {
            if let Some(p) = app.player.as_ref() {
                for (n, reg) in fleet_numbers(&p.vehicle) {
                    out.push((if reg.is_empty() { n.clone() } else { format!("{n}  ({reg})") }, format!("number {n}\u{1}{reg}")));
                }
            }
            if out.is_empty() {
                out.push((tr("This bus has no list of fleet numbers"), "back".into()));
            }
        }
    }
    out.push((tr("Back"), "back".into()));
    out
}

/// Do a line of the list; returns the list to show next (None: back to the menu).
pub(crate) fn run(app: &mut App, kind: &ListKind, action: &str) -> Option<ListKind> {
    if action == "back" {
        return None;
    }
    let (verb, arg) = action.split_once(' ').unwrap_or((action, ""));
    match kind {
        ListKind::Admin => {
            crate::admin::run(app, action);
            Some(ListKind::Admin)
        }
        ListKind::Options => {
            let s = &mut app.settings;
            let key_value: Option<(&str, String)> = match verb {
                "speed" => {
                    s.time_speed = next_step(&SPEEDS, s.time_speed);
                    Some(("time_speed", s.time_speed.to_string()))
                }
                "traffic" => {
                    if let Some(t) = app.traffic.as_mut() {
                        t.target = next_step(&TRAFFIC, t.target);
                        app.args.traffic = t.target;
                    }
                    None
                }
                "pax" => {
                    s.pax_density = next_step(&PAX, s.pax_density);
                    Some(("pax_density", s.pax_density.to_string()))
                }
                "volume" => {
                    s.volume = next_step(&VOLUME, s.volume);
                    Some(("volume", s.volume.to_string()))
                }
                "navigator" => {
                    let on = app.navigator.as_ref().is_some_and(|n| n.enabled);
                    if let Some(n) = app.navigator.as_mut() {
                        n.enabled = !on;
                    }
                    app.settings.navigator = !on;
                    Some(("navigator", (!on as u8).to_string()))
                }
                "shadows" => {
                    s.shadows = !s.shadows;
                    Some(("shadows", (s.shadows as u8).to_string()))
                }
                "head" => {
                    s.head_movement = !s.head_movement;
                    Some(("head_movement", (s.head_movement as u8).to_string()))
                }
                "mouse" => {
                    app.mouse_drive = !app.mouse_drive;
                    None
                }
                "fps" => {
                    s.show_fps = !s.show_fps;
                    Some(("show_fps", (s.show_fps as u8).to_string()))
                }
                _ => None,
            };
            // kept for the next game too, as OMSI keeps its options
            if let Some((k, v)) = key_value {
                remember_setting(k, &v);
            }
            Some(ListKind::Options)
        }
        ListKind::Lines => match verb {
            "line" => Some(ListKind::Tours(arg.to_string())),
            "free" => {
                app.duty = None;
                app.service_msg = Some(("Free drive: no duty".into(), 4.0));
                None
            }
            _ => None,
        },
        ListKind::Tours(_) => {
            if let Some((line, tour)) = arg.split_once('\u{1}') {
                start_duty(app, line, tour);
            }
            None
        }
        ListKind::Drivers => {
            switch_driver(app, arg);
            Some(ListKind::Drivers)
        }
        ListKind::Numbers => {
            if let (Some((n, reg)), Some(p)) = (arg.split_once('\u{1}'), app.player.as_mut()) {
                let v = &mut p.vehicle;
                if let Some(i) = v.ty.program.str_var("number") {
                    v.state.str_vars[i as usize] = n.to_string();
                }
                if !reg.is_empty() {
                    if let Some(i) = v.ty.program.str_var("ident") {
                        v.state.str_vars[i as usize] = reg.to_string();
                    }
                }
                app.service_msg = Some((format!("Fleet number {n}"), 3.0));
            }
            None
        }
    }
}

/// Numbers compared as numbers where they are ("5" before "13", "N30" after "M49").
fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    let key = |s: &str| {
        let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
        (digits.parse::<u64>().unwrap_or(u64::MAX), s.to_ascii_lowercase())
    };
    key(a).cmp(&key(b))
}

/// Write one key of `~/.openomsi/settings.cfg` (the launcher's file; the other lines
/// stay as they are).
fn remember_setting(key: &str, value: &str) {
    let Ok(mut v) = omsi_launcher_lib::get_settings() else { return };
    let parsed: serde_json::Value = value.parse::<f64>().map(serde_json::Value::from).unwrap_or_else(|_| serde_json::Value::from(value));
    let parsed = match (key, &parsed) {
        ("navigator" | "shadows" | "head_movement" | "show_fps", serde_json::Value::Number(n)) => serde_json::Value::Bool(n.as_f64().unwrap_or(0.0) > 0.5),
        ("time_speed", _) => serde_json::Value::from(value),
        _ => parsed,
    };
    v[key] = parsed;
    if let Err(e) = omsi_launcher_lib::save_settings(&v) {
        log::warn!("settings not saved: {e:#}");
    }
}

/// The personnel files there are (content folder and OMSI 2's `Drivers`), by name.
fn driver_names(app: &App) -> Vec<String> {
    let mut names: Vec<String> = omsi_cfg::read_dir_merged("Drivers")
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("odr")))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .collect();
    let _ = app;
    names.sort_by_key(|n| n.to_ascii_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    names
}

/// Go on with another driver: this run so far into the old personnel file, the rest into
/// the new one.
fn switch_driver(app: &mut App, name: &str) {
    if app.career.path.is_some() {
        if let Err(e) = app.career.save() {
            log::warn!("writing the personnel file: {e}");
        }
    }
    let rel = format!("Drivers/{name}.odr");
    let mut next = crate::career::Career::load(&app.args.root, &rel);
    // (the distance and the clock of the run go on; the counters start with the new file)
    next.seconds = app.career.seconds;
    app.career = next;
    app.args.driver = Some(rel);
    app.service_msg = Some((format!("Driver: {name}"), 3.0));
}

/// The fleet numbers of the bus's `[number]` list with their registrations.
fn fleet_numbers(v: &omsi_sim::VehicleInstance) -> Vec<(String, String)> {
    let Some(list) = v.ty.def.number_file.as_ref() else { return Vec::new() };
    let Ok(nl) = omsi_vehicle::vehicle::NumberList::load(&omsi_cfg::resolve_path(v.ty.def.dir(), list)) else { return Vec::new() };
    nl.numbers
        .iter()
        .map(|n| {
            let reg = match &v.ty.def.registration_automatic {
                Some((pre, post)) => format!("{pre}{n}{post}"),
                None => String::new(),
            };
            (n.clone(), reg)
        })
        .collect()
}

/// Take on line `line`, tour `tour` from now: the duty, and the IBIS typed for it.
fn start_duty(app: &mut App, line: &str, tour: &str) {
    let (Some(w), Some(sch)) = (app.world.clone(), app.schedule.as_mut()) else { return };
    let now = app.clock.time;
    match sch.player_duty(&w, line, tour, now, None, false) {
        Ok(mut d) => {
            if let Some(p) = app.player.as_mut() {
                d.update(&mut p.vehicle, now);
                let (trip, stop) = d.trip_for_ibis();
                p.set_duty_destination(trip, stop);
            }
            app.args.line = Some(line.to_string());
            app.args.tour = Some(tour.to_string());
            app.duty = Some(d);
            app.service_msg = Some((format!("Line {line}, tour {}", tour.trim()), 4.0));
        }
        Err(e) => app.service_msg = Some((format!("No duty: {e}"), 8.0)),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn steps_wrap_round() {
        assert_eq!(super::next_step(&super::SPEEDS, 1.0), 2.0);
        assert_eq!(super::next_step(&super::SPEEDS, 15.0), 1.0);
        assert_eq!(super::next_step(&super::TRAFFIC, 35), 50);
    }

    #[test]
    fn lines_sort_as_numbers() {
        let mut v = vec!["13N", "5", "137", "N30", "92"];
        v.sort_by(|a, b| super::natural(a, b));
        assert_eq!(v, vec!["5", "13N", "92", "137", "N30"]);
    }
}
