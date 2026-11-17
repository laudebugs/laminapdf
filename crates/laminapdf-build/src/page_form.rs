//! Pages of another document drawn as Form XObjects (ISO 32000-2 §8.10): the page's content
//! and resources copied once, by [`ObjectCopier`], so the drawing stays vector and its text
//! stays text.

use laminapdf_cos::{Dict, Name, ObjRef, Object, Stream};
use laminapdf_model::{Matrix, ObjectCopier};

use crate::builder::Builder;
use crate::error::{Error, Result};

/// A page of another document, copied into this one as a Form XObject by
/// [`Builder::page_forms`]: draw it with [`Surface::page_form`](crate::Surface::page_form) or
/// as a figure of a flow. It is drawn the way up the page displays (its `/Rotate` applied),
/// without its annotations.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
pub struct PageForm {
    pub(crate) object: ObjRef,
    /// The source page's user space to its page space (y down, from its top-left corner).
    pub(crate) to_page: Matrix,
    width: f64,
    height: f64,
}

impl PageForm {
    /// The page's width as displayed, in points.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn width(&self) -> f64 {
        self.width
    }

    /// The page's height as displayed, in points.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn height(&self) -> f64 {
        self.height
    }

    /// The matrix that draws the form over `rect` of a page, in that page's space: the
    /// displayed page stretched to the rectangle.
    pub(crate) fn placement(&self, x: f64, y: f64, width: f64, height: f64) -> Matrix {
        let scale = Matrix::new([
            width / self.width.max(f64::MIN_POSITIVE),
            0.0,
            0.0,
            height / self.height.max(f64::MIN_POSITIVE),
            x,
            y,
        ]);
        scale.after(&self.to_page)
    }
}

impl Builder {
    /// Copies `pages` of `source` (indices from 0) into this document, each as a Form XObject
    /// holding the page's content with its resources, its bounding box the page's crop box.
    /// What several of the pages share (a font, an image) is copied once. The pages'
    /// annotations aren't copied.
    ///
    /// Fonts come as the source has them: under PDF/UA-1 or PDF/A, a page whose fonts aren't
    /// embedded ([`Page::fonts`](laminapdf_model::Page::fonts)) makes a document that doesn't
    /// conform, which the builder's checks don't see.
    ///
    /// # Errors
    /// `page-index-out-of-range` for a page past the end; `limit-exceeded` past the source's
    /// [`Limits`](laminapdf_cos::Limits) on decoded content or copied data; reading errors.
    ///
    /// Added in 0.6.0.
    pub fn page_forms(&mut self, source: &laminapdf_model::Document, pages: &[usize]) -> Result<Vec<PageForm>> {
        let all = source.pages()?;
        let count = all.len();
        let target = self.target();
        let mut copier = ObjectCopier::new(source.cos(), *target.doc.limits());
        let mut out = Vec::with_capacity(pages.len());
        for &index in pages {
            let page = all.get(index).ok_or(Error::PageIndexOutOfRange {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                count,
            })?;
            let frame = page.frame()?;
            let content = source.cos().page_content(page.cos())?;
            let mut edit = target.doc.edit();
            let resources = match page.resources() {
                Some(resources) => copier.copy_value(&mut edit, &Object::Dict(resources.clone()))?,
                None => Object::Dict(Dict::new()),
            };
            let crop = frame.crop_box();
            let mut dict = Dict::new();
            dict.insert("Type", Name::from("XObject"));
            dict.insert("Subtype", Name::from("Form"));
            dict.insert(
                "BBox",
                Object::Array(vec![crop.x0.into(), crop.y0.into(), crop.x1.into(), crop.y1.into()]),
            );
            dict.insert("Resources", resources);
            let stream = if target.state.compress {
                Stream::flate(dict, &content)
            } else {
                Stream::new(dict, content)
            };
            let object = edit.create(Object::Stream(stream))?;
            out.push(PageForm {
                object,
                to_page: frame.to_user_space_matrix().invert().ok_or(Error::InvalidGeometry {
                    what: "a page whose frame can't be inverted",
                })?,
                width: frame.width().get(),
                height: frame.height().get(),
            });
        }
        Ok(out)
    }
}
