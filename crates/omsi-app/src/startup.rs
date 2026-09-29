//! Start-up: finding the OMSI 2 installation, the graphics instance, the build, the allocator and the console.

use super::*;

/// CPU seconds this process has used so far (all threads), from `ps`.
pub(crate) fn process_cpu_seconds() -> Option<f64> {
    let out = std::process::Command::new("ps")
        .args(["-o", "cputime=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    // [[dd-]hh:]mm:ss.ss
    let (days, rest) = match text.trim().split_once('-') {
        Some((d, r)) => (d.parse::<f64>().ok()?, r.to_string()),
        None => (0.0, text.trim().to_string()),
    };
    let secs = rest.split(':').try_fold(0.0, |acc, part| {
        part.parse::<f64>().ok().map(|v| acc * 60.0 + v)
    })?;
    Some(days * 86400.0 + secs)
}

pub(crate) fn shift_held_now(keys: &hashbrown::HashSet<KeyCode>) -> bool {
    keys.contains(&KeyCode::ShiftLeft) || keys.contains(&KeyCode::ShiftRight)
}

/// Does this folder look like an OMSI 2 installation?
pub(crate) fn is_omsi_root(p: &Path) -> bool {
    // a complete installation of the original game (openOMSI's own content folder has the
    // same layout, but it is a mod overlay, not the game)
    omsi_cfg::missing_original_essentials(p).is_empty()
}

/// Say that the game cannot start, where the player sees it: a dialog when there is no
/// terminal to print to (a double click, the launcher), and the log as always.
pub(crate) fn fatal_dialog(title: &str, text: &str) {
    log::error!("{title}: {text}");
    // started from a terminal (the message is right there) or by a test harness
    if std::io::IsTerminal::is_terminal(&std::io::stderr()) || omsi_cfg::env::var_os("OMSI_BACKGROUND").is_some() {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "display alert \"{}\" message \"{}\" as critical",
            esc(title),
            esc(text)
        );
        let _ = std::process::Command::new("osascript").arg("-e").arg(script).status();
    }
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        extern "system" {
            fn MessageBoxW(hwnd: *mut core::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
        }
        let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (t, c) = (wide(text), wide(title));
        // MB_OK | MB_ICONERROR
        unsafe {
            MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), 0x10);
        }
    }
}

/// openOMSI's own content folder: the folder of the game binary, laid out like an OMSI 2
/// installation (Vehicles, maps, Sceneryobjects ...). Mods live here; it is searched before
/// the original installation.
/// The `Inputs/keyboard.cfg` the game follows: the content folder's once the launcher has
/// saved key bindings there, else the original installation's (never written).
pub(crate) fn keyboard_cfg(root: &Path) -> PathBuf {
    if let Some(own) = content_dir().map(|c| c.join("Inputs/keyboard.cfg")).filter(|p| p.exists()) {
        return own;
    }
    root.join("Inputs/keyboard.cfg")
}

pub(crate) fn content_dir() -> Option<PathBuf> {
    if let Some(d) = omsi_cfg::env::var_os("OMSI_CONTENT") {
        return Some(PathBuf::from(d));
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    // inside an .app bundle the binary sits in Contents/MacOS: use the folder beside the bundle
    let dir = if dir.ends_with("Contents/MacOS") {
        dir.parent()?.parent()?.parent()?.to_path_buf()
    } else {
        dir
    };
    Some(dir)
}

/// Where the last working installation was remembered.
pub(crate) fn root_memo() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".openomsi-root"))
}

/// Find the OMSI 2 installation without being told where it is.
pub(crate) fn find_root() -> Option<PathBuf> {
    let mut first: Vec<PathBuf> = Vec::new();
    if let Some(p) = omsi_cfg::env::var_os("OMSI_ROOT").map(PathBuf::from) {
        first.push(p);
    }
    if let Some(memo) = root_memo() {
        if let Ok(text) = std::fs::read_to_string(&memo) {
            first.push(PathBuf::from(text.trim()));
        }
    }
    // the folder the launcher was told about (Setup)
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        if let Ok(t) = std::fs::read_to_string(PathBuf::from(home).join(".openomsi/launcher.json")) {
            if let Some(r) = serde_json::from_str::<serde_json::Value>(&t).ok().and_then(|v| v.get("root").and_then(|r| r.as_str()).map(PathBuf::from)) {
                first.push(r);
            }
        }
    }
    omsi_cfg::find_original_install(&first)
}

/// Use the native supported API for both windowed and offscreen rendering.
pub(crate) fn graphics_instance() -> wgpu::Instance {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    if crate::server::SERVER_MODE.load(std::sync::atomic::Ordering::Relaxed) {
        // the dedicated server draws nothing: wgpu's no-op device takes every call
        descriptor.backends = wgpu::Backends::NOOP;
        descriptor.backend_options.noop = wgpu::NoopBackendOptions { enable: true };
        return wgpu::Instance::new(descriptor);
    }
    descriptor.backends = if cfg!(target_os = "macos") {
        wgpu::Backends::METAL
    } else if cfg!(windows) {
        // Prefer a compatible native backend instead of requiring Vulkan alone.
        wgpu::Backends::DX12 | wgpu::Backends::VULKAN
    } else {
        wgpu::Backends::VULKAN
    };
    wgpu::Instance::new(descriptor)
}

/// The commit this binary was built from (see `build.rs`), so a log or a screenshot says
/// which version is running.
pub const BUILD: &str = env!("OMSI_BUILD");

/// The release version, `MAJOR.MINOR.COMMIT` (see `build.rs` and docs/VERSIONING.md).
pub const VERSION: &str = env!("OPENOMSI_VERSION");

/// The application icon for the window (Windows and Linux; macOS takes the bundle's).
pub(crate) fn window_icon() -> Option<winit::window::Icon> {
    static PNG: &[u8] = include_bytes!("../../../assets/icons/app/openomsi-256.png");
    let img = image::load_from_memory(PNG).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    winit::window::Icon::from_rgba(img.into_raw(), w, h).ok()
}

/// Enhanced graphics wanted (from the settings, `--enhanced`, or OMSI_ENHANCED=1).
pub(crate) static ENHANCED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Vanilla graphics: the picture as OMSI 2 draws it (no Vanilla+ extras, see
/// `omsi_render::Lighting::classic`).
pub(crate) static CLASSIC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Volume of the AI vehicles and of the scenery's own sounds (OMSI's `sound_ai` and
/// `sound_scenery`), as the bits of an f32.
pub(crate) static SOUND_AI: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x3f80_0000);
pub(crate) static SOUND_SCENERY: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x3f80_0000);

/// Edge of a mirror's picture (the `mirror_size` setting, OMSI's reflTexSize).
pub(crate) static MIRROR_SIZE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(256);

pub(crate) fn sound_gain(which: &std::sync::atomic::AtomicU32) -> f32 {
    f32::from_bits(which.load(std::sync::atomic::Ordering::Relaxed))
}

/// The `clouds` setting: clouds in the sky (off: a clear sky whatever the weather says).
pub(crate) static CLOUDS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// macOS's allocator keeps freed large blocks for reuse, and they count as the game's
/// memory: after loading and unloading tiles it held more than half a gigabyte of them, and
/// `malloc_zone_pressure_relief` does not give them back. `MallocLargeCache=0` (read when
/// the process starts) returns them at once, so the game starts itself again with it, in
/// the same process (exec keeps the pid the launcher knows). `OMSI_KEEP_ALLOCATOR=1` skips it.
#[cfg(target_os = "macos")]
pub(crate) fn restart_with_allocator_settings() {
    use std::os::unix::process::CommandExt;
    if std::env::var_os("MallocLargeCache").is_some()
        || omsi_cfg::env::var_os("OMSI_KEEP_ALLOCATOR").is_some()
    {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let err = std::process::Command::new(exe)
        .args(std::env::args_os().skip(1))
        .env("MallocLargeCache", "0")
        .exec();
    // only comes back when the exec failed: go on as we are
    eprintln!("could not restart with MallocLargeCache=0: {err}");
}

/// A Windows GUI program has no console; when it was started from one (cmd, PowerShell)
/// the log and --help still belong there.
#[cfg(windows)]
pub(crate) fn attach_parent_console() {
    extern "system" {
        fn AttachConsole(process: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // fails harmlessly when there is no parent console (a double click, the launcher, whose
    // redirected log file stays the output)
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
