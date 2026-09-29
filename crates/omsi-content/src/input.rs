//! `Inputs/keyboard.cfg`, `Inputs/gamectrler.cfg`, `Inputs/*.kyb` (unit `mc_input`).

use omsi_cfg::CfgFile;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KeyBinding {
    pub action: String,
    /// DirectInput scan code
    pub scan_code: i32,
    /// modifier bits (1 = shift, 2 = ctrl, 4 = alt … as used by the original)
    pub modifier: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KeyboardCfg {
    pub game: Vec<KeyBinding>,
    pub vehicles: Vec<KeyBinding>,
}

impl KeyboardCfg {
    pub fn load(path: &Path) -> Result<KeyboardCfg, omsi_cfg::CfgError> {
        let f = CfgFile::read(path)?;
        let mut k = KeyboardCfg::default();
        let mut section = 0;
        let mut r = f.reader();
        while let Some(kw) = r.next_keyword() {
            match kw.as_str() {
                "game" => section = 0,
                "vehicles" => section = 1,
                "entry" => {
                    let b = KeyBinding {
                        action: r.str().to_string(),
                        scan_code: r.i32(),
                        modifier: r.i32(),
                    };
                    if section == 0 {
                        k.game.push(b);
                    } else {
                        k.vehicles.push(b);
                    }
                }
                _ => {}
            }
        }
        Ok(k)
    }

    /// The keys the game adds to the file's (not written back by `save`).
    pub fn with_game_defaults(mut self) -> Self {
        // The IBIS's next stop with its announcement (`IBIS_vor`) has no key in OMSI's own
        // file: only the mouse on the IBIS reached it. Q (scan code 16, no modifier: Ctrl+Q
        // ends the game, Shift+Q is the microphone) gives it one where Q is still free.
        let q_taken = self.game.iter().chain(self.vehicles.iter()).any(|b| b.scan_code == 16 && b.modifier == 0);
        if !q_taken && !self.vehicles.iter().any(|b| b.action.eq_ignore_ascii_case("IBIS_vor")) {
            self.vehicles.push(KeyBinding { action: "IBIS_vor".into(), scan_code: 16, modifier: 0 });
        }
        self
    }

    pub fn with_vr_defaults(mut self) -> Self {
        // VR controls are included in the same editable list as the game's
        // other keys. An existing entry (including an unbound one) wins.
        for (action, scan_code, modifier) in [
            ("vr_recenter", 19, 3),
            ("vr_toggle_desktop_mirror", 65, 0),
            ("vr_toggle_mode", 66, 0),
        ] {
            if !self.game.iter().any(|b| b.action.eq_ignore_ascii_case(action)) {
                self.game.push(KeyBinding { action: action.into(), scan_code, modifier });
            }
        }
        self
    }

    /// Write a `keyboard.cfg` the game (and this same loader) can read back: one
    /// `[game]`/`[vehicles]` section, each entry as `action / scan code / modifier`, blank
    /// lines between entries as the original ships it (some tools that read the file split
    /// on the blank line rather than the keyword).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut t = String::new();
        for (section, list) in [("game", &self.game), ("vehicles", &self.vehicles)] {
            t.push_str(&format!("[{section}]\r\n"));
            for b in list {
                t.push_str(&format!(
                    "\r\n[entry]\r\n{}\r\n{}\r\n{}\r\n",
                    b.action, b.scan_code, b.modifier
                ));
            }
            t.push_str("\r\n");
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, t)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameController {
    pub name: String,
    pub index: i32,
    /// 16 values: per logical axis (steer, throttle, brake, clutch, combined …) the device
    /// axis number and inversion flag.
    pub axes: Vec<i32>,
    pub buttons: Vec<(String, i32)>,
    pub ff_scale: (f32, f32),
}

pub fn load_game_controllers(path: &Path) -> Result<Vec<GameController>, omsi_cfg::CfgError> {
    let f = CfgFile::read(path)?;
    let mut out: Vec<GameController> = Vec::new();
    let mut r = f.reader();
    while let Some(kw) = r.next_keyword() {
        match kw.as_str() {
            "ctrl" => out.push(GameController {
                name: r.str().to_string(),
                index: r.i32(),
                ..Default::default()
            }),
            "axis" => {
                let v = (0..16).map(|_| r.i32()).collect();
                if let Some(c) = out.last_mut() {
                    c.axes = v;
                }
            }
            "buttons" => {
                let n = r.usize();
                let mut b = Vec::with_capacity(n);
                for _ in 0..n {
                    let a = r.str().to_string();
                    let i = r.i32();
                    b.push((a, i));
                }
                if let Some(c) = out.last_mut() {
                    c.buttons = b;
                }
            }
            "ffscale" => {
                let a = r.f32();
                let b = r.f32();
                if let Some(c) = out.last_mut() {
                    c.ff_scale = (a, b);
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// `.kyb`: `scancode<TAB>name` lines.
pub fn load_key_names(path: &Path) -> Result<Vec<(i32, String)>, omsi_cfg::CfgError> {
    let f = CfgFile::read(path)?;
    Ok(f.lines
        .iter()
        .filter_map(|l| {
            let (a, b) = l.split_once('\t')?;
            Some((omsi_cfg::parse_i32(a), b.trim().to_string()))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vr_defaults_keep_custom_and_unbound_keys() {
        let custom = KeyBinding { action: "vr_recenter".into(), scan_code: 0, modifier: 0 };
        let cfg = KeyboardCfg { game: vec![custom.clone()], ..Default::default() }
            .with_vr_defaults().with_vr_defaults();
        assert_eq!(cfg.game.iter().filter(|b| b.action == "vr_recenter").count(), 1);
        assert!(cfg.game.contains(&custom));
        assert!(cfg.game.iter().any(|b| b.action == "vr_toggle_mode" && b.scan_code == 66));
    }

    #[test]
    fn keyboard_cfg_round_trips_through_save() {
        let k = KeyboardCfg {
            game: vec![KeyBinding {
                action: "exit".into(),
                scan_code: 1,
                modifier: 4,
            }],
            vehicles: vec![
                KeyBinding {
                    action: "kw_blinker_links".into(),
                    scan_code: 44,
                    modifier: 0,
                },
                KeyBinding {
                    action: "kw_m_enginestart".into(),
                    scan_code: 50,
                    modifier: 1,
                },
            ],
        };
        let path =
            std::env::temp_dir().join(format!("omsi-keyboard-cfg-test-{}.cfg", std::process::id()));
        k.save(&path).unwrap();
        let back = KeyboardCfg::load(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(back, k);
    }
}
