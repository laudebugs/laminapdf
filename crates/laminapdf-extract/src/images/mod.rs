//! The images a document draws, extracted as files (operations `B.03` and `H.07`): JPEG and
//! JPEG 2000 images as they are stored, byte for byte, and every other image decoded to a PNG
//! with its `/Decode`, its colour space and its mask applied.
//!
//! Each page's content is walked with its Form XObjects and inline images. An image XObject is
//! extracted once, on the first page that draws it; each inline image is its own. A mask on a
//! JPEG or JPEG 2000 image, which those files can't hold, is extracted as an image of its own,
//! as `pdfimages` does.

mod codecs;
mod color;
mod one;
mod png;
mod samples;

use std::collections::BTreeSet;

use laminapdf_content::{ImageDraw, ImageSource, Visitor, interpret_page};
use laminapdf_model::Document;
use laminapdf_syntax::{Dict, Name, ObjRef, Object, Stream};

use crate::error::ExtractError;

/// Which pages [`extract_images`] reads.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImageOptions {
    /// The pages (0-based) in order; `None` for every page. A page asked for twice is read
    /// once.
    pub pages: Option<Vec<usize>>,
}

impl ImageOptions {
    /// Reads only `pages`.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub fn with_pages(mut self, pages: Vec<usize>) -> Self {
        self.pages = Some(pages);
        self
    }
}

/// What an extracted image is.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageKind {
    /// An image.
    Image,
    /// A stencil mask (`/ImageMask true`): its painted pixels black, the rest white.
    Stencil,
    /// The soft mask (`/SMask`) of a JPEG or JPEG 2000 image, as grey opacity.
    SoftMask,
    /// The stencil mask (`/Mask`) of a JPEG or JPEG 2000 image: its masked pixels white.
    Mask,
}

/// The file an image is written as.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageFileFormat {
    /// A JPEG (`DCTDecode` data), as stored.
    Jpeg,
    /// A JPEG 2000 codestream or file (`JPXDecode` data), as stored.
    Jpeg2000,
    /// A PNG, decoded from the samples.
    Png,
}

impl ImageFileFormat {
    /// The file extension: `jpg`, `jp2` or `png`.
    ///
    /// Added in 0.4.0.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Jpeg2000 => "jp2",
            Self::Png => "png",
        }
    }
}

/// One extracted image.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExtractedImage {
    /// The first page (0-based) that draws it.
    pub page: usize,
    /// Its object; `None` for an inline image.
    pub object: Option<ObjRef>,
    /// What it is.
    pub kind: ImageKind,
    /// Its size in samples.
    pub width: u32,
    /// Its size in samples.
    pub height: u32,
    /// Its bits per component as stored.
    pub bits_per_component: u8,
    /// Its colour space's family as `pdfimages -list` names it (`rgb`, `gray`, `cmyk`,
    /// `index`, `icc` spaces by their components, `lab`, `sep`), or `-` for a stencil.
    pub color_space: String,
    /// Its filters, by their full names, in order.
    pub filters: Vec<String>,
    /// The file it is written as.
    pub format: ImageFileFormat,
    /// The file's bytes.
    pub data: Vec<u8>,
}

/// An image that couldn't be extracted, and why.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SkippedImage {
    /// The page (0-based) that draws it.
    pub page: usize,
    /// Its object; `None` for an inline image.
    pub object: Option<ObjRef>,
    /// Why, in English (`an unsupported colour space DeviceN`); never document content.
    pub reason: String,
}

/// The images of a document.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExtractedImages {
    /// The images, page by page in the order each page draws them.
    pub images: Vec<ExtractedImage>,
    /// The images that couldn't be extracted.
    pub skipped: Vec<SkippedImage>,
}

/// The images `doc`'s pages draw (see the [module docs](self)).
///
/// # Errors
/// [`ExtractError::PageIndexOutOfRange`] for a page past the last; [`ExtractError::Document`]
/// when a page's content can't be read. An image that can't be decoded doesn't stop the rest:
/// it is listed in [`ExtractedImages::skipped`].
///
/// Added in 0.4.0.
pub fn extract_images(doc: &Document, options: &ImageOptions) -> Result<ExtractedImages, ExtractError> {
    let pages = doc.pages()?;
    let count = pages.len();
    let mut todo = options.pages.clone().unwrap_or_else(|| (0..count).collect());
    let mut asked = BTreeSet::new();
    todo.retain(|&i| asked.insert(i));
    if let Some(&index) = todo.iter().find(|&&i| i >= count) {
        return Err(ExtractError::PageIndexOutOfRange { index, count });
    }
    let mut walker = Walker {
        doc,
        page: 0,
        seen: BTreeSet::new(),
        out: ExtractedImages::default(),
    };
    for index in todo {
        let Some(page) = pages.get(index) else {
            continue;
        };
        walker.page = index;
        interpret_page(doc, &page, &mut walker)?;
    }
    Ok(walker.out)
}

/// Collects each page's images as the content walk reports them.
struct Walker<'d> {
    doc: &'d Document,
    page: usize,
    /// The image XObjects extracted already.
    seen: BTreeSet<ObjRef>,
    out: ExtractedImages,
}

impl Visitor for Walker<'_> {
    fn image(&mut self, image: &ImageDraw<'_>) {
        let (stream, object) = match image.source {
            ImageSource::XObject { object, stream, .. } => {
                if let Some(r) = object
                    && !self.seen.insert(r)
                {
                    return;
                }
                let Object::Stream(stream) = stream else {
                    return;
                };
                (std::borrow::Cow::Borrowed(stream), object)
            }
            ImageSource::Inline(inline) => (
                std::borrow::Cow::Owned(Stream::new(inline_dict(&inline.dict), inline.data.to_vec())),
                None,
            ),
            _ => return,
        };
        let input = one::Input {
            stream: &stream,
            object,
            page: self.page,
            resources: image.resources,
        };
        match one::extract(self.doc, &input) {
            Ok(images) => self.out.images.extend(images),
            Err(reason) => self.out.skipped.push(SkippedImage {
                page: self.page,
                object,
                reason,
            }),
        }
    }
}

/// An inline image's dictionary with its keys and device colour-space names written out
/// (§8.9.7, Tables 91 and 92), as an image XObject's would be.
fn inline_dict(dict: &Dict) -> Dict {
    let space = |o: &Object| -> Object {
        match o {
            Object::Name(n) => match n.as_bytes() {
                b"G" => Object::name("DeviceGray"),
                b"RGB" => Object::name("DeviceRGB"),
                b"CMYK" => Object::name("DeviceCMYK"),
                _ => o.clone(),
            },
            _ => o.clone(),
        }
    };
    let mut out = Dict::new();
    for (k, v) in dict.iter() {
        let full = Name::new(full_key(k.as_bytes()).to_vec());
        let value = if full.as_bytes() == b"ColorSpace" {
            space(v)
        } else {
            v.clone()
        };
        out.insert(full, value);
    }
    out
}

/// An inline image's abbreviated key, written out (§8.9.7, Table 91).
fn full_key(k: &[u8]) -> &[u8] {
    match k {
        b"BPC" => b"BitsPerComponent",
        b"CS" => b"ColorSpace",
        b"D" => b"Decode",
        b"DP" => b"DecodeParms",
        b"F" => b"Filter",
        b"H" => b"Height",
        b"IM" => b"ImageMask",
        b"I" => b"Interpolate",
        b"W" => b"Width",
        b"L" => b"Length",
        other => other,
    }
}
