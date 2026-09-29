//! Discord's "Playing openOMSI" status (Rich Presence): the map, the bus and the line, sent
//! to the Discord app on this machine over its local IPC channel (a Unix socket, a named pipe
//! on Windows). Nothing leaves the computer but through Discord itself; without Discord
//! running, or without an application id (`discord_app_id` in the settings), nothing happens.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// What the status shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Presence {
    pub details: String,
    pub state: String,
}

pub struct Discord {
    wanted: Arc<Mutex<Option<Presence>>>,
}

#[cfg(unix)]
type Pipe = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Pipe = std::fs::File;

fn connect() -> Option<Pipe> {
    for i in 0..10 {
        #[cfg(unix)]
        {
            let dirs = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"].iter().filter_map(|k| std::env::var(k).ok()).chain(["/tmp".to_string()]);
            for d in dirs {
                let p = std::path::Path::new(&d).join(format!("discord-ipc-{i}"));
                if let Ok(s) = std::os::unix::net::UnixStream::connect(&p) {
                    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
                    return Some(s);
                }
            }
        }
        #[cfg(windows)]
        {
            if let Ok(f) = std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{i}")) {
                return Some(f);
            }
        }
    }
    None
}

fn send(pipe: &mut Pipe, op: u32, body: &str) -> std::io::Result<()> {
    let mut frame = Vec::with_capacity(8 + body.len());
    frame.extend_from_slice(&op.to_le_bytes());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(body.as_bytes());
    pipe.write_all(&frame)?;
    // the answer (read and dropped: an error closes the pipe on Discord's side anyway)
    let mut head = [0u8; 8];
    pipe.read_exact(&mut head)?;
    let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    let mut rest = vec![0u8; len.min(1 << 16)];
    pipe.read_exact(&mut rest)?;
    Ok(())
}

impl Discord {
    /// Start the status for the application `app_id` (empty: none).
    pub fn start(app_id: &str) -> Option<Discord> {
        let app_id = app_id.trim().to_string();
        if app_id.is_empty() || !app_id.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let wanted: Arc<Mutex<Option<Presence>>> = Arc::default();
        let w = wanted.clone();
        let started = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        std::thread::Builder::new()
            .name("discord".into())
            .spawn(move || {
                let mut pipe: Option<Pipe> = None;
                let mut shown: Option<Presence> = None;
                let mut last_try = Instant::now() - Duration::from_secs(60);
                let mut nonce = 0u64;
                loop {
                    if Arc::strong_count(&w) == 1 {
                        return;
                    }
                    std::thread::sleep(Duration::from_secs(2));
                    let want = w.lock().ok().and_then(|g| g.clone());
                    if pipe.is_none() {
                        if last_try.elapsed() < Duration::from_secs(20) {
                            continue;
                        }
                        last_try = Instant::now();
                        if let Some(mut p) = connect() {
                            let hello = serde_json::json!({ "v": 1, "client_id": app_id }).to_string();
                            if send(&mut p, 0, &hello).is_ok() {
                                log::info!("Discord: connected (status shown while the game runs)");
                                pipe = Some(p);
                                shown = None;
                            }
                        }
                        continue;
                    }
                    if want == shown {
                        continue;
                    }
                    nonce += 1;
                    let activity = want.as_ref().map(|p| {
                        serde_json::json!({
                            "details": p.details.chars().take(120).collect::<String>(),
                            "state": p.state.chars().take(120).collect::<String>(),
                            "timestamps": { "start": started },
                            "assets": { "large_image": "logo", "large_text": "openOMSI" },
                        })
                    });
                    let msg = serde_json::json!({
                        "cmd": "SET_ACTIVITY",
                        "args": { "pid": std::process::id(), "activity": activity },
                        "nonce": nonce.to_string(),
                    })
                    .to_string();
                    match send(pipe.as_mut().unwrap(), 1, &msg) {
                        Ok(()) => shown = want,
                        Err(_) => pipe = None,
                    }
                }
            })
            .ok()?;
        Some(Discord { wanted })
    }

    /// What the status should show now (sent within a couple of seconds when it changed).
    pub fn set(&self, p: Presence) {
        if let Ok(mut g) = self.wanted.lock() {
            *g = Some(p);
        }
    }
}
