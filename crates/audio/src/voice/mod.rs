//! Voices: the parameters the runtime hands in ([`params`]) and a playing voice's own state
//! and per-block rendering ([`voice`]). Voice code decides nothing about OMSI conditions.

pub mod params;
pub mod voice;

pub use params::{Level, MixParams, DOPPLER, Listener, VoiceId, VoiceParams};
pub use voice::{doppler_enabled, Voice};

pub mod bus;
