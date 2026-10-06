//! Validated RIFF/WAVE PCM/float reader. The compact cache remains i16 to preserve Clip
//! literals and fleet memory usage. 24/32-bit and float input is rounded (not truncated),
//! using f64 until this boundary; DSP accumulation and radio samples remain f32.
use anyhow::{Result, anyhow, bail};

pub struct WavData { pub sample_rate: u32, pub channels: u16, pub samples: Vec<i16> }
fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes(b[i..i+2].try_into().unwrap())
}
fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i+4].try_into().unwrap())
}
fn quantize(x: f64) -> i16 {
    (x * 32768.0).round().clamp(-32768.0, 32767.0) as i16
}

pub fn parse_wav(bytes: &[u8]) -> Result<WavData> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("not a RIFF/WAVE file");
    }
    let end = (u32_at(bytes, 4) as usize).checked_add(8).ok_or_else(|| anyhow!("RIFF size overflow"))?;
    if end < 12 || end > bytes.len() { bail!("truncated RIFF container"); }
    let mut p = 12usize;
    let mut fmt = None;
    let mut data = None;
    while p < end {
        if end - p < 8 { bail!("truncated chunk header"); }
        let size = u32_at(bytes, p + 4) as usize;
        let body_end = p.checked_add(8).and_then(|i| i.checked_add(size)).ok_or_else(|| anyhow!("chunk overflow"))?;
        let next = body_end.checked_add(size & 1).ok_or_else(|| anyhow!("padding overflow"))?;
        if next > end { bail!("truncated chunk or missing padding"); }
        let body = &bytes[p + 8..body_end];
        match &bytes[p..p+4] {
            b"fmt " => { if fmt.replace(body).is_some() { bail!("duplicate fmt chunk"); } }
            b"data" => { if data.replace(body).is_some() { bail!("duplicate data chunk"); } }
            _ => {}
        }
        p = next;
    }
    let fmt = fmt.ok_or_else(|| anyhow!("missing fmt chunk"))?;
    let data = data.ok_or_else(|| anyhow!("missing data chunk"))?;
    if fmt.len() < 16 { bail!("short fmt chunk"); }
    let mut tag = u16_at(fmt, 0);
    let channels = u16_at(fmt, 2);
    let sample_rate = u32_at(fmt, 4);
    let bits = u16_at(fmt, 14);
    if !(1..=2).contains(&channels) { bail!("only mono/stereo WAV sources are supported"); }
    if !(100..=384000).contains(&sample_rate) { bail!("invalid sample rate"); }
    if fmt.len() == 17 || (fmt.len() >= 18 && 18 + u16_at(fmt, 16) as usize > fmt.len()) {
        bail!("truncated fmt extension");
    }
    if tag == 0xfffe {
        if fmt.len() < 40 || u16_at(fmt, 16) < 22 { bail!("short extensible fmt"); }
        let valid = u16_at(fmt, 18);
        if valid == 0 || valid > bits { bail!("invalid valid-bit count"); }
        let mask = u32_at(fmt, 20);
        if mask != 0 && mask != if channels == 1 { 4 } else { 3 } { bail!("unsupported channel mask"); }
        // KSDATAFORMAT_SUBTYPE_PCM / IEEE_FLOAT: validate all GUID bytes, not just its tag.
        if fmt[26..40] != [0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71] {
            bail!("unsupported extensible subtype GUID");
        }
        tag = u16_at(fmt, 24);
        if tag == 3 && valid != bits { bail!("invalid float valid-bit count"); }
    }
    if !matches!((tag, bits), (1, 8 | 16 | 24 | 32) | (3, 32)) {
        bail!("unsupported WAV format tag {tag} / {bits} bit");
    }
    let sample_bytes = bits as usize / 8;
    let align = channels as usize * sample_bytes;
    if u16_at(fmt, 12) as usize != align || u32_at(fmt, 8) as u64 != sample_rate as u64 * align as u64 {
        bail!("inconsistent WAV block alignment/byte rate");
    }
    if data.len() % align != 0 { bail!("partial sample frame"); }
    let mut samples = Vec::with_capacity(data.len() / sample_bytes);
    for value in data.chunks_exact(sample_bytes) {
        let value = match (tag, bits) {
            (1, 8) => (value[0] as f64 - 128.0) / 128.0,
            (1, 16) => i16::from_le_bytes(value.try_into().unwrap()) as f64 / 32768.0,
            (1, 24) => {
                let signed = i32::from_le_bytes([0, value[0], value[1], value[2]]) >> 8;
                signed as f64 / 8388608.0
            }
            (1, 32) => i32::from_le_bytes(value.try_into().unwrap()) as f64 / 2147483648.0,
            (3, 32) => {
                let f = f32::from_le_bytes(value.try_into().unwrap());
                if !f.is_finite() { bail!("non-finite float WAV sample"); }
                f as f64
            }
            _ => unreachable!(),
        };
        samples.push(quantize(value));
    }
    Ok(WavData { sample_rate, channels, samples })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(tag: u16, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&((36 + data.len() + (data.len() & 1)) as u32).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&tag.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&44100u32.to_le_bytes());
        b.extend_from_slice(&(44100u32 * bits as u32 / 8).to_le_bytes());
        b.extend_from_slice(&(bits / 8).to_le_bytes());
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data.len() as u32).to_le_bytes());
        b.extend_from_slice(data);
        if data.len() & 1 != 0 { b.push(0); }
        b
    }

    #[test]
    fn every_format_becomes_16_bit() {
        // 8-bit is unsigned around 128
        assert_eq!(
            parse_wav(&wav(1, 8, &[0, 128, 255])).unwrap().samples,
            vec![-32768, 0, 32512]
        );
        assert_eq!(
            parse_wav(&wav(1, 16, &[0x00, 0x80, 0xff, 0x7f]))
                .unwrap()
                .samples,
            vec![-32768, 32767]
        );
        // Higher precision sources are rounded once at the compact cache boundary.
        assert_eq!(
            parse_wav(&wav(1, 24, &[0x11, 0x00, 0x40])).unwrap().samples,
            vec![0x4000]
        );
        assert_eq!(
            parse_wav(&wav(1, 32, &[0x11, 0x22, 0x00, 0xc0]))
                .unwrap()
                .samples,
            vec![-0x4000]
        );
        let f: Vec<u8> = [0.5f32, -1.0, 2.0]
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect();
        assert_eq!(
            parse_wav(&wav(3, 32, &f)).unwrap().samples,
            vec![16384, -32768, 32767]
        );
    }
    #[test]
    fn rejects_truncated_chunks_frames_headers_and_nonfinite_samples() {
        let valid = wav(1, 16, &[0, 0, 0, 0]);
        for length in 0..valid.len() { assert!(parse_wav(&valid[..length]).is_err(), "length {length}"); }
        for (offset, value) in [(22, 0u8), (24, 0), (28, 0), (32, 1)] {
            let mut bad = valid.clone(); bad[offset] = value;
            assert!(parse_wav(&bad).is_err(), "header byte {offset}");
        }
        assert!(parse_wav(&wav(1, 16, &[1])).is_err());
        assert!(parse_wav(&wav(3, 32, &f32::NAN.to_le_bytes())).is_err());
        assert!(parse_wav(&wav(3, 32, &f32::INFINITY.to_le_bytes())).is_err());
    }
    #[test]
    fn high_precision_pcm_is_rounded_instead_of_truncated() {
        assert_eq!(parse_wav(&wav(1, 24, &[0x80, 0, 0])).unwrap().samples, [1]);
        assert_eq!(parse_wav(&wav(1, 32, &[0, 0x80, 0, 0])).unwrap().samples, [1]);
    }
    #[test]
    fn extensible_float_is_not_misread_as_integer() {
        let mut bytes = wav(3, 32, &0.5f32.to_le_bytes());
        bytes[20..22].copy_from_slice(&0xfffeu16.to_le_bytes());
        bytes[16..20].copy_from_slice(&40u32.to_le_bytes());
        let mut extension = Vec::new();
        extension.extend_from_slice(&22u16.to_le_bytes());
        extension.extend_from_slice(&32u16.to_le_bytes());
        extension.extend_from_slice(&4u32.to_le_bytes());
        extension.extend_from_slice(&[3, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71]);
        bytes.splice(36..36, extension);
        let size = bytes.len() as u32 - 8; bytes[4..8].copy_from_slice(&size.to_le_bytes());
        assert_eq!(parse_wav(&bytes).unwrap().samples, [16384]);
        bytes[30] = 0; bytes[31] = 0;
        assert!(parse_wav(&bytes).is_err());
    }

}
