//! Sound annotations' samples (ISO 32000-2 §12.5.6.16, §13.3): the sound object a `Sound`
//! annotation names, read with its rate, channels, sample size and encoding, and written out as a
//! WAV file every browser plays. PDF 2.0 deprecates sound annotations; Lamina PDF reads them but
//! never writes one.

use laminapdf_cos::{Dict, Error, Limit, Object, Result};

use crate::document::Document;
use crate::id::AnnotId;
use crate::page::number;
use crate::values::read_name;

/// How a sound's samples are encoded (`/E`, Table 216).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum SoundEncoding {
    /// Unsigned values, the default.
    #[default]
    Raw,
    /// Two's-complement values.
    Signed,
    /// μ-law, 8 bits a sample.
    MuLaw,
    /// A-law, 8 bits a sample.
    ALaw,
}

impl SoundEncoding {
    fn from_name(name: &[u8]) -> Option<Self> {
        match name {
            b"Raw" => Some(Self::Raw),
            b"Signed" => Some(Self::Signed),
            b"muLaw" => Some(Self::MuLaw),
            b"ALaw" => Some(Self::ALaw),
            _ => None,
        }
    }

    /// Its `/E` name.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Raw => "Raw",
            Self::Signed => "Signed",
            Self::MuLaw => "muLaw",
            Self::ALaw => "ALaw",
        }
    }
}

/// What a sound object says of its samples.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct SoundFormat {
    /// `/R`: samples a second, per channel.
    pub rate: f64,
    /// `/C`: channels, interleaved (default 1).
    pub channels: u16,
    /// `/B`: bits a sample (default 8).
    pub bits: u16,
    /// `/E`.
    pub encoding: SoundEncoding,
    /// Whether `/CO` names a compression: such samples are left as they are, unplayed.
    pub compressed: bool,
}

impl SoundFormat {
    /// Whether [`Sound::to_wav`] can write these samples: a rate above 0 and at most 384,000,
    /// one to eight channels, one to 32 bits a sample (8 for μ-law and A-law), and no compression.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn is_playable(&self) -> bool {
        let bits_ok = match self.encoding {
            SoundEncoding::MuLaw | SoundEncoding::ALaw => self.bits == 8,
            SoundEncoding::Raw | SoundEncoding::Signed => (1..=32).contains(&self.bits),
        };
        self.rate.is_finite()
            && self.rate > 0.0
            && self.rate <= 384_000.0
            && (1..=8).contains(&self.channels)
            && bits_ok
            && !self.compressed
    }

    /// The bits a sample of the WAV file has: 8 for samples of up to 8 bits (WAV's 8-bit samples
    /// are unsigned), 16 for every other.
    fn wav_bits(&self) -> u16 {
        let linear = matches!(self.encoding, SoundEncoding::Raw | SoundEncoding::Signed);
        if linear && self.bits <= 8 { 8 } else { 16 }
    }

    /// The samples a channel holds in `bytes` of data.
    fn frames(&self, bytes: usize) -> u64 {
        let bits = u64::try_from(bytes).unwrap_or(u64::MAX).saturating_mul(8);
        bits / (u64::from(self.bits) * u64::from(self.channels)).max(1)
    }

    /// How long `bytes` of samples play, in seconds.
    ///
    /// Added in 0.6.0.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "a sound's frames are far fewer than 2^52")]
    pub fn seconds(&self, bytes: usize) -> f64 {
        if self.rate > 0.0 {
            self.frames(bytes) as f64 / self.rate
        } else {
            0.0
        }
    }
}

/// A sound annotation's samples, decoded from its sound stream.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Sound {
    /// Their format.
    pub format: SoundFormat,
    /// The samples as stored, big-endian, channels interleaved.
    pub samples: Vec<u8>,
}

impl Document {
    /// The samples of the `Sound` annotation `id`: `None` when no page lists an annotation with
    /// this ID, or it isn't a `Sound` annotation with a sound stream.
    ///
    /// # Errors
    /// Decoding errors of the stream (a filter that fails, over
    /// [`Limits::max_stream_decoded`](laminapdf_cos::Limits::max_stream_decoded)); errors that must
    /// propagate.
    ///
    /// Added in 0.6.0.
    pub fn sound(&self, id: &AnnotId) -> Result<Option<Sound>> {
        let Some(dict) = self.annotation_dict(id)? else {
            return Ok(None);
        };
        if read_name(self, dict.get("Subtype"))?.as_deref() != Some(b"Sound".as_slice()) {
            return Ok(None);
        }
        let Some(value) = dict.get("Sound") else {
            return Ok(None);
        };
        let Some(stream) = self.read(value)? else {
            return Ok(None);
        };
        let Object::Stream(stream) = &*stream else {
            return Ok(None);
        };
        let format = self.sound_format(&stream.dict)?;
        let samples = self.decode_stream(stream)?;
        Ok(Some(Sound { format, samples }))
    }

    /// The format a sound stream's dictionary gives, with the standard's defaults.
    pub(crate) fn sound_format(&self, dict: &Dict) -> Result<SoundFormat> {
        let int = |key: &str, default: i64| -> Result<i64> {
            Ok(match dict.get(key) {
                Some(v) => self.read(v)?.and_then(|v| v.as_int()).unwrap_or(default),
                None => default,
            })
        };
        let rate = match dict.get("R") {
            Some(r) => number(self, r)?.unwrap_or(0.0),
            None => 0.0,
        };
        Ok(SoundFormat {
            rate,
            channels: u16::try_from(int("C", 1)?).unwrap_or(0),
            bits: u16::try_from(int("B", 8)?).unwrap_or(0),
            encoding: read_name(self, dict.get("E"))?
                .and_then(|n| SoundEncoding::from_name(&n))
                .unwrap_or_default(),
            compressed: dict.get("CO").is_some(),
        })
    }

    /// The dictionary of the annotation `id`, if a page lists it.
    fn annotation_dict(&self, id: &AnnotId) -> Result<Option<Dict>> {
        let pages = self.pages()?;
        for page in pages.iter() {
            let mut found = None;
            self.each_annotation(&page, |each, _, dict| {
                if found.is_none() && each == *id {
                    found = Some(dict.clone());
                }
                Ok(())
            })?;
            if found.is_some() {
                return Ok(found);
            }
        }
        Ok(None)
    }
}

impl Sound {
    /// The samples as a WAV file (PCM, little-endian): samples of up to 8 bits as 8-bit unsigned
    /// ones, wider samples as 16-bit signed ones (their top 16 bits), μ-law and A-law decoded to
    /// 16-bit linear samples (ITU-T G.711). A partial last frame is dropped. `None` when the
    /// format isn't [playable](SoundFormat::is_playable).
    ///
    /// # Errors
    /// [`Error::LimitExceeded`] when the WAV file would be larger than `max_bytes` (a caller
    /// passes [`Limits::max_stream_decoded`](laminapdf_cos::Limits::max_stream_decoded)) or than
    /// a WAV file can be.
    ///
    /// Added in 0.6.0.
    pub fn to_wav(&self, max_bytes: usize) -> Result<Option<Vec<u8>>> {
        let f = &self.format;
        if !f.is_playable() {
            return Ok(None);
        }
        let out_bits = f.wav_bits();
        let samples = f.frames(self.samples.len()) * u64::from(f.channels);
        let data_len = samples * u64::from(out_bits / 8);
        let too_large = || Error::LimitExceeded {
            limit: Limit::StreamDecoded,
            at: None,
        };
        let total = data_len.checked_add(44).ok_or_else(too_large)?;
        if total > u64::try_from(max_bytes).unwrap_or(u64::MAX) || total > u64::from(u32::MAX) {
            return Err(too_large());
        }
        let mut out = Vec::with_capacity(usize::try_from(total).map_err(|_| too_large())?);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the rate is checked to be 1 to 384,000"
        )]
        let rate = f.rate.round().max(1.0) as u32;
        let block = f.channels * (out_bits / 8);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&u32::try_from(total - 8).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16_u32.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&f.channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
        out.extend_from_slice(&block.to_le_bytes());
        out.extend_from_slice(&out_bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&u32::try_from(data_len).unwrap_or(u32::MAX).to_le_bytes());
        let mut bits = Bits::new(&self.samples);
        for _ in 0..samples {
            let Some(value) = bits.take(f.bits) else { break };
            match (f.encoding, out_bits) {
                (SoundEncoding::MuLaw, _) => out.extend_from_slice(&mu_law(value).to_le_bytes()),
                (SoundEncoding::ALaw, _) => out.extend_from_slice(&a_law(value).to_le_bytes()),
                (encoding, 8) => out.push(unsigned8(value, f.bits, encoding)),
                (encoding, _) => out.extend_from_slice(&signed16(value, f.bits, encoding).to_le_bytes()),
            }
        }
        Ok(Some(out))
    }
}

/// Reads values of any width from 1 to 32 bits, most significant bit first.
struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Bits<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, width: u16) -> Option<u32> {
        let mut value = 0_u32;
        for _ in 0..width {
            let byte = *self.bytes.get(self.at / 8)?;
            let bit = (byte >> (7 - self.at % 8)) & 1;
            value = (value << 1) | u32::from(bit);
            self.at += 1;
        }
        Some(value)
    }
}

/// A sample of `bits` (1 to 8) as WAV's 8-bit unsigned value, its range stretched to 0–255.
fn unsigned8(value: u32, bits: u16, encoding: SoundEncoding) -> u8 {
    let unsigned = match encoding {
        // Two's complement to offset binary: flip the sign bit.
        SoundEncoding::Signed => value ^ (1 << (bits - 1)),
        _ => value,
    };
    let top = (1_u32 << bits) - 1;
    u8::try_from(unsigned * 255 / top).unwrap_or(u8::MAX)
}

/// A sample of `bits` (9 to 32) as WAV's 16-bit signed value: its top 16 bits, or its bits
/// shifted up.
fn signed16(value: u32, bits: u16, encoding: SoundEncoding) -> i16 {
    let unsigned = match encoding {
        SoundEncoding::Signed => value ^ (1 << (bits - 1)),
        _ => value,
    };
    let top = if bits >= 16 {
        unsigned >> (bits - 16)
    } else {
        unsigned << (16 - bits)
    };
    // Offset binary to two's complement.
    i16::from_ne_bytes(u16::try_from(top ^ 0x8000).unwrap_or(0).to_ne_bytes())
}

/// ITU-T G.711 μ-law to a 16-bit linear sample.
fn mu_law(value: u32) -> i16 {
    let u = !u8::try_from(value & 0xFF).unwrap_or(0);
    let exponent = (u >> 4) & 7;
    let mantissa = i32::from(u & 0x0F);
    let magnitude = (((mantissa << 3) + 0x84) << exponent) - 0x84;
    let sample = if u & 0x80 == 0 { magnitude } else { -magnitude };
    i16::try_from(sample.clamp(-32_768, 32_767)).unwrap_or(0)
}

/// ITU-T G.711 A-law to a 16-bit linear sample.
fn a_law(value: u32) -> i16 {
    let a = u8::try_from(value & 0xFF).unwrap_or(0) ^ 0x55;
    let exponent = (a >> 4) & 7;
    let mantissa = i32::from(a & 0x0F);
    let magnitude = match exponent {
        0 => (mantissa << 4) + 8,
        e => ((mantissa << 4) + 0x108) << (e - 1),
    };
    let sample = if a & 0x80 == 0 { -magnitude } else { magnitude };
    i16::try_from(sample.clamp(-32_768, 32_767)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(bits: u16, channels: u16, encoding: SoundEncoding, samples: Vec<u8>) -> Sound {
        Sound {
            format: SoundFormat {
                rate: 8000.0,
                channels,
                bits,
                encoding,
                compressed: false,
            },
            samples,
        }
    }

    fn data(wav: &[u8]) -> &[u8] {
        &wav[44..]
    }

    #[test]
    fn eight_bit_raw_samples_stay_as_they_are() {
        let wav = sound(8, 1, SoundEncoding::Raw, vec![0, 128, 255])
            .to_wav(1 << 20)
            .unwrap()
            .unwrap();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 8);
        assert_eq!(data(&wav), &[0, 128, 255]);
    }

    #[test]
    fn signed_eight_bit_samples_flip_their_sign_bit() {
        let wav = sound(8, 1, SoundEncoding::Signed, vec![0x80, 0x00, 0x7F])
            .to_wav(1 << 20)
            .unwrap()
            .unwrap();
        assert_eq!(data(&wav), &[0, 128, 255]);
    }

    #[test]
    fn sixteen_bit_samples_turn_little_endian() {
        let wav = sound(16, 2, SoundEncoding::Signed, vec![0x12, 0x34, 0xFF, 0xFE])
            .to_wav(1 << 20)
            .unwrap()
            .unwrap();
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 2);
        assert_eq!(data(&wav), &[0x34, 0x12, 0xFE, 0xFF]);
    }

    #[test]
    fn raw_sixteen_bit_samples_become_signed() {
        let wav = sound(16, 1, SoundEncoding::Raw, vec![0x80, 0x00, 0x00, 0x00])
            .to_wav(1 << 20)
            .unwrap()
            .unwrap();
        assert_eq!(data(&wav), &[0x00, 0x00, 0x00, 0x80]);
    }

    #[test]
    fn four_bit_samples_fill_their_byte() {
        let wav = sound(4, 1, SoundEncoding::Raw, vec![0xF0])
            .to_wav(1 << 20)
            .unwrap()
            .unwrap();
        assert_eq!(data(&wav), &[0xFF, 0x00]);
    }

    #[test]
    fn mu_law_and_a_law_decode_to_silence_and_peaks() {
        assert_eq!((mu_law(0xFF), mu_law(0x7F)), (0, 0));
        assert_eq!((mu_law(0x80), mu_law(0x00)), (32_124, -32_124));
        assert_eq!((a_law(0xD5), a_law(0x55)), (8, -8));
        assert_eq!((a_law(0xAA), a_law(0x2A)), (32_256, -32_256));
        let wav = sound(8, 1, SoundEncoding::MuLaw, vec![0xFF, 0x80])
            .to_wav(1 << 20)
            .unwrap()
            .unwrap();
        assert_eq!(data(&wav), &[0, 0, 0x7C, 0x7D]);
    }

    #[test]
    fn compressed_or_impossible_formats_are_not_played() {
        let mut s = sound(8, 1, SoundEncoding::Raw, vec![0]);
        s.format.compressed = true;
        assert_eq!(s.to_wav(1 << 20).unwrap(), None);
        assert_eq!(
            sound(12, 1, SoundEncoding::MuLaw, vec![0]).to_wav(1 << 20).unwrap(),
            None
        );
        assert_eq!(sound(8, 0, SoundEncoding::Raw, vec![0]).to_wav(1 << 20).unwrap(), None);
    }

    #[test]
    fn a_wav_over_the_limit_is_refused() {
        assert!(matches!(
            sound(8, 1, SoundEncoding::Raw, vec![0; 100]).to_wav(100),
            Err(Error::LimitExceeded { .. })
        ));
    }

    #[test]
    fn a_partial_frame_is_dropped() {
        let wav = sound(16, 1, SoundEncoding::Signed, vec![1, 2, 3])
            .to_wav(1 << 20)
            .unwrap()
            .unwrap();
        assert_eq!(data(&wav), &[2, 1]);
        assert!((sound(8, 2, SoundEncoding::Raw, vec![0; 16_000]).format.seconds(16_000) - 1.0).abs() < 1e-9);
    }
}
