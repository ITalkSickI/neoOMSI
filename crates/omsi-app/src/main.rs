//! `openomsi` - the game and its launcher (the game itself is the `openomsi_game` library).

// a game, not a console program: no console window opens beside it on Windows (a start
// from a terminal still prints there, see attach_parent_console)
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> anyhow::Result<()> {
    openomsi_game::run()
}
