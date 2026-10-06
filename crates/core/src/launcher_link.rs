//! Another launcher program (`OMSI_LAUNCHER=<program>`): the game's own window is the
//! launcher now (`launcher`), this only starts a different one when asked to.

use super::*;

/// The launcher binary: next to this program, or the launcher's own build folder in the
/// source tree, or in ~/.neoomsi.
pub(crate) fn find_launcher() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(p) = ::legacy_config::env::var_os("OMSI_LAUNCHER") {
        cands.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            cands.push(dir.join("neoomsi-launcher"));
            cands.push(dir.join("neoomsi-launcher.exe"));
            cands.push(dir.join("neoOMSI.app/Contents/MacOS/neoomsi-launcher"));
        }
    }
    if let Some(memo) = root_memo() {
        if let Some(home) = memo.parent() {
            cands.push(home.join(".neoomsi/neoomsi-launcher"));
        }
    }
    cands.into_iter().find(|p| p.is_file())
}

/// Start the launcher and return once it is running. `Ok(false)` when there is none.
pub(crate) fn open_launcher() -> Result<bool> {
    let Some(bin) = find_launcher() else {
        return Ok(false);
    };
    log::info!("starting the launcher {}", bin.display());
    let exe = std::env::current_exe().unwrap_or_default();
    std::process::Command::new(&bin)
        .env("neoomsi_BIN", &exe)
        .spawn()
        .map_err(|e| anyhow!("starting {}: {e}", bin.display()))?;
    Ok(true)
}
