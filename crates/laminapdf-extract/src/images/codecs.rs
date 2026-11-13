//! An image stream's filters, split at its image codec (ISO 32000-2 §7.4): the general filters
//! before it decode as any stream's do, and the codec's data is either kept (DCT, JPX) or
//! decoded here to 1-bit samples (CCITT fax, JBIG2).

use laminapdf_model::Document;
use laminapdf_syntax::{Dict, Object, Stream};

/// The image codec a stream ends with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Codec {
    /// No codec: the stream decodes to samples.
    None,
    /// `DCTDecode`: a JPEG.
    Dct,
    /// `JPXDecode`: a JPEG 2000 codestream or file.
    Jpx,
    /// `CCITTFaxDecode`.
    Ccitt,
    /// `JBIG2Decode`.
    Jbig2,
}

/// A stream's filter names, full and abbreviated alike, in order.
pub(crate) fn filter_names(doc: &Document, dict: &Dict) -> Vec<String> {
    let names = |object: &Object| -> Vec<String> {
        match object {
            Object::Name(name) => vec![full_name(name.as_bytes())],
            Object::Array(items) => items
                .iter()
                .filter_map(|item| match item {
                    Object::Name(name) => Some(full_name(name.as_bytes())),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    dict.get("Filter")
        .and_then(|filter| doc.deref(filter).ok().map(|resolved| names(&resolved)))
        .unwrap_or_default()
}

/// An inline image's abbreviated filter name, written out (§8.9.7, Table 92).
fn full_name(name: &[u8]) -> String {
    let full: &[u8] = match name {
        b"AHx" => b"ASCIIHexDecode",
        b"A85" => b"ASCII85Decode",
        b"LZW" => b"LZWDecode",
        b"Fl" => b"FlateDecode",
        b"RL" => b"RunLengthDecode",
        b"CCF" => b"CCITTFaxDecode",
        b"DCT" => b"DCTDecode",
        other => other,
    };
    String::from_utf8_lossy(full).into_owned()
}

fn codec_of(name: &str) -> Codec {
    match name {
        "DCTDecode" => Codec::Dct,
        "JPXDecode" => Codec::Jpx,
        "CCITTFaxDecode" => Codec::Ccitt,
        "JBIG2Decode" => Codec::Jbig2,
        _ => Codec::None,
    }
}

/// The stream's data decoded up to its image codec, the codec, and the codec's parameters.
///
/// # Errors
/// Why it can't be decoded: a codec before another filter, or a filter's error.
pub(crate) fn decode_to_codec(doc: &Document, stream: &Stream) -> Result<(Vec<u8>, Codec, Dict), String> {
    let names = filter_names(doc, &stream.dict);
    let codecs = names.iter().filter(|n| codec_of(n) != Codec::None).count();
    let last = names.last().map_or(Codec::None, |n| codec_of(n));
    if codecs > 1 || (codecs == 1 && last == Codec::None) {
        return Err("an image codec before another filter".into());
    }
    if last == Codec::None {
        let data = doc.decode_stream(stream).map_err(|e| e.to_string())?;
        return Ok((data, Codec::None, Dict::new()));
    }
    // The chain without its codec, and the codec's own /DecodeParms.
    let keep = names.len() - 1;
    let parms_of = |i: usize| -> Option<Object> {
        let parms = doc.deref(stream.dict.get("DecodeParms")?).ok()?;
        match &*parms {
            Object::Array(items) => items.get(i).cloned(),
            other if i == 0 => Some(other.clone()),
            _ => None,
        }
    };
    let codec_parms = parms_of(keep)
        .and_then(|p| doc.deref(&p).ok().and_then(|r| r.as_dict().cloned()))
        .unwrap_or_default();
    let mut dict = stream.dict.clone();
    dict.remove("Filter");
    dict.remove("DecodeParms");
    if keep > 0 {
        dict.insert(
            "Filter",
            Object::Array(names.iter().take(keep).map(|n| Object::name(n.as_str())).collect()),
        );
        dict.insert(
            "DecodeParms",
            Object::Array((0..keep).map(|i| parms_of(i).unwrap_or(Object::Null)).collect()),
        );
    }
    let data = doc
        .decode_stream(&Stream::new(dict, stream.data.clone()))
        .map_err(|e| e.to_string())?;
    Ok((data, last, codec_parms))
}

/// Packs runs of pixels into 1-bit rows, each row starting on a byte.
struct Bits {
    width: usize,
    rows: usize,
    data: Vec<u8>,
    x: usize,
    y: usize,
    /// The bit a black pixel takes.
    black: u8,
}

impl Bits {
    fn new(width: usize, rows: usize, black: u8) -> Self {
        Self {
            width,
            rows,
            data: vec![if black == 1 { 0 } else { 0xff }; width.div_ceil(8) * rows],
            x: 0,
            y: 0,
            black,
        }
    }

    fn pixel(&mut self, black: bool) {
        if self.x < self.width && self.y < self.rows && black {
            let at = self.y * self.width.div_ceil(8) + self.x / 8;
            let bit = 0x80 >> (self.x % 8);
            if let Some(byte) = self.data.get_mut(at) {
                if self.black == 1 {
                    *byte |= bit;
                } else {
                    *byte &= !bit;
                }
            }
        }
        self.x += 1;
    }

    fn line(&mut self) {
        self.x = 0;
        self.y += 1;
    }
}

impl hayro_ccitt::Decoder for Bits {
    fn push_pixels(&mut self, white: bool, count: u32) {
        for _ in 0..count {
            self.pixel(!white);
        }
    }

    fn next_line(&mut self) {
        self.line();
    }
}

impl hayro_jbig2::Decoder for Bits {
    fn push_pixel(&mut self, black: bool) {
        self.pixel(black);
    }

    fn push_pixel_chunk(&mut self, black: bool, chunk_count: u32) {
        for _ in 0..chunk_count * 8 {
            self.pixel(black);
        }
    }

    fn next_line(&mut self) {
        self.line();
    }
}

/// CCITT fax data as 1-bit rows, as the filter gives them (§7.4.6): with `/BlackIs1` false,
/// the default, a black pixel is 0.
///
/// # Errors
/// Data the decoder can't read at all; rows it decoded before an error are kept.
pub(crate) fn ccitt(data: &[u8], parms: &Dict, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let int = |key: &str, default: i64| parms.get_int(key).unwrap_or(default);
    let flag = |key: &str, default: bool| match parms.get(key) {
        Some(Object::Bool(b)) => *b,
        _ => default,
    };
    let columns = u32::try_from(int("Columns", 1728)).unwrap_or(width).max(1);
    let rows = u32::try_from(int("Rows", i64::from(height))).unwrap_or(height);
    let k = int("K", 0);
    let encoding = match k {
        k if k < 0 => hayro_ccitt::EncodingMode::Group4,
        0 => hayro_ccitt::EncodingMode::Group3_1D,
        k => hayro_ccitt::EncodingMode::Group3_2D {
            k: u32::try_from(k).unwrap_or(u32::MAX),
        },
    };
    let settings = hayro_ccitt::DecodeSettings {
        columns,
        rows,
        end_of_block: flag("EndOfBlock", true),
        end_of_line: flag("EndOfLine", false),
        rows_are_byte_aligned: flag("EncodedByteAlign", false),
        encoding,
        invert_black: false,
    };
    let black = u8::from(flag("BlackIs1", false));
    let mut bits = Bits::new(columns as usize, rows as usize, black);
    let mut ctx = hayro_ccitt::DecoderContext::new(settings);
    let result = hayro_ccitt::decode(data, &mut bits, &mut ctx);
    if result.is_err() && bits.y == 0 {
        return Err("the CCITT fax data doesn't decode".into());
    }
    Ok(bits.data)
}

/// JBIG2 data, with its `/JBIG2Globals`, as 1-bit rows in which a black pixel is 1 (§7.4.7).
///
/// # Errors
/// Data the decoder can't read.
pub(crate) fn jbig2(doc: &Document, data: &[u8], parms: &Dict) -> Result<(u32, u32, Vec<u8>), String> {
    let globals = parms
        .get("JBIG2Globals")
        .and_then(|g| doc.deref(g).ok())
        .and_then(|g| match &*g {
            Object::Stream(s) => doc.decode_stream(s).ok(),
            _ => None,
        });
    let image = hayro_jbig2::Image::new_embedded(data, globals.as_deref())
        .map_err(|_| "the JBIG2 data doesn't decode".to_owned())?;
    let (width, height) = (image.width(), image.height());
    let mut bits = Bits::new(width as usize, height as usize, 1);
    image
        .decode(&mut bits)
        .map_err(|_| "the JBIG2 data doesn't decode".to_owned())?;
    Ok((width, height, bits.data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_pack_runs_into_rows_that_start_on_a_byte() {
        let mut bits = Bits::new(10, 2, 1);
        hayro_jbig2::Decoder::push_pixel(&mut bits, true);
        hayro_jbig2::Decoder::push_pixel(&mut bits, false);
        hayro_jbig2::Decoder::next_line(&mut bits);
        hayro_jbig2::Decoder::push_pixel_chunk(&mut bits, true, 1);
        assert_eq!(bits.data, vec![0x80, 0, 0xff, 0]);
    }

    #[test]
    fn with_black_as_zero_the_rows_start_white() {
        let mut bits = Bits::new(4, 1, 0);
        hayro_ccitt::Decoder::push_pixels(&mut bits, false, 1);
        assert_eq!(bits.data, vec![0x7f]);
    }

    #[test]
    fn abbreviated_filter_names_are_written_out() {
        assert_eq!(full_name(b"CCF"), "CCITTFaxDecode");
        assert_eq!(codec_of(&full_name(b"DCT")), Codec::Dct);
    }
}
