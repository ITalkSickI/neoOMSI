//! The OMSI plugins of the content roots' `plugins` folders (see `omsi_plugin`), driven
//! every frame with the player's bus as OMSI drives them: system
//! variables, then the bus's variables, string variables and triggers.

use omsi_plugin::{HostConfig, PluginIo, Plugins};
use omsi_script::Host;
use omsi_script::SysVar;

/// Load every plugin of every content root (`OMSI_NO_PLUGINS=1` leaves them out).
pub(crate) fn load() -> Plugins {
    if omsi_cfg::env::var_os("OMSI_NO_PLUGINS").is_some() {
        return Plugins::default();
    }
    // (never from content another machine sent: a LAN host's mods are data only)
    let dirs: Vec<std::path::PathBuf> = omsi_cfg::content_roots()
        .iter()
        .filter(|r| !omsi_cfg::is_sandbox(r))
        .filter_map(|r| omsi_plugin::resolve_path(r, "plugins"))
        .filter(|d| d.is_dir())
        .collect();
    if dirs.is_empty() {
        return Plugins::default();
    }
    Plugins::load(&dirs, &HostConfig::detect())
}

/// The game's side of a plugin frame: the player's bus, when there is one.
pub(crate) struct Io<'a> {
    pub vehicle: Option<&'a mut omsi_sim::VehicleInstance>,
}

impl PluginIo for Io<'_> {
    fn system(&mut self, name: &str) -> Option<f32> {
        let v = SysVar::from_name(name)?;
        self.vehicle.as_mut().map(|veh| veh.host.sys_var(v))
    }

    fn set_system(&mut self, name: &str, v: f32) {
        // the clock, the weather and the input are the game's own; a plugin writing them
        // is told nothing, as the scripts' S.S. writes are not honoured either
        log::debug!("plugin wrote system variable {name} = {v} (kept as it is)");
    }

    fn has_vehicle(&self) -> bool {
        self.vehicle.is_some()
    }

    fn var(&mut self, name: &str) -> Option<f32> {
        self.vehicle.as_ref()?.var(name)
    }

    fn set_var(&mut self, name: &str, v: f32) {
        if let Some(veh) = self.vehicle.as_mut() {
            veh.set_var(name, v);
        }
    }

    fn string(&mut self, name: &str) -> Option<String> {
        let veh = self.vehicle.as_ref()?;
        let i = veh.ty.program.str_var(name)?;
        veh.state.str_vars.get(i as usize).cloned()
    }

    fn set_string(&mut self, name: &str, s: &str) {
        if let Some(veh) = self.vehicle.as_mut() {
            if let Some(i) = veh.ty.program.str_var(name) {
                if let Some(slot) = veh.state.str_vars.get_mut(i as usize) {
                    *slot = s.to_string();
                }
            }
        }
    }

    /// A key down fires the trigger, a key up `<trigger>_off` (OMSI's keyboard event
    /// handler the original, which the plugin frame calls with the new state).
    fn fire(&mut self, trigger: &str, down: bool) {
        if let Some(veh) = self.vehicle.as_mut() {
            if down {
                veh.trigger(trigger);
            } else {
                veh.trigger(&format!("{trigger}_off"));
            }
        }
    }
}
