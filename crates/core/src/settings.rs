// TODO: This class will be removed. Use config-lib api instead.
//! The user's settings: graphics and gameplay switches, kept as `key=value` lines in
//! `~/.neoomsi/settings.cfg` (the launcher writes the same file). Anything missing
//! keeps its default, so an old file never breaks a new build.

use std::path::PathBuf;

/// Version of the settings file (`version=`); files without it are version 1.
pub const SETTINGS_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    // FYI: we don't remove "Migrated" stuff because this shit is cursed, don't ask me why
    pub msaa: u32, // TODO: Migrate to new config lib | category: graphics | default value: 4
    pub anisotropy: u16, // TODO: Migrate to new config lib | category: graphics | default value: 8
    pub ssao: bool, // TODO: Migrate to new config lib | category: graphics | default value: true
    pub shadows: bool, // TODO: Migrate to new config lib | category: graphics | default value: true
    pub shadow_size: u32, // TODO: Migrate to new config lib | category: graphics | default value: 2048
    pub shadow_blobs: bool, // TODO: Migrate to new config lib | category: graphics | default value: true
    pub navigator: bool, // TODO: Migrate to new config lib | category: ui | default value: true
    pub ui_opacity: f32, // TODO: Migrate to new config lib | category: ui | default value: 0.85
    pub navigator_corner: String, // TODO: Migrate to new config lib | category: ui | default value: "bottom-left"
    pub boarding: String, // TODO: Migrate to new config lib | category: gameplay | default value: "auto"
    pub pax_prefer_seats: bool, // TODO: Migrate to new config lib | category: gameplay | default value: false
    pub detail_textures: bool, // TODO: Migrate to new config lib | category: graphics | default value: true
    pub exact_fare: bool, // TODO: Migrate to new config lib | category: gameplay | default value: true
    pub enhanced: bool, // TODO: Migrate to new config lib | category: graphics | default value: false
    pub graphics: String, // TODO: Migrate to new config lib | category: graphics | default value: "vanilla_plus"
    pub fullscreen: bool, // TODO: Migrate to new config lib | category: graphics | default value: false
    pub vsync: bool, // TODO: Migrate to new config lib | category: graphics | default value: true
    pub volume: f32, // Migrated
    pub post_aa: String, // TODO: Migrate to new config lib | category: graphics | default value: "fxaa"
    pub maintenance: u8, // TODO: Migrate to new config lib | category: gameplay | default value: 0
    pub ai_unsched_factor: f32, // TODO: Migrate to new config lib | category: ai | default value: 1.0
    pub ai_max_scheduled: u32, // TODO: Migrate to new config lib | category: ai | default value: 0
    pub ai_max_parked: i32, // TODO: Migrate to new config lib | category: ai | default value: 0
    pub collision_vehicles: bool, // TODO: Migrate to new config lib | category: gameplay | default value: true
    pub collision_objects: bool, // TODO: Migrate to new config lib | category: gameplay | default value: true
    pub collision_pedestrians: bool, // TODO: Migrate to new config lib | category: gameplay | default value: true
    pub head_movement: bool, // TODO: Migrate to new config lib | category: gameplay | default value: true
    pub driverview_smooth: bool, // TODO: Migrate to new config lib | category: gameplay | default value: true
    pub hands_in_cab: bool, // TODO: Migrate to new config lib | category: gameplay | default value: false
    pub alt_view: bool, // Migrated
    pub free_look: bool, // TODO: Migrate to new config lib | category: gameplay | default value: false
    pub crosshair: bool, // Migrated
    pub render_scale: f32, // TODO: Migrate to new config lib | category: graphics | default value: 0.0
    pub language: String, // TODO: Migrate to new config lib | category: ui | default value: "ENG"
    pub units: String, // TODO: Migrate to new config lib | category: ui | default value: "metric"
    pub pax_voices: String, // TODO: Migrate to new config lib | category: passengers | default value: "all"
    pub pax_models: String, // TODO: Migrate to new config lib | category: passengers | default value: "omsi"
    pub pax_motion: String, // TODO: Migrate to new config lib | category: passengers | default value: "natural"
    pub pax_ik: bool, // TODO: Migrate to new config lib | category: passengers | default value: true
    pub nav_arrows: bool, // TODO: Migrate to new config lib | category: navigator | default value: false
    pub nav_ai: bool, // TODO: Migrate to new config lib | category: navigator | default value: true
    pub nav_topbar: bool, // TODO: Migrate to new config lib | category: navigator | default value: true
    pub nav_turn: bool, // TODO: Migrate to new config lib | category: navigator | default value: true
    pub nav_stoplist: bool, // TODO: Migrate to new config lib | category: navigator | default value: true
    pub nav_stops_ext: bool, // TODO: Migrate to new config lib | category: navigator | default value: false
    pub texture_compression: bool, // TODO: Migrate to new config lib | category: graphics | default value: true
    pub texture_memory: u32, // TODO: Migrate to new config lib | category: graphics | default value: 0
    pub auto_clutch: bool, // TODO: Migrate to new config lib | category: gameplay | default value: true
    pub momentary_gears: bool, // TODO: Migrate to new config lib | category: gameplay | default value: false
    pub auto_ibis: bool, // TODO: Migrate to new config lib | category: gameplay | default value: false
    pub auto_shift: bool, // TODO: Migrate to new config lib | category: gameplay | default value: false
    pub min_obj_size: f32, // TODO: Migrate to new config lib | category: graphics | default value: 0.013
    pub map_detail: i16, // TODO: Migrate to new config lib | category: graphics | default value: -1
    pub max_obj_dist: f32, // TODO: Migrate to new config lib | category: graphics | default value: -1.0
    pub max_fps: u32, // TODO: Migrate to new config lib | category: graphics | default value: 0
    pub chat: bool, // TODO: Migrate to new config lib | category: ui | default value: true
    pub tooltips: bool, // TODO: Migrate to new config lib | category: ui | default value: true
    pub name_tags: bool, // TODO: Migrate to new config lib | category: ui | default value: true
    pub driver: bool, // TODO: Migrate to new config lib | category: gameplay | default value: true
    pub show_fps: bool, // TODO: Migrate to new config lib | category: ui | default value: false
    pub notes: bool, // TODO: Migrate to new config lib | category: ui | default value: true
    pub ui_scale: f32, // TODO: Migrate to new config lib | category: ui | default value: 1.0
    pub ui_scale_window: bool, // TODO: Migrate to new config lib | category: ui | default value: true
    pub clouds: bool, // TODO: Migrate to new config lib | category: graphics | default value: true
    pub pax_density: f32, // TODO: Migrate to new config lib | category: passengers | default value: 1.0
    pub vol_ai: f32, // TODO: Migrate to new config lib | category: audio | default value: 1.0
    pub vol_scenery: f32, // TODO: Migrate to new config lib | category: audio | default value: 1.0
    pub mirror_size: u32, // TODO: Migrate to new config lib | category: graphics | default value: 256
    pub mirror_refresh: String, // TODO: Migrate to new config lib | category: graphics | default value: "full"
    pub doppler: bool, // TODO: Migrate to new config lib | category: audio | default value: true
    pub time_speed: f64, // TODO: Migrate to new config lib | category: gameplay | default value: 1.0
    pub time_sync: bool, // TODO: Migrate to new config lib | category: gameplay | default value: false
    pub metar_sync: bool, // TODO: Migrate to new config lib | category: gameplay | default value: false
    pub metar_station: String, // TODO: Migrate to new config lib | category: gameplay | default value: ""
    pub shadow_casters: String, // TODO: Migrate to new config lib | category: graphics | default value: "all"
    pub steering_linear: bool, // TODO: Migrate to new config lib | category: controls | default value: false
    pub old_steering: bool, // TODO: Migrate to new config lib | category: controls | default value: false
    pub red_steer_spd: bool, // TODO: Migrate to new config lib | category: controls | default value: false
    pub reflections: bool, // TODO: Migrate to new config lib | category: graphics | default value: true
    pub led_glow: u8, // TODO: Migrate to new config lib | category: graphics | default value: 6
    pub nightmap_glow: u8, // TODO: Migrate to new config lib | category: graphics | default value: 6
    pub atmosphere_brightness: f32, // TODO: Migrate to new config lib | category: graphics | default value: 1.0
    pub led_mips: f32, // TODO: Migrate to new config lib | category: graphics | default value: 1.3
    pub mouse_sens: f32, // TODO: Migrate to new config lib | category: controls | default value: 1.0
    pub stick_sens: f32, // TODO: Migrate to new config lib | category: controls | default value: 0.25
    pub steer_center: bool, // TODO: Migrate to new config lib | category: controls | default value: true
    pub graphics_api: String, // TODO: Migrate to new config lib | category: graphics | default value: "auto"
    pub brake_hold: bool, // TODO: Migrate to new config lib | category: controls | default value: true
    pub mouse_steering: bool, // TODO: Migrate to new config lib | category: controls | default value: false
    pub mouse_right_off: bool, // TODO: Migrate to new config lib | category: controls | default value: false
    pub look_sens: f32, // Migrated
    pub blinker_cancel: bool, // TODO: Migrate to new config lib | category: controls | default value: true
    pub wheel_range: f32, // TODO: Migrate to new config lib | category: controls | default value: 900.0
    pub wheel_lock: f32, // TODO: Migrate to new config lib | category: controls | default value: 0.0
    pub fov: f32, // Migrated
    pub camera_collision: bool, // Migrated
    pub steer_look: bool, // Migrated
    pub steer_look_angle: f32, // Migrated
    pub steer_look_response: f32, // Migrated
    pub pedal_throttle: f32, // TODO: Migrate to new config lib | category: controls | default value: 1.0
    pub pedal_brake: f32, // TODO: Migrate to new config lib | category: controls | default value: 1.0
    pub seat: [f32; 3], // Migrated
    pub head_tracking: bool, // Migrated
    pub head_tracking_port: u16, // Migrated
    pub head_tracking_invert: String, // Migrated
    pub discord_status: bool, // TODO: Migrate to new config lib | category: integration | default value: true
    pub discord_app_id: String, // TODO: Migrate to new config lib | category: integration | default value: ""
}

/// A pedal's last few per cent of travel are its end: a wheel's pedal on the floor reads
/// 0.93..0.99, and the scripts ask for the ends exactly - the LiAZ/PAZ gearboxes put a gear
/// in only at `(L.L.clutch) 1 =` and part the engine from the wheels only above 0.95, so a
/// clutch held down to the floor still dragged and the engine died at every stop.
pub fn pedal_ends(v: f32) -> f32 {
    if v >= 0.96 {
        1.0
    } else if v <= 0.02 {
        0.0
    } else {
        v
    }
}

/// A pedal as the settings shape it: `v` 0..1 through the response curve of `strength`.
pub fn pedal_curve(v: f32, strength: f32) -> f32 {
    let g = strength.clamp(0.25, 4.0);
    v.clamp(0.0, 1.0).powf(1.0 / g)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            msaa: 4,
            anisotropy: 8,
            ssao: true,
            shadows: true,
            shadow_size: 2048,
            shadow_blobs: true,
            navigator: true,
            ui_opacity: 0.85,
            notes: true,
            ui_scale: 1.0,
            ui_scale_window: true,
            navigator_corner: "bottom-left".into(),
            boarding: "auto".into(),
            pax_prefer_seats: false,
            detail_textures: true,
            exact_fare: true,
            enhanced: false,
            graphics: "vanilla_plus".into(),
            fullscreen: false,
            vsync: true,
            volume: 0.6,
            post_aa: "fxaa".into(),
            render_scale: 0.0,
            language: "ENG".into(),
            units: "metric".into(),
            pax_voices: "all".into(),
            pax_models: "omsi".into(),
            pax_motion: "natural".into(),
            pax_ik: true,
            nav_arrows: false,
            nav_ai: true,
            nav_topbar: true,
            nav_turn: true,
            nav_stoplist: true,
            nav_stops_ext: false,
            texture_compression: true,
            texture_memory: 0,
            auto_clutch: true,
            auto_ibis: false,
            momentary_gears: false,
            auto_shift: false,
            min_obj_size: 0.013,
            map_detail: -1,
            max_obj_dist: -1.0,
            max_fps: 0,
            chat: true,
            tooltips: true,
            name_tags: true,
            show_fps: false,
            clouds: true,
            pax_density: 1.0,
            vol_ai: 1.0,
            vol_scenery: 1.0,
            mirror_size: 256,
            mirror_refresh: "full".into(),
            doppler: true,
            driver: true,
            maintenance: 0,
            ai_unsched_factor: 1.0,
            ai_max_scheduled: 0,
            ai_max_parked: 0,
            collision_vehicles: true,
            collision_objects: true,
            collision_pedestrians: true,
            head_movement: true,
            driverview_smooth: true,
            hands_in_cab: false,
            alt_view: true,
            free_look: false,
            crosshair: true,
            time_speed: 1.0,
            time_sync: false,
            metar_sync: false,
            metar_station: String::new(),
            shadow_casters: "all".into(),
            steering_linear: false,
            old_steering: false,
            red_steer_spd: false,
            reflections: true,
            led_glow: 6,
            nightmap_glow: 6,
            atmosphere_brightness: 1.0,
            led_mips: 1.3,
            mouse_sens: 1.0,
            stick_sens: 0.25,
            steer_center: true,
            graphics_api: "auto".into(),
            brake_hold: true,
            mouse_steering: false,
            mouse_right_off: false,
            look_sens: 1.0,
            blinker_cancel: true,
            wheel_range: 900.0,
            wheel_lock: 0.0,
            fov: 0.0,
            camera_collision: true,
            steer_look: false,
            steer_look_angle: 30.0,
            steer_look_response: 0.25,
            pedal_throttle: 1.0,
            pedal_brake: 1.0,
            seat: [0.0; 3],
            head_tracking: false,
            head_tracking_port: 4242,
            head_tracking_invert: String::new(),
            discord_status: true,
            discord_app_id: String::new(),
        }
    }
}

impl Settings {
    /// `~/.neoomsi/settings.cfg` (or `%USERPROFILE%` on Windows).
    pub fn path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(PathBuf::from(home).join(".neoomsi").join("settings.cfg"))
    }

    pub fn load() -> Settings {
        let Some(p) = Self::path() else {
            return Settings::default();
        };
        let mut text = std::fs::read_to_string(&p).unwrap_or_default();
        // OMSI_GRAPHICS=vanilla|vanilla_plus|enhanced: another renderer for one run
        if let Ok(g) = ::legacy_config::env::var("OMSI_GRAPHICS") {
            text.push_str(&format!("\ngraphics={g}\n"));
        }
        let mut s = Self::from_text(&text);
        // OMSI_SAFE_GPU=<n>: the game was started again after its graphics device was lost
        // (see `App::restart_after_device_loss`): lighter on the card each time
        if let Some(n) = ::legacy_config::env::var("OMSI_SAFE_GPU")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|n| *n > 0)
        {
            s.apply_safe_gpu(n);
        }
        s
    }

    /// The settings a `settings.cfg` text describes (anything missing keeps its default).
    pub fn from_text(text: &str) -> Settings {
        let mut s = Settings::default();
        let mut version = 0u32;
        let mut graphics: Option<String> = None;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            let b =
                |v: &str| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
            match k.as_str() {
                "version" => version = v.parse().unwrap_or(0),
                "msaa" => s.msaa = v.parse().unwrap_or(s.msaa),
                "anisotropy" | "af" => s.anisotropy = v.parse().unwrap_or(s.anisotropy),
                "ssao" | "ambient_occlusion" => s.ssao = b(v),
                "shadows" => s.shadows = b(v),
                "shadow_size" => s.shadow_size = v.parse().unwrap_or(s.shadow_size),
                "shadow_blobs" => s.shadow_blobs = b(v),
                "navigator" => s.navigator = b(v),
                "ui_opacity" | "navigator_opacity" => {
                    s.ui_opacity = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.2, 1.0))
                        .unwrap_or(s.ui_opacity)
                }
                "navigator_corner" => s.navigator_corner = v.to_ascii_lowercase(),
                "boarding" => s.boarding = v.to_ascii_lowercase(),
                "pax_prefer_seats" => s.pax_prefer_seats = b(v),
                "detail_textures" | "fractal" => s.detail_textures = b(v),
                "exact_fare" => s.exact_fare = b(v),
                "enhanced" => s.enhanced = b(v),
                "graphics" | "renderer" => graphics = Some(graphics_mode(v).to_string()),
                "fullscreen" => s.fullscreen = b(v),
                "vsync" => s.vsync = b(v),
                "volume" => s.volume = v.parse().unwrap_or(s.volume),
                // "auto", a fraction (0.75) or a percentage (75)
                "render_scale" => {
                    s.render_scale = if v.eq_ignore_ascii_case("auto") {
                        0.0
                    } else if v.eq_ignore_ascii_case("off") {
                        1.0
                    } else {
                        match v.trim_end_matches('%').parse::<f32>() {
                            Ok(x) if x > 1.5 => (x / 100.0).clamp(0.5, 1.0),
                            Ok(x) if x > 0.0 => x.clamp(0.5, 1.0),
                            Ok(_) => 0.0,
                            Err(_) => s.render_scale,
                        }
                    }
                }
                "language" | "lang" => s.language = crate::describe::language_code(v),
                "units" => {
                    s.units = match v.to_ascii_lowercase().as_str() {
                        "uk" | "british" => "uk".into(),
                        "imperial" => "imperial".into(),
                        _ => "metric".into(),
                    }
                }
                "pax_voices" => {
                    s.pax_voices = match v.to_ascii_lowercase().as_str() {
                        "tickets" => "tickets".into(),
                        "off" | "0" | "none" => "off".into(),
                        _ => "all".into(),
                    }
                }
                "pax_ik" | "ik" => s.pax_ik = b(v),
                "pax_motion" => {
                    s.pax_motion = if v.eq_ignore_ascii_case("omsi") {
                        "omsi"
                    } else {
                        "natural"
                    }
                        .into()
                }
                "pax_models" => {
                    s.pax_models = if v.eq_ignore_ascii_case("realistic") {
                        "realistic"
                    } else {
                        "omsi"
                    }
                        .into()
                }
                "nav_arrows" => s.nav_arrows = b(v),
                "nav_ai" => s.nav_ai = b(v),
                "nav_topbar" => s.nav_topbar = b(v),
                "nav_turn" => s.nav_turn = b(v),
                "nav_stoplist" => s.nav_stoplist = b(v),
                "nav_stops_ext" => s.nav_stops_ext = b(v),
                "texture_compression" => s.texture_compression = b(v),
                "auto_clutch" | "automatic_clutch" => s.auto_clutch = b(v),
                "momentary_gears" | "gear_buttons_hold" => s.momentary_gears = b(v),
                "auto_shift" => s.auto_shift = b(v),
                "auto_ibis" => s.auto_ibis = b(v),
                "map_detail" | "maxcomplexity_map" => {
                    s.map_detail = if v.eq_ignore_ascii_case("auto") {
                        -1
                    } else {
                        v.parse::<i16>()
                            .ok()
                            .filter(|n| (-1..=255).contains(n))
                            .unwrap_or(s.map_detail)
                    };
                }
                "min_obj_size" | "performance_minobjsize" => {
                    s.min_obj_size = v
                        .parse::<f32>()
                        .map(|x| x.clamp(0.0, 0.2))
                        .unwrap_or(s.min_obj_size)
                }
                "max_obj_dist" | "performance_maxobjdist" => {
                    s.max_obj_dist = if v.eq_ignore_ascii_case("off") {
                        0.0
                    } else if v.eq_ignore_ascii_case("auto") {
                        -1.0
                    } else {
                        v.parse::<f32>()
                            .map(|x| x.max(0.0))
                            .unwrap_or(s.max_obj_dist)
                    }
                }
                "max_fps" | "maxfps" => {
                    s.max_fps = v
                        .parse::<f32>()
                        .map(|x| x.max(0.0) as u32)
                        .unwrap_or(s.max_fps);
                    // a phone given the PC OMSI's 30 by the settings import: 60
                    if cfg!(target_os = "android") && s.max_fps == 30 {
                        s.max_fps = 60;
                    }
                }
                "chat" => s.chat = b(v),
                "tooltips" | "mouseover" => s.tooltips = b(v),
                "name_tags" | "nametags" => s.name_tags = b(v),
                "driver" => s.driver = b(v),
                "show_fps" | "fps" => s.show_fps = b(v),
                "clouds" => s.clouds = b(v),
                "pax_density" | "aipassfactor" => {
                    s.pax_density = v
                        .trim_end_matches('%')
                        .parse::<f32>()
                        .map(|x| if x > 5.0 { x / 100.0 } else { x })
                        .map(|x| x.clamp(0.0, 3.0))
                        .unwrap_or(s.pax_density)
                }
                "vol_ai" => {
                    s.vol_ai = v
                        .parse::<f32>()
                        .map(|x| x.clamp(0.0, 1.0))
                        .unwrap_or(s.vol_ai)
                }
                "vol_scenery" => {
                    s.vol_scenery = v
                        .parse::<f32>()
                        .map(|x| x.clamp(0.0, 1.0))
                        .unwrap_or(s.vol_scenery)
                }
                "doppler" | "sound_doppler" => s.doppler = b(v),
                "mirror_size" => {
                    s.mirror_size = v
                        .parse::<u32>()
                        .map(|x| {
                            if x == 0 {
                                0
                            } else {
                                x.clamp(64, 2048).next_power_of_two()
                            }
                        })
                        .unwrap_or(s.mirror_size)
                }
                "mirror_refresh" | "performance_realreflexions" => {
                    s.mirror_refresh = match v.trim().to_ascii_lowercase().as_str() {
                        "off" | "none" => "off",
                        "eco" | "economy" => "eco",
                        _ => "full",
                    }
                        .into()
                }
                "texture_memory" | "texmemlimit" => {
                    s.texture_memory = v
                        .parse::<f32>()
                        .map(|x| x.max(0.0) as u32)
                        .unwrap_or(s.texture_memory)
                }
                "maintenance" | "wear_lifespan" => {
                    s.maintenance = v.parse::<u8>().map(|x| x.min(4)).unwrap_or(s.maintenance)
                }
                "ai_unsched_factor" | "aiunschedfactor" => {
                    s.ai_unsched_factor = v
                        .trim_end_matches('%')
                        .parse::<f32>()
                        .map(|x| (x / 100.0).clamp(0.0, 3.0))
                        .unwrap_or(s.ai_unsched_factor)
                }
                "ai_max_scheduled" | "aimaxcountscheduled" => {
                    s.ai_max_scheduled = v.parse().unwrap_or(s.ai_max_scheduled)
                }
                "ai_max_parked" | "aimaxcountparked" => {
                    s.ai_max_parked = v
                        .parse::<i32>()
                        .map(|x| x.max(-1))
                        .unwrap_or(s.ai_max_parked)
                }
                "collision_vehicles" => s.collision_vehicles = b(v),
                "collision_objects" => s.collision_objects = b(v),
                "collision_pedestrians" => s.collision_pedestrians = b(v),
                "head_movement" | "driverview_moving" => s.head_movement = b(v),
                "driverview_smooth" => s.driverview_smooth = b(v),
                "hands_in_cab" => s.hands_in_cab = b(v),
                "alt_view" => s.alt_view = b(v),
                "free_look" => s.free_look = b(v),
                "crosshair" => s.crosshair = b(v),
                "time_speed" => {
                    s.time_speed = v
                        .trim_start_matches(['x', 'X'])
                        .parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(1.0, 30.0))
                        .unwrap_or(s.time_speed)
                }
                "time_sync" | "real_time_sync" => s.time_sync = b(v),
                "metar_sync" => s.metar_sync = b(v),
                "metar_station" => {
                    s.metar_station = v
                        .trim()
                        .chars()
                        .filter(|c| c.is_ascii_alphabetic())
                        .take(4)
                        .collect::<String>()
                        .to_ascii_uppercase()
                }
                "reflections" | "envmap" => s.reflections = b(v),
                "led_glow" => {
                    s.led_glow = v
                        .trim()
                        .parse::<i32>()
                        .map(|x| x.clamp(0, 15) as u8)
                        .unwrap_or(s.led_glow)
                }
                "nightmap_glow" => {
                    s.nightmap_glow = v
                        .trim()
                        .parse::<i32>()
                        .map(|x| x.clamp(0, 15) as u8)
                        .unwrap_or(s.nightmap_glow)
                }
                "atmosphere_brightness" => {
                    s.atmosphere_brightness = v
                        .trim()
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.0, 2.0))
                        .unwrap_or(s.atmosphere_brightness)
                }
                "led_mips" => {
                    s.led_mips = v
                        .trim()
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.0, 4.0))
                        .unwrap_or(s.led_mips)
                }
                "graphics_api" => s.graphics_api = v.trim().to_ascii_lowercase(),
                "steering_linear" => s.steering_linear = b(v),
                "old_steering" => s.old_steering = b(v),
                "red_steer_spd" => s.red_steer_spd = b(v),
                "brake_hold" => s.brake_hold = b(v),
                "mouse_steering" => s.mouse_steering = b(v),
                "mouse_right_off" => s.mouse_right_off = b(v),
                "blinker_cancel" => s.blinker_cancel = b(v),
                "look_sens" => {
                    s.look_sens = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.1, 2.0))
                        .unwrap_or(s.look_sens)
                }
                "wheel_range" => {
                    s.wheel_range = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(90.0, 2880.0))
                        .unwrap_or(s.wheel_range)
                }
                "wheel_lock" => {
                    s.wheel_lock = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| if x < 45.0 { 0.0 } else { x.min(2880.0) })
                        .unwrap_or(s.wheel_lock)
                }
                "camera_collision" => s.camera_collision = b(v),
                "steer_look" => s.steer_look = b(v),
                "steer_look_angle" => {
                    s.steer_look_angle = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.0, 60.0))
                        .unwrap_or(s.steer_look_angle)
                }
                "steer_look_response" => {
                    s.steer_look_response = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.05, 1.0))
                        .unwrap_or(s.steer_look_response)
                }
                "head_tracking" => s.head_tracking = b(v),
                "head_tracking_invert" => s.head_tracking_invert = v.to_ascii_lowercase(),
                "discord_status" => s.discord_status = b(v),
                "discord_app_id" => s.discord_app_id = v.trim().to_string(),
                "head_tracking_port" => {
                    s.head_tracking_port = v
                        .parse::<u16>()
                        .ok()
                        .filter(|p| *p > 0)
                        .unwrap_or(s.head_tracking_port)
                }
                "pedal_throttle" => {
                    s.pedal_throttle = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.25, 4.0))
                        .unwrap_or(s.pedal_throttle)
                }
                "pedal_brake" => {
                    s.pedal_brake = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.25, 4.0))
                        .unwrap_or(s.pedal_brake)
                }
                "seat_x" | "seat_y" | "seat_z" => {
                    let k = (k.as_bytes()[5] - b'x') as usize;
                    s.seat[k] = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(-1.5, 1.5))
                        .unwrap_or(0.0);
                }
                "fov" => {
                    s.fov = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| if x < 20.0 { 0.0 } else { x.min(120.0) })
                        .unwrap_or(s.fov)
                }
                "mouse_sens" => {
                    s.mouse_sens = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.1, 3.0))
                        .unwrap_or(s.mouse_sens)
                }
                "stick_sens" => {
                    s.stick_sens = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.1, 2.0))
                        .unwrap_or(s.stick_sens)
                }
                "steer_center" => s.steer_center = b(v),
                "ui_scale_window" => s.ui_scale_window = b(v),
                "notes" => s.notes = b(v),
                "ui_scale" => {
                    s.ui_scale = v
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(|x| x.clamp(0.5, 2.0))
                        .unwrap_or(s.ui_scale)
                }
                "shadow_casters" => {
                    s.shadow_casters = if v.eq_ignore_ascii_case("omsi") {
                        "omsi".into()
                    } else {
                        "all".into()
                    }
                }
                "post_aa" => {
                    s.post_aa = if matches!(
                        v.to_ascii_lowercase().as_str(),
                        "off" | "0" | "none" | "false"
                    ) {
                        "off".into()
                    } else {
                        "fxaa".into()
                    }
                }
                _ => {}
            }
        }
        // `graphics` decides; a file without it (older builds) says only `enhanced`, and
        // its vanilla renderer is what is now called Vanilla+
        s.graphics = graphics.unwrap_or_else(|| {
            if s.enhanced {
                "enhanced"
            } else {
                "vanilla_plus"
            }
                .to_string()
        });
        s.enhanced = s.graphics == "enhanced";
        if s.classic() {
            s.ssao = false;
            s.detail_textures = false;
        }
        // Files older than version 2 say `boarding=pay` because that was the launcher's
        // default, not because anybody chose it: passengers then stood at the cash desk
        // waiting for a driver who did not know he had to sell them a ticket.
        if version < SETTINGS_VERSION && s.boarding == "pay" {
            log::info!(
                "settings: boarding=pay from an old settings file taken as auto (choose pay again in the launcher to keep it)"
            );
            s.boarding = "auto".into();
        }
        s
    }

    /// Vanilla graphics: the picture as OMSI 2 draws it.
    pub fn classic(&self) -> bool {
        self.graphics == "vanilla"
    }

    /// How far objects are drawn (m, 0 = no limit): `max_obj_dist`, or when that is `auto`
    /// the visible distance the launcher sets, else the original's 900 m.
    pub fn object_distance(&self) -> f32 {
        if self.max_obj_dist >= 0.0 {
            self.max_obj_dist
        } else {
            view_distance().map(|v| v as f32).unwrap_or(900.0)
        }
    }

    /// `auto` or the fraction, as the file and the log write it.
    pub fn render_scale_text(&self) -> String {
        if self.render_scale > 0.0 {
            format!("{}", self.render_scale)
        } else {
            "auto".into()
        }
    }

    /// Lighter graphics after the graphics device was lost `n` times this session: no
    /// multisampling, no SSAO, smaller shadow and mirror maps, fewer textures kept; a second
    /// loss also a smaller picture and no shadows.
    pub fn apply_safe_gpu(&mut self, n: u32) {
        self.msaa = 1;
        self.ssao = false;
        self.shadow_size = self.shadow_size.min(2048);
        self.mirror_size = self.mirror_size.min(256);
        let budget = if self.texture_memory > 0 {
            self.texture_memory
        } else {
            1200
        };
        self.texture_memory = (budget * 2 / 3).max(400);
        if n >= 2 {
            self.shadows = false;
            self.shadow_size = 1024;
            self.render_scale = if self.render_scale > 0.0 {
                self.render_scale.min(0.75)
            } else {
                0.75
            };
            self.texture_memory = self.texture_memory.min(700);
            self.mirror_size = self.mirror_size.min(128);
        }
        log::warn!(
            "safer graphics after a lost graphics device ({n}): msaa 1, SSAO off, shadows {} ({}), textures {} MB, render scale {}",
            self.shadows,
            self.shadow_size,
            self.texture_memory,
            self.render_scale_text()
        );
    }

    pub fn render_options(&self) -> ::render::RenderOptions {
        ::render::RenderOptions {
            msaa: self.msaa,
            anisotropy: self.anisotropy,
            shadow_size: self.shadow_size,
            ssao: self.ssao,
            render_scale: self.render_scale,
            compress_textures: self.texture_compression,
            fxaa: self.post_aa != "off",
            min_obj_size: self.min_obj_size,
            max_obj_dist: self.object_distance(),
            omsi_shadow_casters: self.shadow_casters == "omsi",
            shadow_blobs: self.shadow_blobs,
            reflections: self.reflections,
            no_enhanced: graphics_mode(&self.graphics) != "enhanced",
        }
    }
}

/// `vanilla`, `vanilla_plus` or `enhanced` from the ways a file may spell them.
pub fn graphics_mode(v: &str) -> &'static str {
    match v
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' '], "_")
        .as_str()
    {
        "enhanced" | "1" => "enhanced",
        "vanilla" | "classic" | "original" | "omsi" | "omsi2" | "omsi_2" => "vanilla",
        _ => "vanilla_plus",
    }
}

/// Snapshot the selected map detail when opening a map; tile workers use this value
/// throughout a session. Changing it in the UI takes effect on the next map load.
pub fn map_detail(root: &std::path::Path) -> u8 {
    let text = Settings::path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    resolve_map_detail(&Settings::from_text(&text), root)
}

fn resolve_map_detail(settings: &Settings, root: &std::path::Path) -> u8 {
    if settings.map_detail >= 0 {
        settings.map_detail as u8
    } else {
        ::content::options::Options::load(&root.join("options.cfg"))
            .map(|o| o.i32("maxcomplexity_map", 2).clamp(0, 255) as u8)
            .unwrap_or(2)
    }
}

/// `view_distance=<metres>` of the settings file: how far around the camera the map's tiles
/// are kept loaded (OMSI's "visible distance"). None when the file does not say.
pub fn view_distance() -> Option<f64> {
    let text = std::fs::read_to_string(Settings::path()?).ok()?;
    text.lines()
        .filter_map(|l| l.trim().split_once('='))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("view_distance"))
        .and_then(|(_, v)| v.trim().parse::<f64>().ok())
        .filter(|v| *v > 0.0)
}

impl Settings {
    /// The player's bus's `wearlifespan` for the maintenance condition (OMSI's table).
    pub fn wear_lifespan(&self) -> f32 {
        [1.5e6, 0.01, 0.1, 1.0, 10.0][self.maintenance.min(4) as usize]
    }
}

/// The player's own turn of a bus's mirrors (degrees yaw, pitch per `[add_camera_reflexion]`),
/// kept per `.bus` file in `~/.neoomsi/mirrors.cfg` as `<bus file>|<mirror>=<yaw>,<pitch>`.
pub fn mirror_offsets(bus: &std::path::Path) -> Vec<[f32; 2]> {
    let key = bus.to_string_lossy().to_ascii_lowercase();
    let Some(p) = Settings::path().map(|p| p.with_file_name("mirrors.cfg")) else {
        return Vec::new();
    };
    let text = std::fs::read_to_string(p).unwrap_or_default();
    let mut out: Vec<[f32; 2]> = Vec::new();
    for line in text.lines() {
        let Some((k, v)) = line.rsplit_once('=') else {
            continue;
        };
        let Some((file, i)) = k.rsplit_once('|') else {
            continue;
        };
        let (Ok(i), Some((y, p))) = (i.trim().parse::<usize>(), v.split_once(',')) else {
            continue;
        };
        if file.trim().to_ascii_lowercase() != key || i > 64 {
            continue;
        }
        if out.len() <= i {
            out.resize(i + 1, [0.0; 2]);
        }
        out[i] = [
            y.trim().parse().unwrap_or(0.0),
            p.trim().parse().unwrap_or(0.0),
        ];
    }
    out
}

/// Keep a bus's mirror turns (see [`mirror_offsets`]).
pub fn save_mirror_offsets(bus: &std::path::Path, offsets: &[[f32; 2]]) {
    let key = bus.to_string_lossy().to_ascii_lowercase();
    let Some(p) = Settings::path().map(|p| p.with_file_name("mirrors.cfg")) else {
        return;
    };
    let text = std::fs::read_to_string(&p).unwrap_or_default();
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| {
            l.rsplit_once('=')
                .and_then(|(k, _)| k.rsplit_once('|'))
                .is_none_or(|(f, _)| f.trim().to_ascii_lowercase() != key)
        })
        .map(str::to_string)
        .collect();
    for (i, o) in offsets.iter().enumerate() {
        if o[0].abs() > 0.01 || o[1].abs() > 0.01 {
            lines.push(format!(
                "{}|{i}={:.1},{:.1}",
                bus.to_string_lossy(),
                o[0],
                o[1]
            ));
        }
    }
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&p, lines.join("\n") + "\n");
}