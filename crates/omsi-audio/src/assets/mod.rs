//! Sound assets: decoded clips and their cache, WAV reading, and the streaming buffers
//! (internet radio). Nothing here knows the OMSI runtime or the mixer.

pub mod clip;
pub mod radio;
pub mod stream;
pub mod wav;

pub use clip::Clip;
