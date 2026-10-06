//! Bounded stereo SPSC ring. Decoder writers serialize only with each other; the mixer
//! reads atomic slots without a decoder lock. Release/acquire indices publish whole frames.
//! One mixer reader per buffer; full rings discard newest frames and count the loss.

use crate::assets::Clip;
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// 524288 frames (~11.9 seconds at 44.1 kHz), allocated once on the game thread.
pub const STREAM_CAPACITY: usize = 1 << 19;
struct Slot { samples: AtomicU64, rate: AtomicU32 }
pub struct StreamBuf {
    slots: Box<[Slot]>,
    head: AtomicUsize,
    tail: AtomicUsize,
    writer: Mutex<()>,
    reader: AtomicBool,
    playing: AtomicBool,
    closed: AtomicBool,
    rate: AtomicU32,
    dropped: AtomicU64,
    status: Mutex<String>,
    pub(crate) empty_clip: Arc<Clip>,
}
/// Compatibility name from the former decoder-lock implementation. The ring no longer
/// exposes a mutable shared inner buffer; query `StreamBuf` for current state instead.
#[doc(hidden)]
pub struct StreamInner {
    pub rate: u32,
    pub closed: bool,
}

impl Default for StreamBuf {
    fn default() -> Self {
        Self {
            slots: (0..STREAM_CAPACITY).map(|_| Slot {
                samples: AtomicU64::new(0), rate: AtomicU32::new(44100),
            }).collect(),
            head: AtomicUsize::new(0), tail: AtomicUsize::new(0), writer: Mutex::new(()),
            reader: AtomicBool::new(false), playing: AtomicBool::new(false),
            closed: AtomicBool::new(false), rate: AtomicU32::new(44100),
            dropped: AtomicU64::new(0), status: Mutex::new(String::new()),
            empty_clip: Arc::new(Clip { sample_rate: 44100, channels: 2, samples: Vec::new() }),
        }
    }
}
impl StreamBuf {
    fn len(&self) -> usize {
        // Read tail first: a concurrently advancing producer must not underflow this count.
        let tail = self.tail.load(Ordering::Acquire);
        self.head.load(Ordering::Acquire).wrapping_sub(tail).min(STREAM_CAPACITY)
    }
    pub fn push(&self, rate: u32, frames: impl IntoIterator<Item = [f32; 2]>) {
        if rate == 0 || self.is_closed() { return; }
        let _writer = self.writer.lock();
        self.rate.store(rate, Ordering::Relaxed);
        for frame in frames {
            if self.is_closed() { break; }
            let head = self.head.load(Ordering::Relaxed);
            let tail = self.tail.load(Ordering::Acquire);
            if head.wrapping_sub(tail) >= STREAM_CAPACITY {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            let finite = |x: f32| if x.is_finite() { x } else { 0.0 };
            let bits = finite(frame[0]).to_bits() as u64 | ((finite(frame[1]).to_bits() as u64) << 32);
            let slot = &self.slots[head % STREAM_CAPACITY];
            slot.samples.store(bits, Ordering::Relaxed);
            slot.rate.store(rate, Ordering::Relaxed);
            self.head.store(head.wrapping_add(1), Ordering::Release);
        }
    }
    pub fn free_frames(&self) -> usize {
        STREAM_CAPACITY - self.len()
    }
    pub fn storage_bytes(&self) -> usize {
        self.slots.len() * std::mem::size_of::<Slot>()
    }
    pub fn buffered(&self) -> f32 {
        self.len() as f32 / self.rate.load(Ordering::Relaxed).max(1) as f32
    }
    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release); self.playing.store(false, Ordering::Relaxed);
    }
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
    pub fn dropped_frames(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
    pub fn set_status(&self, s: impl Into<String>) {
        *self.status.lock() = s.into();
    }
    pub fn status(&self) -> String {
        self.status.lock().clone()
    }
    pub(crate) fn reader(self: &Arc<Self>) -> Option<StreamReader> {
        self.reader.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).ok()?;
        Some(StreamReader { buf: self.clone(), pair: None, fraction: 0.0,
            prebuffer: 1.5, rate: 44100, started: false })
    }
}

/// The interpolation/history belongs to the audio thread, never to the decoder.
pub(crate) struct StreamReader {
    buf: Arc<StreamBuf>, pair: Option<([f32; 2], [f32; 2])>, fraction: f64,
    prebuffer: f32, rate: u32, started: bool,
}
impl StreamReader {
    fn pop(&mut self) -> Option<([f32; 2], u32)> {
        let tail = self.buf.tail.load(Ordering::Relaxed);
        if tail == self.buf.head.load(Ordering::Acquire) { return None; }
        let slot = &self.buf.slots[tail % STREAM_CAPACITY];
        let bits = slot.samples.load(Ordering::Relaxed);
        let rate = slot.rate.load(Ordering::Relaxed);
        self.buf.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(([f32::from_bits(bits as u32), f32::from_bits((bits >> 32) as u32)], rate))
    }
    fn stalled(&mut self) {
        if self.started { self.prebuffer = (self.prebuffer + 1.0).min(6.0); }
        self.started = false; self.pair = None; self.fraction = 0.0;
        self.buf.playing.store(false, Ordering::Relaxed);
    }
    pub(crate) fn next(&mut self, output_rate: u32) -> Option<(f32, f32)> {
        if self.buf.is_closed() { return None; }
        if self.pair.is_none() {
            let required = (self.buf.rate.load(Ordering::Relaxed) as f32 * self.prebuffer) as usize;
            if self.buf.len() < required.min(STREAM_CAPACITY * 3 / 4).max(2) { return None; }
            let (a, rate) = self.pop()?;
            let (b, next_rate) = self.pop()?;
            self.rate = next_rate;
            self.pair = Some((if rate == next_rate { a } else { b }, b));
            self.started = true;
            self.buf.playing.store(true, Ordering::Relaxed);
        }
        let (a, b) = self.pair?;
        let t = self.fraction as f32;
        let out = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
        // Radio has a fixed decoded rate and no engine-pitch modulation. At most 64
        // advances per output frame, matching the clip rate bound.
        self.fraction += (self.rate as f64 / output_rate.max(1) as f64).min(64.0);
        while self.fraction >= 1.0 {
            self.fraction -= 1.0;
            if let Some((next, rate)) = self.pop() {
                let previous = self.pair.unwrap().1;
                self.pair = Some((if rate == self.rate { previous } else { next }, next));
                self.rate = rate;
            } else { self.stalled(); break; }
        }
        Some(out)
    }
}
impl Drop for StreamReader {
    fn drop(&mut self) {
        self.buf.playing.store(false, Ordering::Relaxed); self.buf.reader.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ring_is_bounded_and_wraps_in_fifo_order() {
        let b = Arc::new(StreamBuf::default());
        b.push(48000, (0..STREAM_CAPACITY + 7).map(|i| [i as f32, -(i as f32)]));
        assert_eq!(b.len(), STREAM_CAPACITY); assert_eq!(b.dropped_frames(), 7);
        let mut r = b.reader().unwrap(); assert!(b.reader().is_none());
        for i in 0..STREAM_CAPACITY { assert_eq!(r.pop().unwrap().0, [i as f32, -(i as f32)]); }
        b.push(48000, [[0.25, -0.25]]);
        assert_eq!(r.pop().unwrap().0, [0.25, -0.25]); assert!(r.pop().is_none());
    }
    #[test]
    fn underrun_rebuffers_and_close_is_immediate() {
        let b = Arc::new(StreamBuf::default()); let mut r = b.reader().unwrap();
        assert!(r.next(48000).is_none());
        b.push(48000, std::iter::repeat_n([0.5; 2], 72000));
        assert_eq!(r.next(48000), Some((0.5, 0.5)));
        for _ in 0..72000 { r.next(48000); }
        assert!(!b.is_playing());
        b.push(48000, std::iter::repeat_n([0.5; 2], 72000));
        assert!(r.next(48000).is_none(), "stall raised prebuffer to 2.5 seconds");
        b.close(); assert!(r.next(48000).is_none());
        b.push(48000, [[1.0; 2]]); assert!(b.is_closed());
    }
    #[test]
    fn decoder_and_mixer_preserve_frames_under_concurrent_wraparound() {
        let b = Arc::new(StreamBuf::default());
        let mut reader = b.reader().unwrap();
        let writer = b.clone();
        let producer = std::thread::spawn(move || {
            for i in 0..STREAM_CAPACITY + 1000 {
                while writer.free_frames() == 0 { std::thread::yield_now(); }
                writer.push(48000, [[i as f32, -(i as f32)]]);
            }
        });
        for i in 0..STREAM_CAPACITY + 1000 {
            let pair = loop { if let Some(pair) = reader.pop() { break pair; } std::thread::yield_now(); };
            assert_eq!(pair, ([i as f32, -(i as f32)], 48000));
        }
        producer.join().unwrap(); assert_eq!(b.dropped_frames(), 0);
    }
    #[test]
    fn rate_changes_do_not_interpolate_different_formats() {
        let b = Arc::new(StreamBuf::default()); let mut reader = b.reader().unwrap();
        b.push(48000, std::iter::repeat_n([0.5; 2], 72000));
        b.push(44100, std::iter::repeat_n([-0.5; 2], 72000));
        for _ in 0..73000 {
            let frame = reader.next(48000).unwrap();
            assert!(frame == (0.5, 0.5) || frame == (-0.5, -0.5), "{frame:?}");
        }
    }

}
