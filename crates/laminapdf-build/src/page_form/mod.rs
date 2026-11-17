//! Pages of another document drawn as Form XObjects (ISO 32000-2 §8.10): the page's content
//! and resources copied once, by [`ObjectCopier`], so the drawing stays vector and its text
//! stays text.

mod marked;

use laminapdf_cos::{Dict, Name, ObjRef, Object, PdfString, Stream};
use laminapdf_model::{Copied, Matrix, ObjectCopier, RectPt};

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
    pub(crate) fn placement(&self, rect: &RectPt) -> Matrix {
        let scale = Matrix::new([
            rect.width().get() / self.width.max(f64::MIN_POSITIVE),
            0.0,
            0.0,
            rect.height().get() / self.height.max(f64::MIN_POSITIVE),
            rect.x0.get(),
            rect.y0.get(),
        ]);
        scale.after(&self.to_page)
    }
}

impl Builder {
    /// Copies `pages` of `source` (indices from 0) into this document, each as a Form XObject
    /// holding the page's content with its resources and its transparency group, its bounding
    /// box the page's crop box. What several of the pages share (a font, an image) is copied
    /// once. The pages' annotations aren't copied.
    ///
    /// The copy belongs to whatever it is drawn inside, such as a `Figure`, not to the source's
    /// structure, so its marked-content operators (§14.6) are taken out. Content in an optional
    /// content group that is off as the source opens (§8.11) is left out with them, so a hidden
    /// layer stays hidden. A page whose content can't be read to its end within the source's
    /// [`Limits`](laminapdf_cos::Limits) gives `None`: what follows the point reading stopped could
    /// still hold marked content.
    ///
    /// Fonts come as the source has them: under PDF/UA-1 or PDF/A, a page whose fonts aren't
    /// embedded ([`Page::fonts`](laminapdf_model::Page::fonts)) makes a document that doesn't
    /// conform, which the builder's checks don't see.
    ///
    /// # Errors
    /// `page-index-out-of-range` for a page past the end; `limit-exceeded` past the source's
    /// limits on decoded content or copied data; reading errors.
    ///
    /// Added in 0.6.0.
    pub fn page_forms(&mut self, source: &laminapdf_model::Document, pages: &[usize]) -> Result<Vec<Option<PageForm>>> {
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
            let Some(content) = marked::unmarked(source, page.resources(), &content)? else {
                out.push(None);
                continue;
            };
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
            // The page's transparency group becomes the form's (§8.10.3, §11.6.6).
            if let Some(group) = page.dict().get("Group") {
                dict.insert("Group", copier.copy_value(&mut edit, group)?);
            }
            let stream = if target.state.compress {
                Stream::flate(dict, &content)
            } else {
                Stream::new(dict, content)
            };
            let object = edit.create(Object::Stream(stream))?;
            Self::copied_layers(source, &copier, &mut target.state.layers)?;
            out.push(Some(PageForm {
                object,
                to_page: frame.to_user_space_matrix().invert().ok_or(Error::InvalidGeometry {
                    what: "a page whose frame can't be inverted",
                })?,
                width: frame.width().get(),
                height: frame.height().get(),
            }));
        }
        Ok(out)
    }

    /// Adds the source's optional content groups that `copier` has copied so far to `layers`,
    /// each once, with whether it is off as the source opens: forms and images the copies draw
    /// keep their `/OC`, and the new document's `/OCProperties` must name the groups for a
    /// viewer to hide the ones that are off.
    fn copied_layers(
        source: &laminapdf_model::Document,
        copier: &ObjectCopier<'_>,
        layers: &mut Vec<(ObjRef, bool)>,
    ) -> Result<()> {
        let catalog = source.catalog()?;
        let Some(properties) = catalog.get("OCProperties") else {
            return Ok(());
        };
        let properties = source.cos().deref(properties)?;
        let Some(groups) = properties.as_dict().and_then(|p| p.get("OCGs")) else {
            return Ok(());
        };
        let groups = source.cos().deref(groups)?;
        for group in groups.as_array().into_iter().flatten().filter_map(Object::as_ref) {
            let Some(Copied::Object(copy)) = copier.copy_of(group) else {
                continue;
            };
            if layers.iter().all(|(r, _)| *r != copy) {
                layers.push((copy, source.optional_content_off(&Object::Ref(group))?));
            }
        }
        Ok(())
    }
}

/// `/OCProperties` (§8.11.4.2) for the groups `layers` names: all of them, those that are off
/// in the default configuration's `/OFF`, and the `/Name` PDF/UA-1 asks of a configuration.
pub(crate) fn optional_content(layers: &[(ObjRef, bool)]) -> Dict {
    let refs = |off: Option<bool>| -> Object {
        Object::Array(
            layers
                .iter()
                .filter(|(_, o)| off.is_none_or(|want| *o == want))
                .map(|(r, _)| Object::Ref(*r))
                .collect(),
        )
    };
    let mut config = Dict::new();
    config.insert("Name", PdfString::literal(b"Copied pages".to_vec()));
    config.insert("OFF", refs(Some(true)));
    let mut properties = Dict::new();
    properties.insert("OCGs", refs(None));
    properties.insert("D", config);
    properties
}
