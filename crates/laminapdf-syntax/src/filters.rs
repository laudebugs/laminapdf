//! Stream filters: Flate (via `miniz_oxide`) and PNG predictors (ISO 32000-1 §7.4.4).
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

/// Decoded stream data.
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

/// Decodes stream `data` according to `/Filter` and `/DecodeParms` in `dict`, within
/// `limits` and `budget` (see the [module docs](self)).
///
/// `dict` values are expected to be direct (the reader resolves references first). Data
/// without a filter is returned as is and costs no budget.
///
/// # Errors
/// [`Error::UnsupportedFilter`] for anything but Flate (including `/Crypt`: decryption needs
/// the file key, so the reader decrypts the stream when it loads it and removes `/Crypt` from
/// the chain before calling this), [`Error::LimitExceeded`] for a chain longer than
/// [`Limits::max_filter_chain`] or output over a decode limit, plus any decode error.
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
        let decoded = match *filter {
            b"FlateDecode" | b"Fl" => {
                let raw = flate_decode(input, limits, budget)?;
                flate_truncated |= raw.flate_truncated;
                apply_predictor(raw.data, parms)?
            }
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
    if !(10..=15).contains(&predictor) {
        return Err(Error::UnsupportedFilter(format!(
            "FlateDecode with /Predictor {predictor}"
        )));
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

/// Reverses PNG row filters. Each input row is a filter-type byte followed by `row_len`
/// bytes; a short final row is decoded as far as it goes.
fn png_unpredict(data: &[u8], row_len: usize, bpp: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    let mut prev = vec![0u8; row_len];
    for chunk in data.chunks(row_len + 1) {
        let Some((&kind, row)) = chunk.split_first() else {
            continue;
        };
        let mut cur = row.to_vec();
        for i in 0..cur.len() {
            let left = if i >= bpp {
                cur.get(i - bpp).copied().unwrap_or(0)
            } else {
                0
            };
            let up = prev.get(i).copied().unwrap_or(0);
            let up_left = if i >= bpp {
                prev.get(i - bpp).copied().unwrap_or(0)
            } else {
                0
            };
            let predicted = match kind {
                0 => 0,
                1 => left,
                2 => up,
                3 => left.midpoint(up),
                4 => paeth(left, up, up_left),
                _ => {
                    return Err(Error::InvalidStructure {
                        kind: "PNG predictor",
                        message: "unknown row filter",
                    });
                }
            };
            if let Some(b) = cur.get_mut(i) {
                *b = b.wrapping_add(predicted);
            }
        }
        out.extend_from_slice(&cur);
        prev.iter_mut().zip(&cur).for_each(|(p, c)| *p = *c);
    }
    Ok(out)
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let (pa, pb, pc) = (
        (p - i16::from(a)).abs(),
        (p - i16::from(b)).abs(),
        (p - i16::from(c)).abs(),
    );
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
