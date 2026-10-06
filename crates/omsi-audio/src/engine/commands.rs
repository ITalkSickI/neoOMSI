//! The queue of pending voice parameter changes. The game sets every voice's parameters every
//! frame; through the voice list's lock each of those calls would wait for a whole block to
//! be mixed, so they are queued here and taken in by the mixer at the start of its next block.

use crate::voice::{VoiceId, VoiceParams};
use parking_lot::Mutex;
use std::time::Instant;

pub struct VoiceUpdates {
    queue: Mutex<Vec<(VoiceId, VoiceParams, Instant)>>,
}

impl VoiceUpdates {
    pub fn new() -> VoiceUpdates {
        VoiceUpdates {
            queue: Mutex::new(Vec::new()),
        }
    }

    /// Queue new parameters for a voice, with the time they were given (for the Doppler
    /// shift). Given twice before the next block, the later ones win. A game paused while the
    /// device stopped calling back must not pile them up.
    pub fn push(&self, id: VoiceId, params: VoiceParams, at: Instant) {
        let mut q = self.queue.lock();
        if q.len() > 4096 {
            q.clear();
        }
        q.push((id, params, at));
    }

    /// Take everything queued for the block about to be mixed.
    pub fn take(&self) -> Vec<(VoiceId, VoiceParams, Instant)> {
        std::mem::take(&mut *self.queue.lock())
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }
}
