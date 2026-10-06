//! The launcher's settings file as the lists read it, with delayed writes.

use super::*;

/// The launcher's settings file as the lists show it: read once (until something is
/// written), with the keys still waiting to be written on top.
pub(super) fn settings_file() -> std::sync::Arc<serde_json::Value> {
    // lock order: MERGED_SETTINGS, then SETTINGS_CACHE and PENDING_SETTINGS
    let mut merged = MERGED_SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(v) = merged.as_ref() {
        return v.clone();
    }
    let mut v = SETTINGS_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(|| {
            let text =
                std::fs::read_to_string(omsi_launcher_lib::data_dir().join("settings.cfg")).ok();
            omsi_launcher_lib::settings_from_text(text.as_deref())
        })
        .clone();
    let pending = PENDING_SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
    apply_pending(&mut v, &pending.0);
    gfx_overlay(&mut v);
    let v = std::sync::Arc::new(v);
    *merged = Some(v.clone());
    v
}

/// Forget the file as read (and the merged copy of it): it is read again on the next ask.
pub(super) fn invalidate_settings() {
    *SETTINGS_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *MERGED_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

pub(crate) fn gfx_view() -> serde_json::Value {
    use ::config::Value as T;
    let mut o = serde_json::Map::new();
    for (cat, key, _) in ::config::DEFAULTS {
        if *cat != "graphics" {
            continue;
        }
        let v = match ::config::get_setting(cat, key) {
            Some(T::Boolean(b)) => serde_json::json!(b),
            Some(T::Integer(i)) if *key == "map_detail" && i < 0 => serde_json::json!("auto"),
            Some(T::Integer(i)) => serde_json::json!(i),
            Some(T::Float(f)) if f <= 0.0 && matches!(*key, "render_scale" | "view_distance") => {
                serde_json::json!("auto")
            }
            Some(T::Float(f)) if f < 0.0 && *key == "max_obj_dist" => serde_json::json!("auto"),
            Some(T::Float(f)) => serde_json::json!(f),
            Some(T::String(s)) => serde_json::json!(s),
            _ => continue,
        };
        o.insert((*key).to_string(), v);
    }
    serde_json::Value::Object(o)
}

/// Put the config's graphics on top of the launcher's JSON.
pub(crate) fn gfx_overlay(v: &mut serde_json::Value) {
    if let serde_json::Value::Object(o) = gfx_view() {
        for (k, x) in o {
            v[k.as_str()] = x;
        }
    }
}

/// Write one graphics setting (as the JSON pages hold it) into the config.
pub(crate) fn gfx_set(key: &str, v: &serde_json::Value) {
    use ::config::Value as T;
    if !::config::DEFAULTS.iter().any(|(c, k, _)| *c == "graphics" && *k == key) {
        return;
    }
    let text = match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    };
    if text == "auto" {
        ::config::reset_setting("graphics", key);
        return;
    }
    let value = match ::config::get_setting("graphics", key) {
        Some(T::Boolean(_)) => T::Boolean(text == "true" || text == "1"),
        Some(T::String(_)) => T::String(text),
        Some(T::Float(_)) => match text.parse::<f64>() {
            Ok(f) => T::Float(f),
            Err(_) => return,
        },
        _ => match text.parse::<i64>() {
            Ok(i) => T::Integer(i),
            Err(_) => match text.parse::<f64>() {
                Ok(f) => T::Float(f),
                Err(_) => return,
            },
        },
    };
    ::config::set_setting("graphics", key, value);
}

/// Write what changed between `before` and `after` of the graphics into the config file.
pub(crate) fn gfx_store(before: &serde_json::Value, after: &serde_json::Value) {
    let Some(o) = after.as_object() else { return };
    let mut any = false;
    for (k, x) in o {
        if before.get(k) != Some(x) {
            gfx_set(k, x);
            any = true;
        }
    }
    if any {
        let _ = ::config::save();
        invalidate_settings();
    }
}

pub(super) fn store_with(app: &mut App, change: impl FnOnce(&mut serde_json::Value)) {
    flush_settings(true);
    let Ok(mut v) = omsi_launcher_lib::get_settings() else {
        return;
    };
    gfx_overlay(&mut v);
    let before = v.clone();
    change(&mut v);
    gfx_store(&before, &v);
    invalidate_settings();
    match omsi_launcher_lib::save_settings(&v) {
        Ok(()) => reload_settings(app),
        Err(e) => log::warn!("settings not saved: {e:#}"),
    }
}

pub(super) fn reload_settings(app: &mut App) {
    flush_settings(true);
    crate::ui_language(&::config::get_string("ui", "language").unwrap_or_else(|| "ENG".into()));
    sync_live(app);
}

pub(super) fn sync_live(app: &mut App) {
    crate::startup::SOUND_AI.store(
        (::config::get_float("audio", "ai-volume").unwrap_or(1.0) as f32).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
    crate::startup::SOUND_SCENERY.store(
        (::config::get_float("audio", "scenery-volume").unwrap_or(1.0) as f32).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
    ::audio::DOPPLER.store(
        ::config::get_bool("audio", "doppler").unwrap_or(true),
        std::sync::atomic::Ordering::Relaxed,
    );
    if let Some(n) = app.navigator.as_mut() {
        n.arrows = ::config::get_bool("navigator", "arrows").unwrap_or(false);
    }
    if let Some(h) = app.humans.as_mut() {
        h.exact_fare = ::config::get_bool("gameplay", "exact_fare").unwrap_or(true);
        h.boarding = ::config::get_string("gameplay", "boarding").unwrap_or_else(|| "auto".into());
        h.prefer_seats = ::config::get_bool("gameplay", "pax_prefer_seats").unwrap_or(false);
        h.set_ik(app.args.pax_ik.unwrap_or(::config::get_bool("passengers", "ik").unwrap_or(true)));
        h.set_natural(::config::get_string("passengers", "motion").as_deref().unwrap_or("natural") == "natural");
        h.voices = match ::config::get_string("passengers", "voices").as_deref() {
            Some("off") => 2,
            Some("tickets") => 1,
            _ => 0,
        };
    }
}

pub(super) static PENDING_SETTINGS: std::sync::Mutex<(
    Vec<(String, String)>,
    Option<std::time::Instant>,
)> = std::sync::Mutex::new((Vec::new(), None));
pub(super) const SETTINGS_FLUSH_MS: u128 = 250;
pub(super) static MERGED_SETTINGS: std::sync::Mutex<Option<std::sync::Arc<serde_json::Value>>> =
    std::sync::Mutex::new(None);
pub(super) static SETTINGS_CACHE: std::sync::Mutex<Option<serde_json::Value>> =
    std::sync::Mutex::new(None);

/// Write one key of `~/.neoomsi/settings.cfg` (the launcher's file; the other lines
/// stay as they are). The write is delayed a moment and joined with the ones that follow.
pub(crate) fn remember_setting(key: &str, value: &str) {
    {
        let mut p = PENDING_SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
        match p.0.iter_mut().find(|(k, _)| k == key) {
            Some(e) => e.1 = value.to_string(),
            None => p.0.push((key.to_string(), value.to_string())),
        }
    }
    // after the PENDING_SETTINGS lock is released: settings_file takes the two in the other order
    *MERGED_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()) = None;
    flush_settings(false);
}

/// Write the remembered keys out: all of them when `force`, else only when the last write
/// is `SETTINGS_FLUSH_MS` ago. Called every frame, before the file is read and on exit.
pub(crate) fn flush_settings(force: bool) {
    let pending = {
        let mut p = PENDING_SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
        if p.0.is_empty() {
            return;
        }
        if !force
            && p.1
            .is_some_and(|t| t.elapsed().as_millis() < SETTINGS_FLUSH_MS)
        {
            return;
        }
        p.1 = Some(std::time::Instant::now());
        std::mem::take(&mut p.0)
    };
    let Ok(mut v) = omsi_launcher_lib::get_settings() else {
        return;
    };
    apply_pending(&mut v, &pending);
    if let Err(e) = omsi_launcher_lib::save_settings(&v) {
        log::warn!("settings not saved: {e:#}");
    }
    invalidate_settings();
}

pub(super) fn apply_pending(v: &mut serde_json::Value, pending: &[(String, String)]) {
    for (key, value) in pending {
        let parsed: serde_json::Value = value
            .parse::<f64>()
            .map(serde_json::Value::from)
            .unwrap_or_else(|_| serde_json::Value::from(value.as_str()));
        // a switch goes in as true/false, as the launcher's own values are: written as 1 it
        // was read as not set and saved back as its default (the pause menu's options were
        // lost with the next game)
        let parsed = match (&v[key.as_str()], &parsed) {
            (serde_json::Value::Bool(_), serde_json::Value::Number(n)) => {
                serde_json::Value::Bool(n.as_f64().unwrap_or(0.0) > 0.5)
            }
            _ if key == "time_speed" => serde_json::Value::from(value.as_str()),
            _ => parsed,
        };
        v[key.as_str()] = parsed;
    }
}