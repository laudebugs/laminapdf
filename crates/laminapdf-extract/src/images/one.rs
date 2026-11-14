//! One image stream into the files it gives: itself, and its mask when it is a JPEG or JPEG
//! 2000 image, which can't hold one.

use laminapdf_model::Document;
use laminapdf_syntax::{ObjRef, Object, Stream};

use super::codecs::{self, Codec};
use super::color::{self, Space};
use super::samples::{self, Layout, Mask, Pixels};
use super::{ExtractedImage, ImageFileFormat, ImageKind, png};

/// The most pixels an image may have: past it the image is skipped rather than decoded.
const MAX_PIXELS: u64 = 1 << 28;

/// An image to extract.
pub(super) struct Input<'a> {
    pub(super) stream: &'a Stream,
    pub(super) object: Option<ObjRef>,
    pub(super) page: usize,
    pub(super) resources: Option<&'a Object>,
}

/// The numbers of array `key` in `dict`, resolved.
fn numbers(doc: &Document, dict: &laminapdf_syntax::Dict, key: &str) -> Option<Vec<f64>> {
    let array = doc.deref(dict.get(key)?).ok()?;
    Some(array.as_array()?.iter().filter_map(Object::as_f64).collect())
}

/// The image's own entries.
struct Entries {
    layout: Layout,
    stencil: bool,
    decode: Option<Vec<f64>>,
}

fn entries(doc: &Document, stream: &Stream) -> Result<Entries, String> {
    let dict = &stream.dict;
    let int = |key: &str| dict.get(key).and_then(|v| doc.deref(v).ok()?.as_int());
    let width = int("Width").and_then(|w| u32::try_from(w).ok()).filter(|&w| w > 0);
    let height = int("Height").and_then(|h| u32::try_from(h).ok()).filter(|&h| h > 0);
    let (Some(width), Some(height)) = (width, height) else {
        return Err("no width or height".into());
    };
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(format!("{width} × {height} pixels, over the limit of {MAX_PIXELS}"));
    }
    let stencil = matches!(
        dict.get("ImageMask").and_then(|v| doc.deref(v).ok()).as_deref(),
        Some(Object::Bool(true))
    );
    let bits = if stencil {
        1
    } else {
        int("BitsPerComponent").and_then(|b| u8::try_from(b).ok()).unwrap_or(8)
    };
    if ![1, 2, 4, 8, 16].contains(&bits) {
        return Err(format!("{bits} bits per component"));
    }
    let decode = dict.get("Decode").and_then(|d| doc.deref(d).ok()).and_then(|d| {
        d.as_array()
            .map(|items| items.iter().filter_map(Object::as_f64).collect())
    });
    Ok(Entries {
        layout: Layout { width, height, bits },
        stencil,
        decode,
    })
}

/// The files `input` gives.
///
/// # Errors
/// Why it can't be extracted, in English.
pub(super) fn extract(doc: &Document, input: &Input<'_>) -> Result<Vec<ExtractedImage>, String> {
    let stream = input.stream;
    let mut entries = entries(doc, stream)?;
    let filters = codecs::filter_names(doc, &stream.dict);
    let (data, codec, parms) = codecs::decode_to_codec(doc, stream)?;
    let space_object = stream.dict.get("ColorSpace");
    let space = space_object.map(|cs| color::parse(doc, cs, input.resources));
    let family = if entries.stencil {
        "-".to_owned()
    } else {
        space.as_ref().map_or_else(|| "-".to_owned(), Space::family)
    };
    let record = |kind, format, data, layout: Layout, color_space: String| ExtractedImage {
        page: input.page,
        object: input.object,
        kind,
        width: layout.width,
        height: layout.height,
        bits_per_component: layout.bits,
        color_space,
        filters: filters.clone(),
        format,
        data,
    };
    let format = match codec {
        Codec::Dct => Some(ImageFileFormat::Jpeg),
        Codec::Jpx => Some(ImageFileFormat::Jpeg2000),
        _ => None,
    };
    if let Some(format) = format {
        if data.is_empty() {
            return Err("no image data".into());
        }
        // Kept as stored; its mask, which the file can't hold, follows as an image of its own.
        let mut out = vec![record(ImageKind::Image, format, data, entries.layout, family)];
        out.extend(passthrough_mask(doc, input)?);
        return Ok(out);
    }
    let samples = match codec {
        Codec::Ccitt => codecs::ccitt(&data, &parms, entries.layout.width, entries.layout.height)?,
        Codec::Jbig2 => {
            let (width, height, bits) = codecs::jbig2(doc, &data, &parms)?;
            entries.layout = Layout { width, height, bits: 1 };
            bits
        }
        _ => data,
    };
    if entries.stencil {
        let alpha = samples::stencil_alpha(&samples, entries.layout, inverted(entries.decode.as_deref()));
        let png = png::encode(&stencil_pixels(&alpha, entries.layout))?;
        return Ok(vec![record(
            ImageKind::Stencil,
            ImageFileFormat::Png,
            png,
            entries.layout,
            family,
        )]);
    }
    let Some(space) = space else {
        return Err("no colour space".into());
    };
    if let Space::Unsupported(name) = &space {
        return Err(format!("its colour space: {name}"));
    }
    let raw = samples::unpack(&samples, entries.layout, space.components());
    let mask = alpha(doc, input, entries.layout, &raw, space.components())?;
    let ranges = samples::decode_ranges(&space, entries.decode.as_deref(), entries.layout.bits);
    let pixels = samples::pixels(&raw, entries.layout, &space, &ranges, mask.as_ref());
    let png = png::encode(&pixels)?;
    Ok(vec![record(
        ImageKind::Image,
        ImageFileFormat::Png,
        png,
        entries.layout,
        family,
    )])
}

/// Whether a stencil's `/Decode` is `[1 0]`.
fn inverted(decode: Option<&[f64]>) -> bool {
    matches!(decode, Some(&[lo, hi, ..]) if lo > hi)
}

/// A stencil as grey pixels: painted black, the rest white.
fn stencil_pixels(alpha: &[u8], layout: Layout) -> Pixels {
    samples::gray(layout, alpha.iter().map(|a| 255 - a).collect())
}

/// The image's opacity from its `/SMask`, `/Mask` stream or `/Mask` colour key, when it has one.
fn alpha(
    doc: &Document,
    input: &Input<'_>,
    layout: Layout,
    raw: &[u16],
    components: usize,
) -> Result<Option<Mask>, String> {
    let dict = &input.stream.dict;
    if let Some((mask, mask_layout)) = mask_stream(doc, input, "SMask")? {
        let decode = numbers(doc, &mask.dict, "Decode");
        let alpha = samples::soft_alpha(&mask_data(doc, &mask)?, mask_layout, decode.as_deref());
        let matte = numbers(doc, &mask.dict, "Matte").filter(|m| m.len() == components);
        return Ok(Some(Mask {
            alpha: samples::resample(&alpha, mask_layout, layout),
            matte,
        }));
    }
    if let Some((mask, mask_layout)) = mask_stream(doc, input, "Mask")? {
        let decode = numbers(doc, &mask.dict, "Decode");
        let alpha = samples::stencil_alpha(&mask_data(doc, &mask)?, mask_layout, inverted(decode.as_deref()));
        return Ok(Some(Mask {
            alpha: samples::resample(&alpha, mask_layout, layout),
            matte: None,
        }));
    }
    let key = dict.get("Mask").and_then(|m| doc.deref(m).ok());
    if let Some(Object::Array(items)) = key.as_deref() {
        let numbers: Vec<u16> = items
            .iter()
            .filter_map(|i| i.as_int().and_then(|v| u16::try_from(v).ok()))
            .collect();
        let ranges: Vec<(u16, u16)> = numbers
            .chunks(2)
            .filter_map(|p| Some((*p.first()?, *p.get(1)?)))
            .collect();
        if ranges.len() == components {
            return Ok(Some(Mask {
                alpha: samples::color_key_alpha(raw, components, &ranges),
                matte: None,
            }));
        }
    }
    Ok(None)
}

/// The image's `/SMask` or `/Mask` stream, with its size and bits.
fn mask_stream(doc: &Document, input: &Input<'_>, key: &str) -> Result<Option<(Stream, Layout)>, String> {
    let Some(found) = input.stream.dict.get(key).and_then(|m| doc.deref(m).ok()) else {
        return Ok(None);
    };
    let Object::Stream(mask) = &*found else {
        return Ok(None);
    };
    let mask = mask.clone();
    let layout = entries(doc, &mask)?.layout;
    // A /Mask stream is a stencil: one bit, whatever it says.
    let layout = if key == "Mask" {
        Layout { bits: 1, ..layout }
    } else {
        layout
    };
    Ok(Some((mask, layout)))
}

/// A mask's samples: decoded through its filters, a CCITT or JBIG2 mask included.
fn mask_data(doc: &Document, mask: &Stream) -> Result<Vec<u8>, String> {
    let (data, codec, parms) = codecs::decode_to_codec(doc, mask)?;
    let layout = entries(doc, mask)?.layout;
    match codec {
        Codec::None => Ok(data),
        Codec::Ccitt => codecs::ccitt(&data, &parms, layout.width, layout.height),
        Codec::Jbig2 => codecs::jbig2(doc, &data, &parms).map(|(_, _, bits)| bits),
        Codec::Dct | Codec::Jpx => Err("a JPEG mask, which isn't decoded".into()),
    }
}

/// A JPEG or JPEG 2000 image's mask as an image of its own, as `pdfimages` writes it.
fn passthrough_mask(doc: &Document, input: &Input<'_>) -> Result<Option<ExtractedImage>, String> {
    for (key, kind) in [("SMask", ImageKind::SoftMask), ("Mask", ImageKind::Mask)] {
        let Some((mask, layout)) = mask_stream(doc, input, key)? else {
            continue;
        };
        let data = mask_data(doc, &mask)?;
        let decode = numbers(doc, &mask.dict, "Decode");
        let pixels = if kind == ImageKind::SoftMask {
            samples::gray(layout, samples::soft_alpha(&data, layout, decode.as_deref()))
        } else {
            stencil_pixels(
                &samples::stencil_alpha(&data, layout, inverted(decode.as_deref())),
                layout,
            )
        };
        let reference = input.stream.dict.get(key).and_then(Object::as_ref);
        return Ok(Some(ExtractedImage {
            page: input.page,
            object: reference.or(input.object),
            kind,
            width: layout.width,
            height: layout.height,
            bits_per_component: layout.bits,
            color_space: "gray".into(),
            filters: codecs::filter_names(doc, &mask.dict),
            format: ImageFileFormat::Png,
            data: png::encode(&pixels)?,
        }));
    }
    Ok(None)
}
