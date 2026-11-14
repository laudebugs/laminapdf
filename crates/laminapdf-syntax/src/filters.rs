//! Stream filters (ISO 32000-1 §7.4): `FlateDecode` (via `miniz_oxide`), `LZWDecode`,
//! `ASCIIHexDecode`, `ASCII85Decode` and `RunLengthDecode`, with the PNG and TIFF predictors.
//! The image codecs (DCT, JPX, CCITT, JBIG2) are left to their readers: [`decode`] reports them
//! as unsupported.
//!
//! Decoding is bounded by [`Limits`]: every filter stage's output is capped by
//! [`Limits::max_stream_decoded`] and charged to the call's [`DecodeBudget`]
//! ([`Limits::max_decoded_total`]), and a `/Filter` chain may hold at most
//! [`Limits::max_filter_chain`] filters.
//!
//! ```
//! use laminapdf_syntax::filters::{decode, flate_encode};
//! use laminapdf_syntax::{DecodeBudget, Dict, Limits, Object};
//!
//! let dict: Dict = [("Filter", Object::name("FlateDecode"))].into_iter().collect();
//! let limits = Limits::default();
//! let mut budget = DecodeBudget::new(&limits);
//! let decoded = decode(&dict, &flate_encode(b"hello"), &limits, &mut budget)?;
//! assert_eq!(decoded.data, b"hello");
//! assert!(!decoded.flate_truncated);
//! assert_eq!(budget.remaining(), limits.max_decoded_total() - 5);
//! # Ok::<(), laminapdf_syntax::Error>(())
//! ```

use miniz_oxide::inflate::TINFLStatus;

use crate::error::{Error, Result};
use crate::limits::{DecodeBudget, Limit, Limits};
use crate::object::{Dict, Object};

mod lzw;
mod text;

/// A filter's data that can't be decoded, with what is wrong.
const fn bad(message: &'static str) -> Error {
    Error::InvalidStructure {
        kind: "stream filter",
        message,
    }
}

/// A filter stage's output, refused past its cap with the limit it would break.
struct Output {
    data: Vec<u8>,
    cap: usize,
    over: Limit,
}

impl Output {
    const fn new(cap: usize, over: Limit) -> Self {
        Self {
            data: Vec::new(),
            cap,
            over,
        }
    }

    /// The cap for a stage of a call within `limits`, with `budget` left.
    fn within(limits: &Limits, budget: DecodeBudget) -> Self {
        let stream_cap = limits.max_stream_decoded();
        if budget.remaining() < stream_cap {
            Self::new(budget.remaining(), Limit::DecodedTotal)
        } else {
            Self::new(stream_cap, Limit::StreamDecoded)
        }
    }

    fn room(&self, more: usize) -> Result<()> {
        if self.data.len().saturating_add(more) > self.cap {
            return Err(Error::limit(self.over, None));
        }
        Ok(())
    }

    fn push(&mut self, byte: u8) -> Result<()> {
        self.room(1)?;
        self.data.push(byte);
        Ok(())
    }

    fn extend(&mut self, bytes: &[u8]) -> Result<()> {
        self.room(bytes.len())?;
        self.data.extend_from_slice(bytes);
        Ok(())
    }

    fn fill(&mut self, byte: u8, count: usize) -> Result<()> {
        self.room(count)?;
        self.data.resize(self.data.len() + count, byte);
        Ok(())
    }

    fn set(&mut self, at: usize, byte: u8) {
        if let Some(slot) = self.data.get_mut(at) {
            *slot = byte;
        }
    }

    const fn len(&self) -> usize {
        self.data.len()
    }

    fn into_inner(self) -> Vec<u8> {
        self.data
    }
}

/// Runs `filter` on `input` within `limits`, charging its output to `budget`.
fn stage(
    input: &[u8],
    limits: &Limits,
    budget: &mut DecodeBudget,
    filter: impl FnOnce(&[u8], &mut Output) -> Result<()>,
) -> Result<Vec<u8>> {
    let mut out = Output::within(limits, *budget);
    filter(input, &mut out)?;
    let data = out.into_inner();
    budget.spend(data.len());
    Ok(data)
}

/// Decoded stream data.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Decoded {
    /// The decoded bytes.
    pub data: Vec<u8>,
    /// True when Flate data was truncated or had a bad checksum and the partial output was
    /// kept (the reader reports it as the `flate-truncated` warning).
    pub flate_truncated: bool,
}

/// Zlib-compresses `data` at the default level (6).
///
/// Added in 0.2.0.
#[must_use]
pub fn flate_encode(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(data, 6)
}

/// Inflates zlib `data`, refusing output larger than [`Limits::max_stream_decoded`] or than
/// what is left of `budget`, and charges the output to `budget`.
///
/// Truncated streams and bad checksums are tolerated when some output was produced, as
/// real-world readers do ([`Decoded::flate_truncated`] reports it).
///
/// # Errors
/// [`Error::LimitExceeded`] ([`Limit::StreamDecoded`] or [`Limit::DecodedTotal`]) when the
/// output would exceed either cap, [`Error::Decompress`] when the data is not inflatable at all.
///
/// Added in 0.2.0.
pub fn flate_decode(data: &[u8], limits: &Limits, budget: &mut DecodeBudget) -> Result<Decoded> {
    let stream_cap = limits.max_stream_decoded();
    let (cap, over) = if budget.remaining() < stream_cap {
        (budget.remaining(), Limit::DecodedTotal)
    } else {
        (stream_cap, Limit::StreamDecoded)
    };
    let (data, flate_truncated) = match miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, cap) {
        Ok(out) => (out, false),
        Err(e) => match e.status {
            TINFLStatus::HasMoreOutput => return Err(Error::limit(over, None)),
            TINFLStatus::Adler32Mismatch | TINFLStatus::FailedCannotMakeProgress if !e.output.is_empty() => {
                (e.output, true)
            }
            _ => return Err(Error::Decompress("invalid zlib data")),
        },
    };
    budget.spend(data.len());
    Ok(Decoded { data, flate_truncated })
}

/// A filter's full name for its inline-image abbreviation (ISO 32000-2 §8.9.7, Table 92):
/// `AHx` is `ASCIIHexDecode`, `CCF` is `CCITTFaxDecode`…; a full name comes back as it is.
///
/// Added in 0.4.0.
#[must_use]
pub fn full_name(name: &[u8]) -> &[u8] {
    match name {
        b"AHx" => b"ASCIIHexDecode",
        b"A85" => b"ASCII85Decode",
        b"LZW" => b"LZWDecode",
        b"Fl" => b"FlateDecode",
        b"RL" => b"RunLengthDecode",
        b"CCF" => b"CCITTFaxDecode",
        b"DCT" => b"DCTDecode",
        other => other,
    }
}

/// Decodes stream `data` according to `/Filter` and `/DecodeParms` in `dict`, within
/// `limits` and `budget` (see the [module docs](self)).
///
/// `dict` values are expected to be direct (the reader resolves references first). Data
/// without a filter is returned as is and costs no budget.
///
/// # Errors
/// [`Error::UnsupportedFilter`] for the image codecs (DCT, JPX, CCITT, JBIG2) and `/Crypt` (decryption needs
/// the file key, so the reader decrypts the stream when it loads it and removes `/Crypt` from
/// the chain before calling this), [`Error::LimitExceeded`] for a chain longer than
/// [`Limits::max_filter_chain`] or output over a decode limit, plus any decode error.
///
/// Added in 0.2.0.
pub fn decode(dict: &Dict, data: &[u8], limits: &Limits, budget: &mut DecodeBudget) -> Result<Decoded> {
    let filters: Vec<&[u8]> = match dict.get("Filter") {
        None | Some(Object::Null) => Vec::new(),
        Some(Object::Name(n)) => vec![n.as_bytes()],
        Some(Object::Array(items)) => items
            .iter()
            .map(|o| match o {
                Object::Name(n) => Ok(n.as_bytes()),
                _ => Err(Error::Type {
                    expected: "name",
                    context: "/Filter",
                }),
            })
            .collect::<Result<_>>()?,
        Some(_) => {
            return Err(Error::Type {
                expected: "name or array",
                context: "/Filter",
            });
        }
    };
    if filters.len() > limits.max_filter_chain() {
        return Err(Error::limit(Limit::FilterChain, None));
    }
    let all_parms = dict.get("DecodeParms");
    let mut current: Option<Vec<u8>> = None;
    let mut flate_truncated = false;
    for (i, filter) in filters.iter().enumerate() {
        let input = current.as_deref().unwrap_or(data);
        let parms = match all_parms {
            Some(Object::Array(a)) => a.get(i).and_then(Object::as_dict),
            Some(o) if i == 0 => o.as_dict(),
            _ => None,
        };
        let decoded = match full_name(filter) {
            b"FlateDecode" => {
                let raw = flate_decode(input, limits, budget)?;
                flate_truncated |= raw.flate_truncated;
                apply_predictor(raw.data, parms)?
            }
            b"LZWDecode" => {
                let early = int_param(parms, "EarlyChange", 1) != 0;
                let raw = stage(input, limits, budget, |data, out| lzw::decode(data, early, out))?;
                apply_predictor(raw, parms)?
            }
            b"ASCIIHexDecode" => stage(input, limits, budget, text::ascii_hex)?,
            b"ASCII85Decode" => stage(input, limits, budget, text::ascii85)?,
            b"RunLengthDecode" => stage(input, limits, budget, text::run_length)?,
            other => return Err(Error::UnsupportedFilter(String::from_utf8_lossy(other).into_owned())),
        };
        current = Some(decoded);
    }
    Ok(Decoded {
        data: current.unwrap_or_else(|| data.to_vec()),
        flate_truncated,
    })
}

fn int_param(parms: Option<&Dict>, key: &str, default: i64) -> i64 {
    parms.and_then(|p| p.get_int(key)).unwrap_or(default)
}

fn apply_predictor(data: Vec<u8>, parms: Option<&Dict>) -> Result<Vec<u8>> {
    let predictor = int_param(parms, "Predictor", 1);
    if predictor <= 1 {
        return Ok(data);
    }
    if predictor == 2 {
        return tiff_unpredict(data, parms);
    }
    if !(10..=15).contains(&predictor) {
        return Err(Error::UnsupportedFilter(format!("/Predictor {predictor}")));
    }
    let colors = int_param(parms, "Colors", 1);
    let bpc = int_param(parms, "BitsPerComponent", 8);
    let columns = int_param(parms, "Columns", 1);
    let bad = |message| Error::InvalidStructure {
        kind: "PNG predictor",
        message,
    };
    if !(1..=32).contains(&colors) || ![1, 2, 4, 8, 16].contains(&bpc) || !(1..=1 << 24).contains(&columns) {
        return Err(bad("parameters out of range"));
    }
    // Do the row-length math in u64 so it is exact on 32-bit targets (wasm32) as well:
    // 32 colours × 16 bits × 2^24 columns is 2^33 bits, which would overflow a 32-bit usize.
    let bits_per_pixel = u64::try_from(colors * bpc).map_err(|_| bad("overflow"))?;
    let columns = u64::try_from(columns).map_err(|_| bad("overflow"))?;
    let row_bits = bits_per_pixel.checked_mul(columns).ok_or(bad("overflow"))?;
    let row_len = row_bits.div_ceil(8);
    // A row is a filter byte plus `row_len` bytes, so a row wider than the whole input can
    // never be complete. Rejecting it caps the `prev` row buffer at the input size instead
    // of letting crafted parameters force a large allocation.
    let input_len = u64::try_from(data.len()).map_err(|_| bad("overflow"))?;
    if row_len > input_len.saturating_add(1) {
        return Err(bad("row length exceeds stream data"));
    }
    let row_len = usize::try_from(row_len).map_err(|_| bad("overflow"))?;
    let bpp = usize::try_from(bits_per_pixel.div_ceil(8).max(1)).map_err(|_| bad("overflow"))?;
    png_unpredict(&data, row_len, bpp)
}

/// Reverses TIFF predictor 2 (horizontal differencing): each sample is the difference from the
/// same component of the pixel to its left, for 8- and 16-bit samples.
fn tiff_unpredict(mut data: Vec<u8>, parms: Option<&Dict>) -> Result<Vec<u8>> {
    let colors = int_param(parms, "Colors", 1);
    let bpc = int_param(parms, "BitsPerComponent", 8);
    let columns = int_param(parms, "Columns", 1);
    let bad = |message| Error::InvalidStructure {
        kind: "TIFF predictor",
        message,
    };
    if !(1..=32).contains(&colors) || !(1..=1 << 24).contains(&columns) {
        return Err(bad("parameters out of range"));
    }
    let bytes = match bpc {
        8 => 1,
        16 => 2,
        _ => {
            return Err(Error::UnsupportedFilter(format!(
                "/Predictor 2 with {bpc} bits per component"
            )));
        }
    };
    let colors = usize::try_from(colors).map_err(|_| bad("overflow"))?;
    let columns = usize::try_from(columns).map_err(|_| bad("overflow"))?;
    let pixel = colors * bytes;
    let row = pixel.checked_mul(columns).ok_or(bad("overflow"))?;
    for line in data.chunks_mut(row) {
        for at in (pixel..line.len()).step_by(bytes) {
            let left = at - pixel;
            if bytes == 1 {
                let value = line
                    .get(at)
                    .copied()
                    .unwrap_or(0)
                    .wrapping_add(line.get(left).copied().unwrap_or(0));
                if let Some(slot) = line.get_mut(at) {
                    *slot = value;
                }
            } else {
                let read = |line: &[u8], i: usize| {
                    u16::from_be_bytes([line.get(i).copied().unwrap_or(0), line.get(i + 1).copied().unwrap_or(0)])
                };
                let value = read(line, at).wrapping_add(read(line, left)).to_be_bytes();
                if let Some(slot) = line.get_mut(at..at + 2) {
                    slot.copy_from_slice(&value);
                }
            }
        }
    }
    Ok(data)
}

/// Reverses PNG row filters. Each input row is a filter-type byte followed by `row_len`
/// bytes; a short final row is decoded as far as it goes.
fn png_unpredict(data: &[u8], row_len: usize, bpp: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    let mut prev = vec![0u8; row_len];
    let mut cur = vec![0u8; row_len];
    for chunk in data.chunks(row_len + 1) {
        let Some((&kind, row)) = chunk.split_first() else {
            continue;
        };
        let cur = cur.get_mut(..row.len()).unwrap_or_default();
        unfilter_row(kind, row, &prev, cur, bpp)?;
        out.extend_from_slice(cur);
        prev.iter_mut().zip(&*cur).for_each(|(p, c)| *p = *c);
    }
    Ok(out)
}

/// Reverses one row's PNG filter (PNG §9.2): `raw` (the row after its filter-type byte) into
/// `out`, given the previous row's unfiltered bytes (zeros for the first row) and the bytes
/// per complete pixel (at least 1). Flate's PNG predictors (§7.4.4.4) and PNG images share it.
///
/// # Errors
/// [`Error::InvalidStructure`] for a filter type other than 0–4.
///
/// Added in 0.2.0.
pub fn unfilter_row(filter: u8, raw: &[u8], prev: &[u8], out: &mut [u8], bpp: usize) -> Result<()> {
    for i in 0..raw.len() {
        let x = raw.get(i).copied().unwrap_or(0);
        let a = i.checked_sub(bpp).and_then(|j| out.get(j)).copied().unwrap_or(0);
        let b = prev.get(i).copied().unwrap_or(0);
        let c = i.checked_sub(bpp).and_then(|j| prev.get(j)).copied().unwrap_or(0);
        let value = match filter {
            0 => x,
            1 => x.wrapping_add(a),
            2 => x.wrapping_add(b),
            3 => x.wrapping_add(a.midpoint(b)),
            4 => x.wrapping_add(paeth(a, b, c)),
            _ => {
                return Err(Error::InvalidStructure {
                    kind: "PNG predictor",
                    message: "unknown row filter",
                });
            }
        };
        if let Some(o) = out.get_mut(i) {
            *o = value;
        }
    }
    Ok(())
}

/// The Paeth predictor (PNG §9.4): of `a` (left), `b` (up) and `c` (up-left), the one closest
/// to `a + b - c`.
///
/// Added in 0.2.0.
#[must_use]
pub fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (ia, ib, ic) = (i16::from(a), i16::from(b), i16::from(c));
    let p = ia + ib - ic;
    let (pa, pb, pc) = ((p - ia).abs(), (p - ib).abs(), (p - ic).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(max_stream: usize) -> Limits {
        Limits::default().with_max_stream_decoded(max_stream)
    }

    fn inflate(data: &[u8], max_stream: usize) -> Result<Vec<u8>> {
        let limits = limits(max_stream);
        flate_decode(data, &limits, &mut DecodeBudget::new(&limits)).map(|d| d.data)
    }

    fn decode_with(dict: &Dict, data: &[u8], max_stream: usize) -> Result<Vec<u8>> {
        let limits = limits(max_stream);
        decode(dict, data, &limits, &mut DecodeBudget::new(&limits)).map(|d| d.data)
    }

    fn flate_chain(n: usize) -> Dict {
        [("Filter", Object::Array(vec![Object::name("FlateDecode"); n]))]
            .into_iter()
            .collect()
    }

    fn encode_times(data: &[u8], n: usize) -> Vec<u8> {
        (0..n).fold(data.to_vec(), |acc, _| flate_encode(&acc))
    }

    fn predictor_parms(columns: i64) -> Dict {
        [("Predictor", 12), ("Columns", columns)].into_iter().collect()
    }

    #[test]
    fn flate_round_trips() {
        let data = b"hello hello hello".to_vec();
        assert_eq!(inflate(&flate_encode(&data), 1024), Ok(data));
    }

    #[test]
    fn flate_decode_accepts_output_at_the_stream_limit() {
        assert_eq!(inflate(&flate_encode(&[0u8; 1000]), 1000).map(|d| d.len()), Ok(1000));
    }

    #[test]
    fn flate_decode_refuses_output_past_the_stream_limit() {
        assert_eq!(
            inflate(&flate_encode(&[0u8; 1001]), 1000),
            Err(Error::LimitExceeded {
                limit: Limit::StreamDecoded,
                at: None
            })
        );
    }

    #[test]
    fn flate_decode_charges_its_output_to_the_budget() {
        let limits = Limits::default();
        let mut budget = DecodeBudget::new(&limits);
        flate_decode(&flate_encode(&[0u8; 100]), &limits, &mut budget).unwrap();
        assert_eq!(budget.remaining(), limits.max_decoded_total() - 100);
    }

    #[test]
    fn flate_decode_accepts_output_that_spends_the_whole_budget() {
        let limits = Limits::default().with_max_decoded_total(100);
        let mut budget = DecodeBudget::new(&limits);
        assert!(flate_decode(&flate_encode(&[0u8; 100]), &limits, &mut budget).is_ok());
    }

    #[test]
    fn flate_decode_refuses_output_past_the_remaining_budget() {
        let limits = Limits::default().with_max_decoded_total(150);
        let mut budget = DecodeBudget::new(&limits);
        let data = flate_encode(&[0u8; 100]);
        flate_decode(&data, &limits, &mut budget).unwrap();
        assert_eq!(
            flate_decode(&data, &limits, &mut budget),
            Err(Error::LimitExceeded {
                limit: Limit::DecodedTotal,
                at: None
            })
        );
    }

    #[test]
    fn flate_decode_keeps_truncated_output_and_flags_it() {
        let full = flate_encode(&[7u8; 4000]);
        let cut = full.get(..full.len() - 6).unwrap();
        let limits = Limits::default();
        let decoded = flate_decode(cut, &limits, &mut DecodeBudget::new(&limits)).unwrap();
        assert!(decoded.flate_truncated && !decoded.data.is_empty());
    }

    #[test]
    fn flate_decode_rejects_garbage() {
        assert!(inflate(b"not zlib", 1000).is_err());
    }

    #[test]
    fn decode_accepts_a_filter_chain_at_the_limit() {
        let limits = Limits::default().with_max_filter_chain(3);
        let decoded = decode(
            &flate_chain(3),
            &encode_times(b"x", 3),
            &limits,
            &mut DecodeBudget::new(&limits),
        );
        assert_eq!(decoded.map(|d| d.data), Ok(b"x".to_vec()));
    }

    #[test]
    fn decode_rejects_a_filter_chain_past_the_limit() {
        let limits = Limits::default().with_max_filter_chain(3);
        assert_eq!(
            decode(
                &flate_chain(4),
                &encode_times(b"x", 4),
                &limits,
                &mut DecodeBudget::new(&limits)
            ),
            Err(Error::LimitExceeded {
                limit: Limit::FilterChain,
                at: None
            })
        );
    }

    #[test]
    fn decode_charges_every_stage_of_a_chain_to_the_budget() {
        let limits = Limits::default();
        let mut budget = DecodeBudget::new(&limits);
        let encoded = encode_times(&[0u8; 1000], 2);
        let once = flate_encode(&[0u8; 1000]).len();
        decode(&flate_chain(2), &encoded, &limits, &mut budget).unwrap();
        assert_eq!(budget.remaining(), limits.max_decoded_total() - 1000 - once);
    }

    #[test]
    fn png_up_predictor_adds_previous_row() {
        // Two rows of 2 columns, filter type 2 (Up).
        let encoded = [2, 1, 2, 2, 1, 1];
        assert_eq!(png_unpredict(&encoded, 2, 1), Ok(vec![1, 2, 2, 3]));
    }

    #[test]
    fn png_paeth_and_sub_predictors_decode() {
        let encoded = [1, 5, 1, 4, 0, 0];
        assert_eq!(png_unpredict(&encoded, 2, 1), Ok(vec![5, 6, 5, 6]));
    }

    #[test]
    fn decode_applies_predictor_after_flate() {
        let dict: Dict = [
            ("Filter", Object::name("FlateDecode")),
            ("DecodeParms", Object::Dict(predictor_parms(2))),
        ]
        .into_iter()
        .collect();
        let encoded = flate_encode(&[2, 1, 2, 2, 1, 1]);
        assert_eq!(decode_with(&dict, &encoded, 1024), Ok(vec![1, 2, 2, 3]));
    }

    #[test]
    fn decode_should_reject_predictor_row_longer_than_stream_data() {
        // 32 colours × 16 bits × 16,777,215 columns ≈ 1 GiB per row; the stream holds 6 bytes.
        let parms: Dict = [
            ("Predictor", 12),
            ("Colors", 32),
            ("BitsPerComponent", 16),
            ("Columns", 16_777_215),
        ]
        .into_iter()
        .collect();
        let dict: Dict = [
            ("Filter", Object::name("FlateDecode")),
            ("DecodeParms", Object::Dict(parms)),
        ]
        .into_iter()
        .collect();
        let encoded = flate_encode(&[2, 1, 2, 2, 1, 1]);
        assert_eq!(
            decode_with(&dict, &encoded, 1024),
            Err(Error::InvalidStructure {
                kind: "PNG predictor",
                message: "row length exceeds stream data",
            })
        );
    }

    #[test]
    fn decode_should_reject_out_of_range_predictor_colors() {
        let parms: Dict = [("Predictor", 12), ("Colors", 33)].into_iter().collect();
        let dict: Dict = [
            ("Filter", Object::name("FlateDecode")),
            ("DecodeParms", Object::Dict(parms)),
        ]
        .into_iter()
        .collect();
        assert!(decode_with(&dict, &flate_encode(&[0, 1]), 1024).is_err());
    }

    #[test]
    fn decode_runs_a_chain_of_text_and_lzw_filters() {
        // "Hello" as hex, then that text as ASCII85: decoded in the chain's order.
        let dict: Dict = [("Filter", Object::Array(vec![Object::name("A85"), Object::name("AHx")]))]
            .into_iter()
            .collect();
        assert_eq!(decode_with(&dict, b"1cRBN2IeXT2J,,~>", 100), Ok(b"Hello".to_vec()));
    }

    #[test]
    fn decode_charges_a_text_filter_to_the_limits() {
        let dict: Dict = [("Filter", Object::name("RunLengthDecode"))].into_iter().collect();
        assert!(matches!(
            decode_with(&dict, &[200, b'x'], 10),
            Err(Error::LimitExceeded { .. })
        ));
    }

    #[test]
    fn tiff_predictor_adds_the_pixel_to_the_left() {
        let parms: Dict = [("Predictor", 2), ("Colors", 2), ("Columns", 3)].into_iter().collect();
        assert_eq!(
            apply_predictor(vec![1, 10, 1, 1, 1, 1, 5, 5, 0, 0, 1, 2], Some(&parms)),
            Ok(vec![1, 10, 2, 11, 3, 12, 5, 5, 5, 5, 6, 7])
        );
    }

    #[test]
    fn decode_reports_unsupported_filter() {
        let dict: Dict = [("Filter", Object::name("DCTDecode"))].into_iter().collect();
        assert_eq!(
            decode_with(&dict, b"", 10),
            Err(Error::UnsupportedFilter("DCTDecode".into()))
        );
    }

    #[test]
    fn decode_does_not_pass_crypt_through() {
        // Undecrypted data must never be treated as plaintext: the reader strips `/Crypt`
        // only after decrypting.
        let dict: Dict = [("Filter", Object::name("Crypt"))].into_iter().collect();
        assert_eq!(
            decode_with(&dict, b"x", 10),
            Err(Error::UnsupportedFilter("Crypt".into()))
        );
    }
}
