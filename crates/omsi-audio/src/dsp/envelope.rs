//! Sample-time transitions, independent of callback size: gain 5 ms, pitch 15 ms,
//! start/stop 3 ms. Fades are finite so stopped voices can be retired predictably.
pub fn coefficient(rate: u32, seconds: f32) -> f32 {
    1.0 - (-1.0 / (rate.max(1) as f32 * seconds)).exp()
}
pub struct Envelope { value: f32, stopping: bool }
impl Default for Envelope {
    fn default() -> Self { Self { value: 0.0, stopping: false } }
}
impl Envelope {
    pub fn stop(&mut self) {
        self.stopping = true;
    }
    pub fn ended(&self) -> bool {
        self.stopping && self.value <= 0.0
    }
    pub fn next(&mut self, rate: u32) -> f32 {
        let step = 1.0 / (0.003 * rate.max(1) as f32);
        self.value = if self.stopping { (self.value - step).max(0.0) }
            else { (self.value + step).min(1.0) };
        self.value
    }
    #[cfg(test)]
    pub fn steady(&mut self) {
        self.value = 1.0;
    }
}
