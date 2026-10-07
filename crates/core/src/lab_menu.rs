//! The lab pause menu (new pause menu in development)

use super::*;
use crate::ui::{PauseState, PAGE_COUNT, PAUSE_ENTRIES, VEHICLE_PAGE};

impl App {
    pub(crate) fn open_lab_menu(&mut self) {
        self.open_game_menu();
        self.lab_menu = Some(PauseState::default());
    }

    fn lab_activate(&mut self, event_loop: &ActiveEventLoop, entry: usize) {
        match entry {
            0 => self.close_game_menu(),
            1 => {
                self.quick_save();
                self.close_game_menu();
            }
            2 => self.lab_menu = Some(PauseState { page: Some(0), sel: 2 }),
            3 => {
                self.game_menu = None;
                self.lab_menu = None;
                self.finish_session();
                crate::platform::exit(event_loop);
            }
            _ => {}
        }
    }

    pub(crate) fn lab_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode) {
        let n = PAGE_COUNT;
        let st = self.lab_menu.unwrap_or_default();
        let Some(tab) = st.page else {
            let m = PAUSE_ENTRIES.len();
            let go = |p: usize| Some(PauseState { page: Some(p), sel: 2 });
            match code {
                KeyCode::Escape => self.close_game_menu(),
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.lab_menu = Some(PauseState { sel: (st.sel + m - 1) % m, ..st })
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.lab_menu = Some(PauseState { sel: (st.sel + 1) % m, ..st })
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    self.lab_activate(event_loop, st.sel)
                }
                KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::KeyQ => self.lab_menu = go(n - 1),
                KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::KeyE => {
                    self.lab_menu = go(0)
                }
                KeyCode::Digit1 | KeyCode::Numpad1 => self.lab_menu = go(0),
                KeyCode::Digit2 | KeyCode::Numpad2 => self.lab_menu = go(1),
                KeyCode::Digit3 | KeyCode::Numpad3 => self.lab_menu = go(2),
                KeyCode::Digit4 | KeyCode::Numpad4 => self.lab_menu = go(3),
                _ => {}
            }
            return;
        };
        // a page
        let to = |p: usize| Some(PauseState { page: Some(p), ..st });
        match code {
            KeyCode::Escape => self.lab_menu = Some(PauseState { page: None, ..st }),
            KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::KeyQ => {
                self.lab_menu = to((tab + n - 1) % n)
            }
            KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::KeyE => {
                self.lab_menu = to((tab + 1) % n)
            }
            KeyCode::Digit1 | KeyCode::Numpad1 => self.lab_menu = to(0),
            KeyCode::Digit2 | KeyCode::Numpad2 => self.lab_menu = to(1),
            KeyCode::Digit3 | KeyCode::Numpad3 => self.lab_menu = to(2),
            KeyCode::Digit4 | KeyCode::Numpad4 => self.lab_menu = to(3),
            _ => {}
        }
    }

    pub(crate) fn lab_click(&mut self, event_loop: &ActiveEventLoop) {
        let (x, y) = self.cursor;
        let st = self.lab_menu.unwrap_or_default();
        let hit = |list: &[[f32; 4]]| {
            list.iter()
                .position(|r| x >= r[0] && x < r[2] && y >= r[1] && y < r[3])
        };
        if st.page.is_some() {
            if let Some(k) = self
                .ui
                .as_ref()
                .and_then(|u| hit(&u.lab_tabs))
            {
                self.lab_menu = Some(PauseState { page: Some(k), ..st });
            } else if st.page == Some(VEHICLE_PAGE) {
                if let Some(k) = self.ui.as_ref().and_then(|u| hit(&u.lab_groups)) {
                    if let Some(u) = self.ui.as_mut() {
                        u.lab_group = k;
                    }
                } else if let Some(k) = self.ui.as_ref().and_then(|u| hit(&u.lab_actions)) {
                    let g = self.ui.as_ref().map_or(0, |u| u.lab_group);
                    let id = crate::game_lists::vehicle_menu(self)
                        .into_iter()
                        .nth(g)
                        .and_then(|(_, acts)| acts.into_iter().nth(k))
                        .map(|a| a.0);
                    if let Some(id) = id {
                        self.page_action(id);
                    }
                }
            }
        } else if let Some(k) = self.ui.as_ref().and_then(|u| hit(&u.pause_items)) {
            self.lab_menu = Some(PauseState { sel: k, ..st });
            self.lab_activate(event_loop, k);
        }
    }
}