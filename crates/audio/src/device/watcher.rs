//! Every two seconds, the name of the system's default output device: when it is not the
//! one played on (`first`, then the last one seen), `reopen` is set. Ends with the engine.

use cpal::traits::{DeviceTrait, HostTrait};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn watch_default_device(first: String, reopen: std::sync::Weak<AtomicBool>) {
    let _ = std::thread::Builder::new()
        .name("audio device".into())
        .spawn(move || {
            let mut current = first;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let Some(flag) = reopen.upgrade() else { return };
                let name = cpal::default_host()
                    .default_output_device()
                    .and_then(|d| d.description().ok().map(|d| d.name().to_string()))
                    .unwrap_or_default();
                if name != current {
                    current = name;
                    flag.store(true, Ordering::Relaxed);
                }
            }
        });
}
