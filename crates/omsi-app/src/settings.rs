//! The user's settings: graphics and gameplay switches, kept as `key=value` lines in
//! `~/.openomsi/settings.cfg` (the launcher writes the same file). Anything missing
//! keeps its default, so an old file never breaks a new build.

use std::path::PathBuf;

/// Version of the settings file (`version=`); files without it are version 1.
pub const SETTINGS_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Samples per pixel: 1, 2, 4 or 8.
    pub msaa: u32,
    /// Anisotropic filtering 1..16.
    pub anisotropy: u16,
    pub ssao: bool,
    pub shadows: bool,
    pub shadow_size: u32,
    /// The route navigator in the lower right corner.
    pub navigator: bool,
    /// Navigator opacity 0..1 (it has no background; this scales the whole thing).
    pub navigator_opacity: f32,
    /// Which corner the navigator sits in: `bottom-left` (default), `bottom-right`,
    /// `top-left` or `top-right`.
    pub navigator_corner: String,
    /// How passengers board: `auto` - they pay at the cash desk and take the ticket
    /// themselves; `pay` - they wait at the desk for the driver to sell the ticket (the
    /// ticket key or the printer); `walk` - they just walk into the saloon (a flat-fare
    /// or ticket-machine service).
    pub boarding: String,
    /// Procedural detail (fractal) texturing of the ground and large walls when close.
    pub detail_textures: bool,
    /// Passengers pay the exact fare (no change to give at the cash desk).
    pub exact_fare: bool,
    /// Enhanced graphics: the physically based renderer (its own lighting, sky, exposure).
    pub enhanced: bool,
    /// The graphics: `vanilla` (as OMSI 2 draws it: no sun shadows, no ambient occlusion,
    /// no detail grain, no snow cover or rain drops of our own), `vanilla_plus` (the same
    /// renderer with those extras, the default) or `enhanced` (`enhanced` follows it).
    pub graphics: String,
    pub fullscreen: bool,
    pub vsync: bool,
    /// Master volume 0..1.
    pub volume: f32,
    /// Control preset: "simple", "wasd", "arrows" or "omsi".
    pub drive_keys: String,
    /// Anti-aliasing of the enhanced picture after tone mapping: `fxaa` (default) or `off`.
    pub post_aa: String,
    /// OMSI's maintenance condition (`[wear_lifespan]`): 0 infinite (no wear), 1 very bad,
    /// 2 bad, 3 normal, 4 good - the player's bus's `wearlifespan` 1.5e6, 0.01, 0.1, 1, 10
    ///; AI vehicles never wear.
    pub maintenance: u8,
    /// `[AIUnschedFactor]`: the share of the random traffic (percent of the map's density).
    pub ai_unsched_factor: f32,
    /// `[AIMaxCountScheduled]`: timetable vehicles on the road at once (0 = no limit).
    pub ai_max_scheduled: u32,
    /// `[AIMaxCountParked]`: parked cars placed in the loaded tiles (0 = every space).
    pub ai_max_parked: u32,
    /// `[no_collision_vehToVeh]` off: the player's bus collides with the traffic.
    pub collision_vehicles: bool,
    /// `[no_collision_pedastrians]` off: people are knocked down.
    pub collision_pedestrians: bool,
    /// `[driverview_moving]`: the driver's head moves with the bus (braking, bends, bumps).
    pub head_movement: bool,
    /// The 3D picture drawn at this fraction of the window's size and scaled up (0.5..1),
    /// 0 = automatic (full size unless the window has more pixels than a 2560x1080 screen,
    /// as a Retina window does). The HUD is always drawn at full size.
    pub render_scale: f32,
    /// Language of the texts the game shows about the cockpit: `ENG` (default), `DEU` or
    /// `FRA` - OMSI's own language file codes.
    pub language: String,
    /// What passengers say: `all`, `tickets` (only what they ask for) or `off`.
    pub pax_voices: String,
    /// OMSI 2's route arrows over the road (as well as or instead of the navigator).
    pub nav_arrows: bool,
    /// The driver may get up from the seat and walk about (Ctrl+Shift+G).
    pub get_up: bool,
    /// Uncompressed texture files are compressed on loading where that leaves the picture
    /// close (DXT files always stay compressed on a GPU that takes them).
    pub texture_compression: bool,
    /// Texture memory the scenery may take (MB) before far textures lose their finest mip
    /// levels, like OMSI's `[texmemlimit]`; 0 = automatic (a share of the machine's memory).
    pub texture_memory: u32,
    /// OMSI's automatic clutch (`AutoClutch`, on unless `[no_automaticClutch]`): the
    /// manual-gearbox scripts work the clutch themselves while it is on.
    pub auto_clutch: bool,
    /// The original's `performance_minObjSize`: objects smaller on the screen than this are
    /// not drawn (its presets say 0.013; 0.020 for slow machines, smaller keeps more).
    pub min_obj_size: f32,
    /// The original's `performance_maxObjDist` (m): objects farther away are not drawn
    /// (0 = no limit). `auto` (-1) takes `view_distance` when the file sets one, else 900 m
    /// (the original's high presets).
    pub max_obj_dist: f32,
    /// Frames a second at most (the original's `[maxFPS]`); 0 = no limit. The frame waits
    /// asleep, so a limit also saves power and heat, and the CPU time for the rest.
    pub max_fps: u32,
    /// The chat of a LAN session (V shows and hides it, / types); off leaves it out altogether.
    pub chat: bool,
    /// The name of what the cursor points at, shown next to the cursor.
    pub tooltips: bool,
    /// The other players' names above their buses.
    pub name_tags: bool,
    /// The driver sits in the player's bus, turning the wheel, seen from outside and the
    /// passengers' seats and in the mirrors (`driver`), never in the driver's own view.
    pub driver: bool,
    /// The frame rate in the HUD.
    pub show_fps: bool,
    /// Clouds in the sky (volumetric with enhanced graphics, OMSI's cloud layer without).
    pub clouds: bool,
    /// How many people wait and ride, against the map's own numbers (OMSI's `AIPassFactor`,
    /// 1 = 100 %).
    pub pax_density: f32,
    /// Volume of the AI vehicles and of the scenery's sounds (OMSI's `sound_ai`,
    /// `sound_scenery`), 0..1.
    pub vol_ai: f32,
    pub vol_scenery: f32,
    /// Edge of the mirrors' pictures in pixels (OMSI's `performance_reflTexSize`, 2^n).
    pub mirror_size: u32,
    /// OMSI's `sound_doppler`: approaching sounds higher, receding ones lower.
    pub doppler: bool,
    /// How fast the clock runs (1 real time .. 30); in LAN play the host's decides.
    pub time_speed: f64,
    /// Texts the interface has no translation of are translated on this machine by a
    /// neural translation model (downloaded once, ~620 MB), for the languages OMSI has no
    /// language files of.
    pub machine_translation: bool,
    /// Which meshes cast sun shadows: "all" solid ones, or "omsi" - only those the models
    /// mark `[shadow]`, as OMSI 2's shadows do.
    pub shadow_casters: String,
    /// Dead zone round the centre of a set-up game controller's axes (0..0.3).
    pub ctrl_deadzone: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { msaa: 4, anisotropy: 8, ssao: true, shadows: true, shadow_size: 2048, navigator: true, navigator_opacity: 0.85, navigator_corner: "bottom-left".into(), boarding: "auto".into(), detail_textures: true, exact_fare: true, enhanced: false, graphics: "vanilla_plus".into(), fullscreen: false, vsync: true, volume: 0.6, drive_keys: "simple".into(), post_aa: "fxaa".into(), render_scale: 0.0, language: "ENG".into(), pax_voices: "all".into(), nav_arrows: false, get_up: false, texture_compression: true, texture_memory: 0, auto_clutch: true, min_obj_size: 0.013, max_obj_dist: -1.0, max_fps: 0, chat: true, tooltips: true, name_tags: true, show_fps: false, clouds: true, pax_density: 1.0, vol_ai: 1.0, vol_scenery: 1.0, mirror_size: 256, doppler: true, driver: true, maintenance: 0, ai_unsched_factor: 1.0, ai_max_scheduled: 0, ai_max_parked: 0, collision_vehicles: true, collision_pedestrians: true, head_movement: true, time_speed: 1.0, machine_translation: false, shadow_casters: "all".into(), ctrl_deadzone: 0.0 }
    }
}

impl Settings {
    /// `~/.openomsi/settings.cfg` (or `%USERPROFILE%` on Windows).
    pub fn path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(PathBuf::from(home).join(".openomsi").join("settings.cfg"))
    }

    pub fn load() -> Settings {
        let Some(p) = Self::path() else { return Settings::default() };
        let mut text = std::fs::read_to_string(&p).unwrap_or_default();
        // OMSI_GRAPHICS=vanilla|vanilla_plus|enhanced: another renderer for one run
        if let Ok(g) = omsi_cfg::env::var("OMSI_GRAPHICS") {
            text.push_str(&format!("\ngraphics={g}\n"));
        }
        let s = Self::from_text(&text);
        log::info!("settings from {}: msaa {} af {} ssao {} shadows {} ({}) navigator {} graphics {} post aa {} vsync {} render scale {} boarding {} min object size {} max object distance {} max fps {}", p.display(), s.msaa, s.anisotropy, s.ssao, s.shadows, s.shadow_size, s.navigator, s.graphics, s.post_aa, s.vsync, s.render_scale_text(), s.boarding, s.min_obj_size, s.object_distance(), s.max_fps);
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
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            let b = |v: &str| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
            match k.as_str() {
                "version" => version = v.parse().unwrap_or(0),
                "msaa" => s.msaa = v.parse().unwrap_or(s.msaa),
                "anisotropy" | "af" => s.anisotropy = v.parse().unwrap_or(s.anisotropy),
                "ssao" | "ambient_occlusion" => s.ssao = b(v),
                "shadows" => s.shadows = b(v),
                "shadow_size" => s.shadow_size = v.parse().unwrap_or(s.shadow_size),
                "navigator" => s.navigator = b(v),
                "navigator_opacity" => s.navigator_opacity = v.parse().unwrap_or(s.navigator_opacity),
                "navigator_corner" => s.navigator_corner = v.to_ascii_lowercase(),
                "boarding" => s.boarding = v.to_ascii_lowercase(),
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
                "pax_voices" => s.pax_voices = match v.to_ascii_lowercase().as_str() { "tickets" => "tickets".into(), "off" | "0" | "none" => "off".into(), _ => "all".into() },
                "nav_arrows" => s.nav_arrows = b(v),
                "get_up" => s.get_up = b(v),
                "texture_compression" => s.texture_compression = b(v),
                "auto_clutch" | "automatic_clutch" => s.auto_clutch = b(v),
                "min_obj_size" | "performance_minobjsize" => s.min_obj_size = v.parse::<f32>().map(|x| x.clamp(0.0, 0.2)).unwrap_or(s.min_obj_size),
                "max_obj_dist" | "performance_maxobjdist" => s.max_obj_dist = if v.eq_ignore_ascii_case("off") { 0.0 } else if v.eq_ignore_ascii_case("auto") { -1.0 } else { v.parse::<f32>().map(|x| x.max(0.0)).unwrap_or(s.max_obj_dist) },
                "max_fps" | "maxfps" => s.max_fps = v.parse::<f32>().map(|x| x.max(0.0) as u32).unwrap_or(s.max_fps),
                "chat" => s.chat = b(v),
                "tooltips" | "mouseover" => s.tooltips = b(v),
                "name_tags" | "nametags" => s.name_tags = b(v),
                "driver" => s.driver = b(v),
                "show_fps" | "fps" => s.show_fps = b(v),
                "clouds" => s.clouds = b(v),
                "pax_density" | "aipassfactor" => s.pax_density = v.trim_end_matches('%').parse::<f32>().map(|x| if x > 5.0 { x / 100.0 } else { x }).map(|x| x.clamp(0.0, 3.0)).unwrap_or(s.pax_density),
                "vol_ai" => s.vol_ai = v.parse::<f32>().map(|x| x.clamp(0.0, 1.0)).unwrap_or(s.vol_ai),
                "vol_scenery" => s.vol_scenery = v.parse::<f32>().map(|x| x.clamp(0.0, 1.0)).unwrap_or(s.vol_scenery),
                "doppler" | "sound_doppler" => s.doppler = b(v),
                "mirror_size" => s.mirror_size = v.parse::<u32>().map(|x| x.clamp(64, 2048).next_power_of_two()).unwrap_or(s.mirror_size),
                "texture_memory" | "texmemlimit" => s.texture_memory = v.parse::<f32>().map(|x| x.max(0.0) as u32).unwrap_or(s.texture_memory),
                "maintenance" | "wear_lifespan" => s.maintenance = v.parse::<u8>().map(|x| x.min(4)).unwrap_or(s.maintenance),
                "ai_unsched_factor" | "aiunschedfactor" => s.ai_unsched_factor = v.trim_end_matches('%').parse::<f32>().map(|x| (x / 100.0).clamp(0.0, 3.0)).unwrap_or(s.ai_unsched_factor),
                "ai_max_scheduled" | "aimaxcountscheduled" => s.ai_max_scheduled = v.parse().unwrap_or(s.ai_max_scheduled),
                "ai_max_parked" | "aimaxcountparked" => s.ai_max_parked = v.parse().unwrap_or(s.ai_max_parked),
                "collision_vehicles" => s.collision_vehicles = b(v),
                "collision_pedestrians" => s.collision_pedestrians = b(v),
                "head_movement" | "driverview_moving" => s.head_movement = b(v),
                "time_speed" => s.time_speed = v.trim_start_matches(['x', 'X']).parse::<f64>().ok().filter(|x| x.is_finite()).map(|x| x.clamp(1.0, 30.0)).unwrap_or(s.time_speed),
                "machine_translation" => s.machine_translation = b(v),
                "ctrl_deadzone" => s.ctrl_deadzone = v.parse::<f32>().ok().filter(|x| x.is_finite()).map(|x| x.clamp(0.0, 0.3)).unwrap_or(s.ctrl_deadzone),
                "shadow_casters" => s.shadow_casters = if v.eq_ignore_ascii_case("omsi") { "omsi".into() } else { "all".into() },
                "post_aa" => s.post_aa = if matches!(v.to_ascii_lowercase().as_str(), "off" | "0" | "none" | "false") { "off".into() } else { "fxaa".into() },
                "drive_keys" => s.drive_keys = match v.to_ascii_lowercase().as_str() { "wasd" | "arrows" | "omsi" | "simple" => v.to_ascii_lowercase(), _ => s.drive_keys },
                _ => {}
            }
        }
        // `graphics` decides; a file without it (older builds) says only `enhanced`, and
        // its vanilla renderer is what is now called Vanilla+
        s.graphics = graphics.unwrap_or_else(|| if s.enhanced { "enhanced" } else { "vanilla_plus" }.to_string());
        s.enhanced = s.graphics == "enhanced";
        if s.classic() {
            s.shadows = false;
            s.ssao = false;
            s.detail_textures = false;
        }
        // Files older than version 2 say `boarding=pay` because that was the launcher's
        // default, not because anybody chose it: passengers then stood at the cash desk
        // waiting for a driver who did not know he had to sell them a ticket.
        if version < SETTINGS_VERSION && s.boarding == "pay" {
            log::info!("settings: boarding=pay from an old settings file taken as auto (choose pay again in the launcher to keep it)");
            s.boarding = "auto".into();
        }
        s
    }

    /// The settings as the file holds them. The game only ever reads the file - the
    /// launcher's settings page writes it - so this is here for the round-trip test that
    /// every key read is written back.
    #[cfg(test)]
    pub fn to_text(&self) -> String {
        format!(
            "# openOMSI settings\nversion={}\nmsaa={}\nanisotropy={}\nssao={}\nshadows={}\nshadow_size={}\nnavigator={}\nnavigator_opacity={}\nnavigator_corner={}\nboarding={}\ndetail_textures={}\nexact_fare={}\nenhanced={}\ngraphics={}\nfullscreen={}\nvsync={}\nvolume={}\ndrive_keys={}\npost_aa={}\nrender_scale={}\nlanguage={}\ntexture_compression={}\ntexture_memory={}\nauto_clutch={}\nmin_obj_size={}\nmax_obj_dist={}\nmax_fps={}\nchat={}\ntooltips={}\nname_tags={}\nshow_fps={}\nclouds={}\npax_density={}\nvol_ai={}\nvol_scenery={}\nmirror_size={}\ndoppler={}\ndriver={}\n",
            SETTINGS_VERSION, self.msaa, self.anisotropy, self.ssao as u8, self.shadows as u8, self.shadow_size, self.navigator as u8, self.navigator_opacity, self.navigator_corner, self.boarding, self.detail_textures as u8, self.exact_fare as u8, self.enhanced as u8, self.graphics, self.fullscreen as u8, self.vsync as u8, self.volume, self.drive_keys, self.post_aa, self.render_scale_text(), self.language, self.texture_compression as u8, self.texture_memory, self.auto_clutch as u8, self.min_obj_size, if self.max_obj_dist < 0.0 { "auto".to_string() } else { self.max_obj_dist.to_string() }, self.max_fps, self.chat as u8, self.tooltips as u8, self.name_tags as u8, self.show_fps as u8, self.clouds as u8, self.pax_density, self.vol_ai, self.vol_scenery, self.mirror_size, self.doppler as u8, self.driver as u8
        )
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
        if self.render_scale > 0.0 { format!("{}", self.render_scale) } else { "auto".into() }
    }

    pub fn render_options(&self) -> omsi_render::RenderOptions {
        omsi_render::RenderOptions { msaa: self.msaa, anisotropy: self.anisotropy, shadow_size: self.shadow_size, ssao: self.ssao, render_scale: self.render_scale, compress_textures: self.texture_compression, fxaa: self.post_aa != "off", min_obj_size: self.min_obj_size, max_obj_dist: self.object_distance(), omsi_shadow_casters: self.shadow_casters == "omsi" }
    }
}

/// `vanilla`, `vanilla_plus` or `enhanced` from the ways a file may spell them.
pub fn graphics_mode(v: &str) -> &'static str {
    match v.trim().to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
        "enhanced" | "1" => "enhanced",
        "vanilla" | "classic" | "original" | "omsi" | "omsi2" | "omsi_2" => "vanilla",
        _ => "vanilla_plus",
    }
}

/// `view_distance=<metres>` of the settings file: how far around the camera the map's tiles
/// are kept loaded (OMSI's "visible distance"). None when the file does not say.
pub fn view_distance() -> Option<f64> {
    let text = std::fs::read_to_string(Settings::path()?).ok()?;
    text.lines().filter_map(|l| l.trim().split_once('=')).find(|(k, _)| k.trim().eq_ignore_ascii_case("view_distance")).and_then(|(_, v)| v.trim().parse::<f64>().ok()).filter(|v| *v > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_files_board_automatically() {
        // the launcher's old default, written without a version
        assert_eq!(Settings::from_text("msaa=1\nboarding=pay\n").boarding, "auto");
        // chosen again in a current file, it stays
        assert_eq!(Settings::from_text("version=2\nboarding=pay\n").boarding, "pay");
        assert_eq!(Settings::from_text("boarding=walk\n").boarding, "walk");
        // what we write reads back the same
        let s = Settings { boarding: "pay".into(), ..Default::default() };
        assert_eq!(Settings::from_text(&s.to_text()), s);
        let s = Settings { texture_compression: false, texture_memory: 1500, ..Default::default() };
        assert_eq!(Settings::from_text(&s.to_text()), s);
        assert_eq!(Settings::from_text("texmemlimit=401.0\n").texture_memory, 401);
    }

    #[test]
    fn graphics_modes() {
        // an old file: its vanilla renderer is Vanilla+ now, enhanced stays enhanced
        assert_eq!(Settings::from_text("enhanced=0\n").graphics, "vanilla_plus");
        assert_eq!(Settings::from_text("enhanced=1\n").graphics, "enhanced");
        let v = Settings::from_text("graphics=vanilla\nshadows=1\nssao=1\n");
        assert!(v.classic() && !v.shadows && !v.ssao && !v.detail_textures && !v.enhanced);
        assert!(Settings::from_text("graphics=enhanced\nenhanced=0\n").enhanced);
        assert_eq!(graphics_mode("Vanilla+"), "vanilla_plus");
        assert_eq!(graphics_mode("OMSI 2"), "vanilla");
        let s = Settings { graphics: "enhanced".into(), enhanced: true, ..Default::default() };
        assert_eq!(Settings::from_text(&s.to_text()), s);
    }

    #[test]
    fn post_aa_is_read_and_written() {
        assert_eq!(Settings::from_text("enhanced=1\n").post_aa, "fxaa");
        let off = Settings::from_text("enhanced=1\npost_aa=off\n");
        assert_eq!(off.post_aa, "off");
        assert!(!off.render_options().fxaa);
        assert!(Settings::from_text("post_aa=FXAA").render_options().fxaa);
        assert_eq!(Settings::from_text(&off.to_text()), off);
    }
}

impl Settings {
    /// The player's bus's `wearlifespan` for the maintenance condition (OMSI's table).
    pub fn wear_lifespan(&self) -> f32 {
        [1.5e6, 0.01, 0.1, 1.0, 10.0][self.maintenance.min(4) as usize]
    }
}
