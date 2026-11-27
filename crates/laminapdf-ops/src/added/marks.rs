//! Page marks (E6 plan §2.3): watermarks, backgrounds, headers and footers, and Bates numbers,
//! which [`AddWatermark`](super::AddWatermark), [`AddBackground`](super::AddBackground),
//! [`SetHeaderFooter`](super::SetHeaderFooter) and [`ApplyBates`](super::ApplyBates) put on many
//! pages at once, [`UpdatePageMark`](super::UpdatePageMark) replaces and [`DeletePageMark`] takes
//! off, before or after a save. [`page_marks`] lists what a page holds.
//!
//! A mark is one Form XObject per page, drawn by one small stream of its own in the page's
//! `/Contents` like every added object (`place`): a form in page space that draws the mark's text
//! and pictures, which the generator draws as the content ops do. The mark's ID is the same on
//! every page it is on, so a page's marks and its added objects share one namespace. In a tagged
//! file the drawing is an artifact (§14.8.2.2) and never owns structure.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use laminapdf_content::{PageContentWriter, ResourceName};
use laminapdf_cos::{Dict, ObjRef, Object, Stream};
use laminapdf_model::{Matrix, PageId, PdfDate};
use laminapdf_syntax::format_real;
use serde::{Deserialize, Serialize};

use super::mark_layout::{self, Child, Face, Fields, Sheet};
use super::mark_types::{MarkContent, PageMark, Slot};
use super::ocg::{self, Element};
use super::place::{Mark, Placed, Placement, Private, drawing, fade, mark_form, place_form, scan};
use super::spec::{ContentId, ContentLayer};
use super::{draw, page_parts};
use crate::annot::Appearances;
use crate::blob::BlobStore;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::parent_tree::ParentTree;
use crate::wire;

/// The most pages one mark op names.
const MAX_PAGES: usize = 100_000;

/// What `invalid-op` says of a page mark whose pages, text, size, angle or margins are missing or
/// out of range.
pub(super) const INVALID: &str = "a page mark needs pages without repeats, some text (or an image), a size from 1 to 500, finite angle and margins, and at most 20 digits";

/// What the page the mark goes on looks like to the layout.
struct PageContext<'a> {
    sheet: Sheet,
    /// The page's 0-based place in the document.
    index: usize,
    pages: usize,
    label: &'a str,
    /// The page's 0-based place among the pages the op names, which Bates numbers count by.
    ordinal: usize,
}

/// What a mark draws on one page: its text and pictures, each with the matrix that places it.
fn children(
    mark: &PageMark,
    page: &PageContext<'_>,
    appearances: &dyn Appearances,
    blobs: &BlobStore,
) -> Result<Vec<Child>> {
    let sheet = page.sheet;
    Ok(match mark {
        PageMark::Watermark(w) | PageMark::Background(w) => match &w.mark {
            MarkContent::Text {
                text,
                fonts,
                size,
                color,
            } => vec![mark_layout::centred_text(
                text,
                w.angle,
                sheet,
                Face {
                    fonts,
                    size: *size,
                    color: *color,
                },
            )],
            MarkContent::Image { blob } => {
                let bytes = blobs.get(blob).ok_or(Error::UnknownBlob(*blob))?;
                let pixels = appearances.image_size(bytes).ok_or(Error::ContentObjectInvalid)?;
                let cover = matches!(mark, PageMark::Background(_));
                vec![mark_layout::centred_image(*blob, pixels, cover, w.angle, sheet)]
            }
        },
        PageMark::HeaderFooter(h) => {
            let fields = Fields {
                page: page.index + 1,
                pages: page.pages,
                label: page.label,
                date: &h.date,
                file: &h.file,
            };
            let face = Face {
                fonts: &h.fonts,
                size: h.size,
                color: h.color,
            };
            Slot::ALL
                .into_iter()
                .zip(&h.slots)
                .filter_map(|(slot, text)| {
                    let text = mark_layout::fill_fields(text.as_deref()?, &fields);
                    mark_layout::slot_child(&text, slot, h.margins, sheet, face)
                })
                .collect()
        }
        PageMark::Bates(b) => {
            let number = u64::try_from(page.ordinal)
                .ok()
                .and_then(|n| b.start.checked_add(n))
                .ok_or(Error::InvalidOp(INVALID))?;
            let text = mark_layout::bates(&b.prefix, number, b.digits, &b.suffix);
            let face = Face {
                fonts: &b.fonts,
                size: b.size,
                color: b.color,
            };
            mark_layout::slot_child(&text, b.slot, b.margins, sheet, face)
                .into_iter()
                .collect()
        }
    })
}

/// Everything a mark needs about one page, read before anything is written.
struct Target {
    page: laminapdf_cos::Page,
    sheet: Sheet,
    /// The page space to user space matrix.
    to_user: Matrix,
    index: usize,
    label: String,
    /// The page's added objects.
    placed: Vec<Placed>,
}

/// Whether a mark is new or replaces one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Add,
    Update,
}

/// What an op that puts a mark on pages needs.
pub(super) struct Put<'a> {
    pub(super) id: &'a ContentId,
    pub(super) pages: &'a [PageId],
    pub(super) mark: &'a PageMark,
    pub(super) modified: &'a PdfDate,
    pub(super) mode: Mode,
}

/// Distinct pages, at least one and not too many.
fn check_pages(pages: &[PageId]) -> Result<()> {
    let distinct: BTreeSet<&PageId> = pages.iter().collect();
    if pages.is_empty() || pages.len() > MAX_PAGES || distinct.len() != pages.len() {
        return Err(Error::InvalidOp(INVALID));
    }
    Ok(())
}

/// Reads each page's parts and its added objects.
fn targets(doc: &laminapdf_model::Document, pages: &[PageId]) -> Result<Vec<Target>> {
    let view = doc.pages()?;
    pages
        .iter()
        .map(|id| {
            let (page, frame) = page_parts(doc, id)?;
            let index = view.index_of(id).ok_or_else(|| Error::UnknownPage(id.clone()))?;
            let label = view
                .get(index)
                .map(|p| p.label())
                .transpose()?
                .flatten()
                .unwrap_or_else(|| (index + 1).to_string());
            Ok(Target {
                sheet: Sheet {
                    width: frame.width().get(),
                    height: frame.height().get(),
                },
                to_user: frame.to_user_space_matrix(),
                index,
                label,
                placed: scan(doc, &page)?,
                page,
            })
        })
        .collect()
}

/// A form over the page, in page space, that draws each of `forms` through its matrix. `group`,
/// when given, is the optional content group that switches the whole form off.
fn container(
    doc: &mut laminapdf_model::Document,
    sheet: Sheet,
    forms: &[(ObjRef, Matrix)],
    group: Option<ObjRef>,
) -> Result<ObjRef> {
    let mut content = String::new();
    let mut xobjects = Dict::new();
    for (i, (form, matrix)) in forms.iter().enumerate() {
        let name = format!("M{i}");
        let cm = matrix.to_array().map(format_real).join(" ");
        let _ = writeln!(content, "q {cm} cm /{name} Do Q");
        xobjects.insert(name.as_str(), Object::Ref(*form));
    }
    let mut resources = Dict::new();
    resources.insert("XObject", xobjects);
    let bbox = [0.0, 0.0, sheet.width, sheet.height].map(Object::number);
    let mut dict = Dict::new();
    dict.insert("Type", Object::name("XObject"));
    dict.insert("Subtype", Object::name("Form"));
    dict.insert("BBox", Object::Array(bbox.into()));
    dict.insert("Resources", resources);
    if let Some(group) = group {
        dict.insert("OC", Object::Ref(group));
    }
    Ok(doc
        .cos_mut()
        .edit()
        .create(Object::Stream(Stream::flate(dict, content.as_bytes())))?)
}

/// Checks what the op needs of each page before anything is written: no ID in use for a new mark,
/// a mark of the same kind for a replacement. Returns the optional content groups the old forms
/// name and the one a hideable replacement keeps.
fn preflight(put: &Put<'_>, targets: &[Target]) -> Result<(BTreeSet<ObjRef>, Option<ObjRef>)> {
    let mut old_groups = BTreeSet::new();
    let mut reused = None;
    for target in targets {
        let old = target.placed.iter().find(|p| &p.private.id == put.id);
        match (put.mode, old) {
            (Mode::Add, Some(_)) => return Err(Error::ContentIdInUse(put.id.clone())),
            (Mode::Add, None) => {}
            (Mode::Update, None) => return Err(Error::UnknownPageMark(put.id.clone())),
            (Mode::Update, Some(old)) => match &old.private.mark {
                None => return Err(Error::UnknownPageMark(put.id.clone())),
                Some(was) if std::mem::discriminant(was) != std::mem::discriminant(put.mark) => {
                    return Err(Error::InvalidOp("a page mark is replaced by one of its own kind"));
                }
                Some(_) => {
                    if let Some(group) = ocg::group_of(&old.form_dict) {
                        old_groups.insert(group);
                        reused.get_or_insert(group);
                    }
                }
            },
        }
    }
    Ok((old_groups, reused))
}

/// Puts `put.mark` on `put.pages`, new or in place of the one with the same ID.
pub(super) fn put(
    doc: &mut laminapdf_model::Document,
    appearances: Option<&dyn Appearances>,
    blobs: &BlobStore,
    put: &Put<'_>,
) -> Result<Effect> {
    check_pages(put.pages)?;
    if !put.mark.is_valid() {
        return Err(Error::InvalidOp(INVALID));
    }
    let generator = appearances.ok_or(Error::AppearanceUnavailable)?;
    let pages = doc.pages()?.len();
    let targets = targets(doc, put.pages)?;
    // Every page is checked before the first is written.
    let (old_groups, reused) = preflight(put, &targets)?;
    let group = match reused {
        Some(group) if put.mark.hideable() => Some(group),
        _ if put.mark.hideable() => {
            let element = if put.mark.layer() == ContentLayer::Behind {
                Element::Background
            } else {
                Element::Foreground
            };
            Some(ocg::add_group(doc, &put.mark.group_name(), element)?)
        }
        _ => None,
    };
    let artifact = if ParentTree::read(doc)?.is_some() {
        Mark::Page(put.mark.artifact())
    } else {
        Mark::None
    };
    // The forms drawn so far, so the same text on many pages is drawn, and its font embedded,
    // once.
    let mut drawn: BTreeMap<String, ObjRef> = BTreeMap::new();
    for (ordinal, target) in targets.iter().enumerate() {
        let context = PageContext {
            sheet: target.sheet,
            index: target.index,
            pages,
            label: &target.label,
            ordinal,
        };
        let mut forms = Vec::new();
        for child in children(put.mark, &context, generator, blobs)? {
            if !child.spec.is_valid() {
                return Err(Error::InvalidOp(INVALID));
            }
            let key = serde_json::to_string(&child.spec).map_err(|_| Error::InvalidOp(INVALID))?;
            let form = if let Some(form) = drawn.get(&key) {
                *form
            } else {
                let form = draw(doc, appearances, blobs, &child.spec)?;
                drawn.insert(key, form);
                form
            };
            forms.push((form, child.matrix));
        }
        let form = container(doc, target.sheet, &forms, group)?;
        let private = Private {
            id: put.id.clone(),
            kind: put.mark.kind().to_owned(),
            spec: None,
            alt: None,
            layer: put.mark.layer(),
            mark: Some(stored(put.mark, ordinal)),
        };
        mark_form(doc, form, &private, put.modified)?;
        let placement = Placement {
            cm: target.to_user,
            opacity: put.mark.opacity(),
            mark: artifact,
            layer: put.mark.layer(),
        };
        match put.mode {
            Mode::Add => {
                let behind: Vec<ObjRef> = target
                    .placed
                    .iter()
                    .filter(|p| p.private.layer == ContentLayer::Behind)
                    .map(|p| p.stream)
                    .collect();
                place_form(doc, &target.page, form, placement, &behind)?;
            }
            Mode::Update => {
                let old = target.placed.iter().find(|p| &p.private.id == put.id);
                if let Some(old) = old {
                    replace(doc, &target.page, old, form, placement)?;
                }
            }
        }
    }
    // A group no form names any more (a mark made unhideable) leaves the document's layers.
    for group in old_groups {
        release(doc, group)?;
    }
    Ok(Effect {
        pages: put.pages.to_vec(),
        ..Effect::default()
    })
}

/// What a page keeps of the mark: a Bates number's `start` becomes the page's own number.
fn stored(mark: &PageMark, ordinal: usize) -> PageMark {
    match mark {
        PageMark::Bates(b) => {
            let mut own = b.clone();
            own.start = b.start.saturating_add(u64::try_from(ordinal).unwrap_or(0));
            PageMark::Bates(own)
        }
        other => other.clone(),
    }
}

/// Draws `form` where the drawing of the mark `old` was, in place of it, and takes the old form's
/// name and the old opacity's ExtGState out of the page's resources.
fn replace(
    doc: &mut laminapdf_model::Document,
    page: &laminapdf_cos::Page,
    old: &Placed,
    form: ObjRef,
    placement: Placement,
) -> Result<()> {
    let mut writer = PageContentWriter::new(doc.cos_mut().edit(), page)?;
    let gs = fade(&mut writer, placement.opacity)?;
    let new = drawing(&mut writer, form, placement.cm, gs, placement.mark)?;
    let index = writer
        .contents()
        .iter()
        .position(|o| o.as_ref() == Some(old.stream))
        .unwrap_or(old.index);
    writer.remove(old.stream)?;
    writer.insert(index, &new.to_bytes())?;
    let mut gone = BTreeSet::new();
    if new.form != old.drawing.form {
        gone.insert(ResourceName::new("XObject", old.drawing.form.clone()));
    }
    if let Some(gs) = old.drawing.gs.as_ref().filter(|gs| new.gs.as_ref() != Some(*gs)) {
        gone.insert(ResourceName::new("ExtGState", gs.clone()));
    }
    writer.remove_resources(&gone)?;
    Ok(())
}

/// Takes `group` out of the document's layers unless a page still draws a form that names it.
fn release(doc: &mut laminapdf_model::Document, group: ObjRef) -> Result<()> {
    for page in doc.pages()?.iter() {
        for placed in scan(doc, page.cos())? {
            if ocg::group_of(&placed.form_dict) == Some(group) {
                return Ok(());
            }
        }
    }
    ocg::remove_group(doc, group)
}

/// Takes a page mark off the pages named, with its optional content group once no page draws a
/// form that names it. The pages' earlier revisions still draw it after an incremental save. A
/// page that draws no mark with the ID is `unknown-page-mark`, and nothing is deleted.
///
/// On the wire: `{ "op": "deletePageMark", "args": { "id": "draft", "pages": ["p3.0"] } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DeletePageMark {
    /// The mark.
    pub id: ContentId,
    /// The pages to take it off, without repeats.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
}

impl DeletePageMark {
    /// The op that takes the mark `id` off `pages`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(id: ContentId, pages: Vec<PageId>) -> Self {
        Self { id, pages }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        check_pages(&self.pages)?;
        let targets = targets(doc, &self.pages)?;
        let mut found = Vec::new();
        for target in &targets {
            let old = target
                .placed
                .iter()
                .find(|p| p.private.id == self.id && p.private.mark.is_some())
                .ok_or_else(|| Error::UnknownPageMark(self.id.clone()))?;
            found.push(old);
        }
        let mut groups = BTreeSet::new();
        for (target, old) in targets.iter().zip(found) {
            groups.extend(ocg::group_of(&old.form_dict));
            let mut writer = PageContentWriter::new(doc.cos_mut().edit(), &target.page)?;
            writer.remove(old.stream)?;
            let mut gone: BTreeSet<ResourceName> = [ResourceName::new("XObject", old.drawing.form.clone())].into();
            gone.extend(old.drawing.gs.clone().map(|gs| ResourceName::new("ExtGState", gs)));
            writer.remove_resources(&gone)?;
        }
        for group in groups {
            release(doc, group)?;
        }
        Ok(Effect {
            pages: self.pages.clone(),
            removed: true,
            ..Effect::default()
        })
    }
}

/// A page mark on a page, as [`page_marks`] lists it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct PageMarkInfo {
    /// Its ID.
    pub id: ContentId,
    /// What it is, as the page keeps it: a header's slots are the text with its fields, a Bates
    /// number's `start` is this page's own number.
    pub mark: PageMark,
    /// Whether a reader's layers panel can switch it off.
    pub hideable: bool,
}

/// The page marks Lamina PDF added to `page`, in the order they paint: backgrounds first. A mark
/// whose form another app changed, or that an older version stored differently, is the page's own
/// content and isn't listed. A mark has the same ID on every page it is on, so the IDs this lists
/// on each page name the sets of pages to update or delete a mark on.
///
/// # Errors
/// `unknown-page`; reading errors.
///
/// Added in 0.7.0.
pub fn page_marks(doc: &laminapdf_model::Document, page: &PageId) -> Result<Vec<PageMarkInfo>> {
    let (cos_page, _) = page_parts(doc, page)?;
    Ok(scan(doc, &cos_page)?
        .into_iter()
        .filter_map(|p| {
            let mark = p.private.mark?;
            Some(PageMarkInfo {
                id: p.private.id,
                hideable: ocg::group_of(&p.form_dict).is_some(),
                mark,
            })
        })
        .collect())
}
