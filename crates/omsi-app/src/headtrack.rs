//! Head tracking: the head's pose from opentrack's "UDP over network" output (six little-
//! endian doubles - x, y, z in cm, yaw, pitch, roll in degrees - to UDP port 4242). opentrack
//! takes TrackIR, Tobii, webcams (neuralnet tracker) and phones, on every platform.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The last pose received: x, y, z (cm; right, up, back) and yaw, pitch, roll (degrees).
#[derive(Debug, Clone, Copy, Default)]
pub struct HeadPose {
    pub pos: [f32; 3],
    pub rot: [f32; 3],
}

pub struct HeadTracker {
    last: Arc<Mutex<Option<(HeadPose, Instant)>>>,
}

impl HeadTracker {
    /// Listen on `port` (on every interface: opentrack may run on another machine).
    pub fn start(port: u16) -> Option<HeadTracker> {
        let sock = match std::net::UdpSocket::bind(("0.0.0.0", port)) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("head tracking: cannot listen on UDP port {port}: {e}");
                return None;
            }
        };
        let _ = sock.set_read_timeout(Some(Duration::from_millis(500)));
        let last: Arc<Mutex<Option<(HeadPose, Instant)>>> = Arc::default();
        let out = last.clone();
        std::thread::Builder::new()
            .name("head tracking".into())
            .spawn(move || {
                let mut buf = [0u8; 64];
                let mut announced = false;
                loop {
                    // (the game's end takes the thread with it)
                    if Arc::strong_count(&out) == 1 {
                        return;
                    }
                    let Ok(n) = sock.recv(&mut buf) else { continue };
                    if n < 48 {
                        continue;
                    }
                    let d = |i: usize| f64::from_le_bytes(buf[i * 8..i * 8 + 8].try_into().unwrap()) as f32;
                    let v = [d(0), d(1), d(2), d(3), d(4), d(5)];
                    if v.iter().any(|x| !x.is_finite()) {
                        continue;
                    }
                    if !announced {
                        announced = true;
                        log::info!("head tracking: receiving poses on UDP port {port}");
                    }
                    *out.lock().unwrap() = Some((HeadPose { pos: [v[0], v[1], v[2]], rot: [v[3], v[4], v[5]] }, Instant::now()));
                }
            })
            .ok()?;
        log::info!("head tracking: listening for opentrack on UDP port {port}");
        Some(HeadTracker { last })
    }

    /// The pose, while poses keep coming (none for half a second: the head is centred).
    pub fn pose(&self) -> Option<HeadPose> {
        let l = self.last.lock().ok()?;
        l.filter(|(_, t)| t.elapsed() < Duration::from_millis(500)).map(|(p, _)| p)
    }
}
