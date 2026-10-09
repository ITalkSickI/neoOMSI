use crate::controllers::{self, DeviceCfg, Func};
use crate::pax_pack::{PaxPack, Status as PaxStatus};
use anyhow::{Context, Result, anyhow};
use omsi_launcher_lib as lib;
use omsi_launcher_lib::servers::{self, ServerEntry};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub(super) const COMMANDS: &[&str] = &[
    "config",
    "save_config",
    "maps",
    "vehicles",
    "weather",
    "lines",
    "minimap",
    "ibis",
    "profiles",
    "profile",
    "create_profile",
    "delete_profile",
    "mods",
    "modinfo",
    "install",
    "start_install",
    "cancel_install",
    "clear_installs",
    "instances",
    "launch",
    "stop",
    "log",
    "join",
    "settings",
    "save_settings",
    "pax_pack",
    "install_pax_pack",
    "update_check",
    "option_presets",
    "keybindings",
    "save_keybindings",
    "controllers",
    "save_controllers",
    "preview",
    "situations",
    "tutorials",
    "servers",
    "save_servers",
    "version",
];

type Slot = Arc<Mutex<Option<Value>>>;

static LISTS: Mutex<Vec<(&str, Slot)>> = Mutex::new(Vec::new());

/// The bus list of a big installation takes minutes.
fn cached(name: &'static str, read: impl FnOnce() -> Result<Value>) -> Result<Value> {
    let slot = {
        let mut lists = LISTS.lock().unwrap_or_else(|e| e.into_inner());
        match lists.iter().find(|(n, _)| *n == name) {
            Some((_, s)) => s.clone(),
            None => {
                let s = Slot::default();
                lists.push((name, s.clone()));
                s
            }
        }
    };
    let mut value = slot.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(v) = value.as_ref() {
        return Ok(v.clone());
    }
    let fresh = read()?;
    *value = Some(fresh.clone());
    Ok(fresh)
}

static SETTINGS_FILE: Mutex<()> = Mutex::new(());
static CONFIG_FILE: Mutex<()> = Mutex::new(());

fn settings_file() -> std::sync::MutexGuard<'static, ()> {
    SETTINGS_FILE.lock().unwrap_or_else(|e| e.into_inner())
}

fn save_settings_with(
    changes: &Value,
    read: impl Fn() -> Result<Value>,
    write: impl Fn(&Value) -> Result<()>,
) -> Result<Value> {
    let _file = settings_file();
    let mut v = read()?;
    if let (Some(v), Some(changes)) = (v.as_object_mut(), changes.as_object()) {
        v.extend(changes.clone());
    }
    write(&v)?;
    read()
}

pub(super) fn forget_content() {
    LISTS.lock().unwrap_or_else(|e| e.into_inner()).clear();
    *CONTENT.lock().unwrap_or_else(|e| e.into_inner()) = None;
    super::minimap::forget();
}

pub(super) fn call(cmd: &str, a: &Value) -> Result<Value> {
    let s = |k: &str| a.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let n = |k: &str| a.get(k).and_then(|v| v.as_u64());
    Ok(match cmd {
        "config" => serde_json::to_value(lib::load_config())?,
        "save_config" => {
            let _file = CONFIG_FILE.lock().unwrap_or_else(|e| e.into_inner());
            let mut c = lib::load_config();
            let before = (c.root.clone(), c.game.clone());
            for (key, field) in [("root", &mut c.root), ("game", &mut c.game), ("profile", &mut c.profile)] {
                if let Some(v) = a.get(key).and_then(|v| v.as_str()) {
                    *field = v.to_string();
                }
            }
            lib::save_config(&c)?;
            if (c.root.clone(), c.game.clone()) != before {
                forget_content();
            }
            serde_json::to_value(lib::load_config())?
        }
        "maps" => cached("maps", || Ok(serde_json::to_value(lib::list_maps()?)?))?,
        "vehicles" => cached("vehicles", || Ok(serde_json::to_value(lib::list_vehicles()?)?))?,
        "weather" => cached("weather", || Ok(serde_json::to_value(lib::list_weather()?)?))?,
        "lines" => serde_json::to_value(lib::list_lines(&s("map"), &s("date"))?)?,
        "minimap" => super::minimap::minimap(&s("map"), &s("date"))?,
        "ibis" => serde_json::to_value(lib::ibis_info(&s("bus"), &s("hof"), &s("line"))?)?,
        "profiles" => serde_json::to_value(lib::list_profiles()?)?,
        "profile" => serde_json::to_value(lib::get_profile(&s("name"))?)?,
        "create_profile" => {
            let sex = s("sex");
            serde_json::to_value(lib::create_profile(&s("name"), if sex.is_empty() { "M" } else { &sex })?)?
        }
        "delete_profile" => {
            lib::delete_profile(&s("name"))?;
            json!({ "deleted": true })
        }
        "mods" => serde_json::to_value(lib::mods_status()?)?,
        "modinfo" => serde_json::to_value(lib::inspect_mod(Path::new(&s("path")))?)?,
        "install" | "start_install" => {
            let mode = s("mode");
            serde_json::to_value(lib::start_install(
                Path::new(&s("path")),
                if mode.is_empty() { "auto" } else { &mode },
            )?)?
        }
        "cancel_install" => json!({ "cancelled": lib::install::cancel(n("id").unwrap_or(0)) }),
        "clear_installs" => {
            lib::install::clear_finished();
            json!({})
        }
        "instances" => serde_json::to_value(lib::list_instances())?,
        "launch" => serde_json::to_value(lib::launch(&serde_json::from_value(a.clone())?)?)?,
        "stop" => {
            let by_itself = lib::stop_instance(n("pid").unwrap_or(0) as u32)?;
            json!({ "stopped": true, "ended_by_itself": by_itself })
        }
        "log" => json!(lib::log_tail(n("pid").unwrap_or(0) as u32, n("lines").unwrap_or(40) as usize)?),
        "join" => lib::check_join(&s("text")),
        "settings" => {
            let _file = settings_file();
            lib::init_settings();
            lib::get_settings()?
        }
        "save_settings" => {
            let saved = save_settings_with(
                a,
                || {
                    lib::init_settings();
                    lib::get_settings()
                },
                lib::save_settings,
            )?;
            // content names come in the settings' language
            if a.get("language").is_some() {
                forget_content();
            }
            if a.get("pax_models").and_then(|v| v.as_str()) == Some("realistic") {
                with_pax(|p| {
                    if matches!(p.status(), PaxStatus::Missing | PaxStatus::Outdated) {
                        p.start();
                    }
                });
            }
            saved
        }
        "pax_pack" => pax_status(),
        "update_check" => match crate::updater::latest()? {
            Some(r) => json!({
                "version": r.version,
                "page": r.page,
                "notes": r.notes,
                "prerelease": r.prerelease,
                "size": r.size,
            }),
            None => Value::Null,
        },
        "install_pax_pack" => {
            with_pax(PaxPack::start);
            pax_status()
        }
        "option_presets" => Value::Array(
            lib::option_presets()
                .into_iter()
                .map(|(name, values)| json!({ "name": name, "values": values }))
                .collect(),
        ),
        "keybindings" => {
            let _file = settings_file();
            lib::init_settings();
            lib::get_keybindings()?
        }
        "save_keybindings" => {
            let _file = settings_file();
            lib::save_keybindings(a)?;
            lib::get_keybindings()?
        }
        "controllers" => {
            let _file = settings_file();
            lib::init_settings();
            controllers_now()
        }
        "save_controllers" => {
            let _file = settings_file();
            lib::init_settings();
            save_controllers(a)?;
            controllers_now()
        }
        "preview" => json!(lib::bus_preview(&s("bus"), &s("paint"))?),
        "situations" => Value::Array(
            lib::saved_situations(&s("map"))
                .into_iter()
                .map(|x| json!({ "name": x.name, "file": x.file.to_string_lossy(), "time": x.saved }))
                .collect(),
        ),
        "tutorials" => Value::Array(
            lib::tutorials()
                .into_iter()
                .map(|(number, title, text)| json!({ "number": number, "title": title, "text": text }))
                .collect(),
        ),
        "servers" => servers(),
        "save_servers" => {
            #[derive(Deserialize)]
            struct Args {
                servers: Vec<ServerEntry>,
            }
            let list: Args = serde_json::from_value(a.clone()).context("servers: a list of name and address")?;
            servers::store(&servers::with_official(list.servers))?;
            json!({})
        }
        "version" => json!({ "version": crate::startup::VERSION, "protocol": launcher_protocol::VERSION.parse::<u32>().unwrap_or(0) }),
        _ => return Err(anyhow!("this engine has no command {cmd:?}")),
    })
}

static PAX: Mutex<Option<PaxPack>> = Mutex::new(None);
/// The game's content folder, as `launch` starts it: looking it up writes a probe file.
static CONTENT: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

fn content() -> Option<PathBuf> {
    CONTENT
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(lib::content_dir)
        .clone()
}

fn with_pax<T>(f: impl FnOnce(&mut PaxPack) -> T) -> T {
    let mut pax = PAX.lock().unwrap_or_else(|e| e.into_inner());
    let p = match pax.take() {
        Some(p)
            if matches!(
                p.status(),
                PaxStatus::Downloading { .. } | PaxStatus::Installing | PaxStatus::Failed(_)
            ) =>
        {
            p
        }
        _ => PaxPack::new(content()),
    };
    f(pax.insert(p))
}

pub(super) fn pax_status() -> Value {
    let (state, done, total, message) = match with_pax(|p| p.status()) {
        PaxStatus::Missing => ("missing", 0, 0, String::new()),
        PaxStatus::Outdated => ("outdated", 0, 0, String::new()),
        PaxStatus::Downloading { done, total } => ("downloading", done, total, String::new()),
        PaxStatus::Installing => ("installing", 0, 0, String::new()),
        PaxStatus::Installed => ("installed", 0, 0, String::new()),
        PaxStatus::Failed(e) => ("failed", 0, 0, e),
    };
    let installed = content().as_deref().and_then(crate::pax_pack::installed_version);
    let latest = crate::pax_pack::latest().map(|r| {
        json!({ "version": r.version, "notes": r.notes, "page": r.page, "published": r.published })
    });
    json!({
        "state": state,
        "done": done,
        "total": total,
        "message": message,
        "installed": installed,
        "latest": latest,
    })
}

fn servers() -> Value {
    let asked: Vec<_> = servers::load()
        .into_iter()
        .map(|e| {
            let address = e.address.clone();
            (e, std::thread::spawn(move || network::ws::query(&address, false)))
        })
        .collect();
    Value::Array(
        asked
            .into_iter()
            .map(|(e, answer)| {
                let official = network::official::is_alias(&e.address);
                let answer = answer
                    .join()
                    .unwrap_or_else(|_| Err("the query stopped on an error".into()));
                match answer {
                    Ok(i) => json!({
                        "address": e.address,
                        "name": if e.name.trim().is_empty() { i.name } else { e.name },
                        "official": official,
                        "motd": i.motd,
                        "map": i.map,
                        "time": i.time,
                        "weather": i.weather,
                        "players": i.players,
                        "max_players": i.max_players,
                        "error": null,
                    }),
                    Err(err) => json!({
                        "address": e.address,
                        "name": if e.name.trim().is_empty() { e.address.clone() } else { e.name },
                        "official": official,
                        "motd": "",
                        "map": "",
                        "time": "",
                        "weather": "",
                        "players": 0,
                        "max_players": 0,
                        "error": err,
                    }),
                }
            })
            .collect(),
    )
}

const AXES: [&str; 8] = [
    "X axis",
    "Y axis",
    "Z axis",
    "X rotation",
    "Y rotation",
    "Z rotation",
    "Slider 1",
    "Slider 2",
];

const FUNCTIONS: [(Func, &str); 7] = [
    (Func::Steering, "steering"),
    (Func::Throttle, "throttle"),
    (Func::Brake, "brake"),
    (Func::Clutch, "clutch"),
    (Func::ThrottleBrake, "throttle_brake"),
    (Func::LookX, "look_x"),
    (Func::LookY, "look_y"),
];

const SHAPE_BITS: i32 = 4 | 8 | 0x10;

const SHAPES: [(&str, i32); 5] = [
    ("linear", 0),
    ("progressive", 8),
    ("degressive", 4),
    ("bi-progressive", 8 | 0x10),
    ("bi-degressive", 4 | 0x10),
];

fn controller_json(d: &DeviceCfg, live: Option<&controllers::Connected>) -> Value {
    let axes: Vec<Value> = (0..8)
        .map(|k| {
            let function = d.axes[k]
                .and_then(|(f, _)| FUNCTIONS.iter().find(|(g, _)| *g == f))
                .map_or("", |(_, name)| name);
            let shape = SHAPES
                .iter()
                .find(|(_, bits)| *bits == d.axis_flags[k] & SHAPE_BITS)
                .map_or("linear", |(name, _)| name);
            let value = live
                .and_then(|c| c.axes.iter().find(|(slot, _)| *slot == k))
                .map_or(0.0, |(_, v)| *v);
            json!({
                "name": AXES[k],
                "value": value,
                "function": function,
                "reversed": d.axes[k].is_some_and(|(_, r)| r),
                "shape": shape,
            })
        })
        .collect();
    json!({
        "name": d.name,
        "connected": live.is_some(),
        "enabled": d.enabled,
        "deadzone": d.deadzone.unwrap_or_else(controllers::global_deadzone),
        "force_feedback": d.ff_scale.is_none_or(|(steer, _)| steer > 0.0),
        "axes": axes,
        "buttons": d.buttons,
    })
}

fn controllers_now() -> Value {
    let connected = super::pads::connected();
    let configured = controllers::read_cfg();
    let live = |name: &str| {
        connected
            .iter()
            .find(|c| controllers::names_match(&c.name, name))
    };
    let mut out: Vec<Value> = configured
        .iter()
        .map(|d| controller_json(d, live(&d.name)))
        .collect();
    for c in &connected {
        if !configured.iter().any(|d| controllers::names_match(&c.name, &d.name)) {
            let d = DeviceCfg {
                name: c.name.clone(),
                ..Default::default()
            };
            out.push(controller_json(&d, Some(c)));
        }
    }
    Value::Array(out)
}

#[derive(Deserialize)]
struct AxisIn {
    #[serde(default)]
    function: String,
    #[serde(default)]
    reversed: bool,
    #[serde(default)]
    shape: String,
}

#[derive(Deserialize)]
struct ControllerIn {
    name: String,
    #[serde(default = "yes")]
    enabled: bool,
    #[serde(default)]
    deadzone: f32,
    #[serde(default = "yes")]
    force_feedback: bool,
    #[serde(default)]
    axes: Vec<AxisIn>,
    #[serde(default)]
    buttons: Vec<(String, String)>,
}

fn yes() -> bool {
    true
}

/// Keeps what the page does not show: calibration, other axis flags, vibration strength.
fn apply(d: &mut DeviceCfg, c: &ControllerIn) {
    d.enabled = c.enabled;
    let deadzone = c.deadzone.clamp(0.0, 0.3);
    // a calibrated axis's own dead zone would outrank the new one
    if (deadzone - d.deadzone.unwrap_or_else(controllers::global_deadzone)).abs() > 1e-4 {
        d.deadzone = Some(deadzone);
        for cal in d.calibration.iter_mut().flatten() {
            cal.deadzone = None;
        }
    }
    for (k, axis) in c.axes.iter().take(8).enumerate() {
        d.axes[k] = FUNCTIONS
            .iter()
            .find(|(_, name)| *name == axis.function)
            .map(|(f, _)| (*f, axis.reversed));
        let bits = SHAPES
            .iter()
            .find(|(name, _)| *name == axis.shape)
            .map_or(0, |(_, bits)| *bits);
        d.axis_flags[k] = (d.axis_flags[k] & !SHAPE_BITS) | bits;
    }
    d.buttons = c.buttons.clone();
    let (steer, vibration) = d.ff_scale.unwrap_or((1.0, 1.0));
    d.ff_scale = match (c.force_feedback, steer > 0.0) {
        (false, _) => Some((0.0, vibration)),
        (true, false) => Some((1.0, vibration)),
        (true, true) => d.ff_scale,
    };
}

fn save_controllers(a: &Value) -> Result<()> {
    let list: Vec<ControllerIn> = serde_json::from_value(a.get("controllers").cloned().unwrap_or_default())
        .context("controllers: the list as `controllers` gives it")?;
    let devices: Vec<DeviceCfg> = list
        .iter()
        .map(|c| {
            let mut d = controllers::read_device(&c.name);
            apply(&mut d, c);
            d
        })
        .collect();
    controllers::write_cfg(&devices);
    ::config::save().map_err(|e| anyhow!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_controller_round_trips_and_keeps_what_the_page_does_not_show() {
        let mut d = DeviceCfg {
            name: "G29".into(),
            ..Default::default()
        };
        d.axes[0] = Some((Func::Steering, false));
        d.axes[2] = Some((Func::Brake, true));
        d.axis_flags[2] = 2 | 8;
        d.ff_scale = Some((0.7, 0.4));
        let v = controller_json(&d, None);
        assert_eq!(v["axes"][2]["function"], "brake");
        assert_eq!(v["axes"][2]["reversed"], true);
        assert_eq!(v["axes"][2]["shape"], "progressive");
        assert_eq!(v["force_feedback"], true);
        assert_eq!(v["connected"], false);

        let mut changed: ControllerIn = serde_json::from_value(v).unwrap();
        changed.axes[2].shape = "bi-degressive".into();
        changed.axes[1].function = "clutch".into();
        changed.force_feedback = false;
        let mut back = d.clone();
        apply(&mut back, &changed);
        assert_eq!(back.axes[1], Some((Func::Clutch, false)));
        assert_eq!(back.axis_flags[2], 2 | 4 | 0x10, "the range bit stays");
        assert_eq!(back.ff_scale, Some((0.0, 0.4)), "the vibration strength stays");
        changed.force_feedback = true;
        apply(&mut back, &changed);
        assert_eq!(back.ff_scale, Some((1.0, 0.4)));
    }

    #[test]
    fn a_new_dead_zone_replaces_the_calibrated_ones() {
        let mut d = DeviceCfg {
            name: "G29".into(),
            deadzone: Some(0.05),
            ..Default::default()
        };
        d.calibration[0] = Some(controllers::AxisCal {
            min: -1.0,
            centre: Some(0.0),
            max: 1.0,
            deadzone: Some(0.2),
        });
        let mut changed: ControllerIn = serde_json::from_value(controller_json(&d, None)).unwrap();
        apply(&mut d, &changed);
        assert_eq!(d.deadzone(0), 0.2, "an unchanged dead zone keeps the calibration's");
        changed.deadzone = 0.1;
        apply(&mut d, &changed);
        assert_eq!(d.deadzone(0), 0.1);
        assert_eq!(d.calibration[0].unwrap().deadzone, None);
    }

    #[test]
    fn settings_saved_at_the_same_time_all_stay() {
        let store = Arc::new(Mutex::new(json!({ "language": "en" })));
        let threads: Vec<_> = (0..8)
            .map(|k| {
                let store = store.clone();
                std::thread::spawn(move || {
                    let read = || {
                        let v = store.lock().unwrap().clone();
                        // a slow disk: without the lock every thread reads before any writes
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        Ok(v)
                    };
                    let write = |v: &Value| {
                        *store.lock().unwrap() = v.clone();
                        Ok(())
                    };
                    save_settings_with(&json!({ format!("key{k}"): k }), read, write).unwrap()
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        let saved = store.lock().unwrap().clone();
        for k in 0..8 {
            assert_eq!(saved[format!("key{k}")], k, "{saved}");
        }
        assert_eq!(saved["language"], "en");
    }

    #[test]
use anyhow::{Result, anyhow};
use launcher_protocol::api::{
    self, AxisFunction, AxisShape, Controller, ControllerAxis, Empty, PaxState, request::Command, response::Answer,
    setting_value,
};
use omsi_launcher_lib::servers;
use serde_json::Value;
use std::collections::HashMap;
type Slot = Arc<Mutex<Option<Answer>>>;
fn cached(name: &'static str, read: impl FnOnce() -> Result<Answer>) -> Result<Answer> {
fn list<T, U: From<T>>(items: Vec<T>) -> Vec<U> {
    items.into_iter().map(Into::into).collect()
}

fn settings(v: &Value) -> api::Settings {
    api::Settings {
        values: api::setting_values(v),
    }
}

pub(super) fn call(command: Command) -> Result<Answer> {
    Ok(match command {
        Command::Handshake(_) | Command::Shutdown(_) => {
            return Err(anyhow!("the handshake and shutdown are answered before any command"));
        }
        Command::Config(_) => Answer::Config(lib::load_config().into()),
        Command::SaveConfig(changes) => {
            for (v, field) in [(changes.root, &mut c.root), (changes.game, &mut c.game), (changes.profile, &mut c.profile)] {
                if let Some(v) = v {
                    *field = v;
            Answer::SaveConfig(lib::load_config().into())
        Command::Maps(_) => cached("maps", || Ok(Answer::Maps(api::MapList { maps: list(lib::list_maps()?) })))?,
        Command::Vehicles(_) => cached("vehicles", || {
            Ok(Answer::Vehicles(api::VehicleList {
                vehicles: list(lib::list_vehicles()?),
            }))
        })?,
        Command::Weather(_) => cached("weather", || {
            Ok(Answer::Weather(api::WeatherList {
                weather: list(lib::list_weather()?),
            }))
        })?,
        Command::Lines(a) => Answer::Lines(api::LineList {
            lines: list(lib::list_lines(&a.map, &a.date)?),
        }),
        Command::Minimap(a) => Answer::Minimap(super::minimap::minimap(&a.map, a.date.as_deref().unwrap_or(""))?),
        Command::Ibis(a) => Answer::Ibis(lib::ibis_info(&a.bus, &a.hof, &a.line)?.into()),
        Command::Profiles(_) => Answer::Profiles(api::ProfileNames {
            names: lib::list_profiles()?,
        }),
        Command::Profile(a) => Answer::Profile(lib::get_profile(&a.name)?.into()),
        Command::CreateProfile(a) => {
            let sex = a.sex.as_deref().filter(|s| !s.is_empty()).unwrap_or("M");
            Answer::CreateProfile(lib::create_profile(&a.name, sex)?.into())
        Command::DeleteProfile(a) => {
            lib::delete_profile(&a.name)?;
            Answer::DeleteProfile(Empty {})
        Command::Mods(_) => Answer::Mods(lib::mods_status()?.into()),
        Command::Modinfo(a) => Answer::Modinfo(lib::inspect_mod(Path::new(&a.path))?.into()),
        Command::StartInstall(a) => {
            Answer::StartInstall(lib::start_install(Path::new(&a.path), lib::wire::install_mode(a.mode()))?.into())
        Command::CancelInstall(a) => Answer::CancelInstall(api::Cancelled {
            cancelled: lib::install::cancel(a.id),
        }),
        Command::ClearInstalls(_) => {
            Answer::ClearInstalls(Empty {})
        Command::Instances(_) => Answer::Instances(api::InstanceList {
            instances: list(lib::list_instances()),
        }),
        Command::Launch(duty) => Answer::Launch(lib::launch(&duty.into())?.into()),
        Command::Stop(a) => Answer::Stop(api::Stopped {
            ended_by_itself: lib::stop_instance(a.pid)?,
        }),
        Command::Log(a) => Answer::Log(api::LogLines {
            lines: lib::log_tail(a.pid, a.lines.unwrap_or(40) as usize)?,
        }),
        Command::Join(a) => {
            let (ok, text) = match ::network::describe_join(&a.text) {
                Ok(d) => (true, d),
                Err(e) => (false, e),
            };
            Answer::Join(api::JoinCheck {
                ok,
                text,
                local: lib::instances::local_hosts().iter().map(api::lan_status).collect(),
            })
        Command::Settings(_) => {
            Answer::Settings(settings(&lib::get_settings()?))
        Command::SaveSettings(changes) => {
                &api::settings_json(&changes.values),
            if changes.values.contains_key("language") {
            if text_of(&changes.values, "pax_models") == Some("realistic") {
            Answer::SaveSettings(settings(&saved))
        Command::PaxPack(_) => Answer::PaxPack(pax_status()),
        Command::InstallPaxPack(_) => {
            Answer::InstallPaxPack(pax_status())
        Command::UpdateCheck(_) => Answer::UpdateCheck(api::UpdateCheck {
            release: crate::updater::latest()?.map(|r| api::GameRelease {
                version: r.version,
                page: r.page,
                notes: r.notes,
                prerelease: r.prerelease,
                size: r.size,
            }),
        }),
        Command::OptionPresets(_) => Answer::OptionPresets(api::OptionPresetList {
            presets: lib::option_presets()
                .map(|(name, values)| api::OptionPreset {
                    name,
                    values: api::setting_values(&values),
                })
        }),
        Command::Keybindings(_) => {
            Answer::Keybindings(lib::wire::key_bindings(lib::keyboard_cfg()?))
        Command::SaveKeybindings(k) => {
            lib::save_keyboard_cfg(&lib::wire::keyboard_cfg(k))?;
            Answer::SaveKeybindings(lib::wire::key_bindings(lib::keyboard_cfg()?))
        Command::Controllers(_) => {
            Answer::Controllers(controllers_now())
        Command::SaveControllers(list) => {
            save_controllers(&list.controllers)?;
            Answer::SaveControllers(controllers_now())
        Command::Preview(a) => Answer::Preview(lib::bus_preview(&a.bus, &a.paint)?),
        Command::Situations(a) => Answer::Situations(api::SituationList {
            situations: lib::saved_situations(&a.map)
                .map(|x| api::SavedSituation {
                    name: x.name,
                    file: x.file.to_string_lossy().into_owned(),
                    time: x.saved,
                })
        }),
        Command::Tutorials(_) => Answer::Tutorials(api::TutorialList {
            tutorials: lib::tutorials()
                .map(|(number, title, text)| api::Tutorial {
                    number: number as u32,
                    title,
                    text,
                })
        }),
        Command::Servers(_) => Answer::Servers(servers()),
        Command::SaveServers(a) => {
            let list = a
                .servers
                .into_iter()
                .map(|s| servers::ServerEntry {
                    name: s.name,
                    address: s.address,
                })
                .collect();
            servers::store(&servers::with_official(list))?;
            Answer::SaveServers(Empty {})
        Command::Version(_) => Answer::Version(api::EngineVersion {
            version: crate::startup::VERSION.into(),
            protocol: api::VERSION.parse().unwrap_or(0),
        }),

fn text_of<'a>(values: &'a HashMap<String, api::SettingValue>, key: &str) -> Option<&'a str> {
    match values.get(key)?.value.as_ref()? {
        setting_value::Value::Text(s) => Some(s),
        _ => None,
    }
}
pub(super) fn pax_status() -> api::PaxPack {
        PaxStatus::Missing => (PaxState::Missing, 0, 0, String::new()),
        PaxStatus::Outdated => (PaxState::Outdated, 0, 0, String::new()),
        PaxStatus::Downloading { done, total } => (PaxState::Downloading, done, total, String::new()),
        PaxStatus::Installing => (PaxState::Installing, 0, 0, String::new()),
        PaxStatus::Installed => (PaxState::Installed, 0, 0, String::new()),
        PaxStatus::Failed(e) => (PaxState::Failed, 0, 0, e),
    api::PaxPack {
        state: state.into(),
        done,
        total,
        message,
        installed: content().as_deref().and_then(crate::pax_pack::installed_version),
        latest: crate::pax_pack::latest().map(|r| api::PaxRelease {
            version: r.version,
            notes: r.notes,
            page: r.page,
            published: r.published,
        }),
    }
fn servers() -> api::ServerList {
    let servers = asked
        .into_iter()
        .map(|(e, answer)| {
            let official = network::official::is_alias(&e.address);
            let answer = answer
                .join()
                .unwrap_or_else(|_| Err("the query stopped on an error".into()));
            match answer {
                Ok(i) => api::ServerInfo {
                    name: if e.name.trim().is_empty() { i.name } else { e.name },
                    address: e.address,
                    official,
                    motd: i.motd,
                    map: i.map,
                    time: i.time,
                    weather: i.weather,
                    players: i.players as u32,
                    max_players: i.max_players as u32,
                    error: None,
                },
                Err(err) => api::ServerInfo {
                    name: if e.name.trim().is_empty() { e.address.clone() } else { e.name },
                    address: e.address,
                    official,
                    error: Some(err),
                    ..Default::default()
                },
            }
        })
        .collect();
    api::ServerList { servers }
const FUNCTIONS: [(Func, AxisFunction); 7] = [
    (Func::Steering, AxisFunction::Steering),
    (Func::Throttle, AxisFunction::Throttle),
    (Func::Brake, AxisFunction::Brake),
    (Func::Clutch, AxisFunction::Clutch),
    (Func::ThrottleBrake, AxisFunction::ThrottleBrake),
    (Func::LookX, AxisFunction::LookX),
    (Func::LookY, AxisFunction::LookY),
const SHAPES: [(AxisShape, i32); 5] = [
    (AxisShape::Linear, 0),
    (AxisShape::Progressive, 8),
    (AxisShape::Degressive, 4),
    (AxisShape::BiProgressive, 8 | 0x10),
    (AxisShape::BiDegressive, 4 | 0x10),
fn controller(d: &DeviceCfg, live: Option<&controllers::Connected>) -> Controller {
    let axes = (0..8)
                .map_or(AxisFunction::None, |(_, function)| *function);
                .map_or(AxisShape::Linear, |(shape, _)| *shape);
            ControllerAxis {
                name: AXES[k].into(),
                value: live
                    .and_then(|c| c.axes.iter().find(|(slot, _)| *slot == k))
                    .map_or(0.0, |(_, v)| *v),
                function: function.into(),
                reversed: d.axes[k].is_some_and(|(_, r)| r),
                shape: shape.into(),
            }
    Controller {
        name: d.name.clone(),
        connected: live.is_some(),
        enabled: d.enabled,
        deadzone: d.deadzone.unwrap_or_else(controllers::global_deadzone),
        force_feedback: d.ff_scale.is_none_or(|(steer, _)| steer > 0.0),
        axes,
        buttons: d
            .buttons
            .iter()
            .map(|(action, number)| api::ButtonBinding {
                action: action.clone(),
                number: number.clone(),
            })
            .collect(),
    }
fn controllers_now() -> api::ControllerList {
    let mut out: Vec<Controller> = configured
        .map(|d| controller(d, live(&d.name)))
            out.push(controller(&d, Some(c)));
    api::ControllerList { controllers: out }
fn apply(d: &mut DeviceCfg, c: &Controller) {
            .find(|(_, function)| *function == axis.function())
            .find(|(shape, _)| *shape == axis.shape())
    d.buttons = c
        .buttons
        .iter()
        .map(|b| (b.action.clone(), b.number.clone()))
        .collect();
fn save_controllers(list: &[Controller]) -> Result<()> {
    use serde_json::json;
        d.buttons = vec![("Horn".into(), "0".into())];
        let mut changed = controller(&d, None);
        assert_eq!(changed.axes[2].function(), AxisFunction::Brake);
        assert!(changed.axes[2].reversed);
        assert_eq!(changed.axes[2].shape(), AxisShape::Progressive);
        assert_eq!(changed.axes[1].function(), AxisFunction::None);
        assert!(changed.force_feedback && !changed.connected);
        changed.axes[2].set_shape(AxisShape::BiDegressive);
        changed.axes[1].set_function(AxisFunction::Clutch);
        assert_eq!(back.buttons, d.buttons);
        let mut changed = controller(&d, None);
    fn the_version_names_this_protocol() {
        let Answer::Version(v) = call(Command::Version(Empty {})).unwrap() else {
            panic!("not a version");
        };
        assert_eq!((v.version.as_str(), v.protocol), (crate::startup::VERSION, 2));
        assert!(call(Command::Shutdown(Empty {})).is_err());
    }
}
