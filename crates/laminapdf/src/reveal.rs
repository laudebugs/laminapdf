//! What a file holds beyond its visible pages, for `laminapdf reveal`: its revisions and what
//! each changed, its metadata, attachments, scripts and actions, annotations, form values,
//! layers that are off, and hidden text. Each [`Category`] names a kind of finding that a
//! sanitiser can remove.

use std::collections::BTreeSet;

use laminapdf_cos::OpenOptions;
use laminapdf_syntax::Object;
use laminapdf_syntax::text::decode_text_string;

pub use laminapdf_model::{
    Action, ActionKind, Annotation, Disclosure, EmbeddedFile, EmbeddedFileHolder, FormField, Owner, Trigger, XmpPacket,
};

use crate::extract::{DropReason, Dropped, ExtractOptions};
use crate::job::CancelToken;
use crate::redaction::earlier_revisions;
use crate::{Document, Error, Result};

/// A kind of finding, in the order a report lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Category {
    /// Earlier revisions, and objects nothing references.
    History,
    /// The Info dictionary, XMP and object-level metadata.
    Metadata,
    /// Embedded files.
    Attachments,
    /// Scripts, actions a trigger fires, and XFA.
    ActiveContent,
    /// Annotations other than links, widgets and popups.
    Annotations,
    /// Form fields with values.
    Forms,
    /// Hidden text, layers that are off, and unapplied redactions.
    HiddenContent,
}

impl Category {
    /// Every category, in report order.
    pub const ALL: [Self; 7] = [
        Self::History,
        Self::Metadata,
        Self::Attachments,
        Self::ActiveContent,
        Self::Annotations,
        Self::Forms,
        Self::HiddenContent,
    ];

    /// The kebab-case name (`history`, `active-content`, …).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::History => "history",
            Self::Metadata => "metadata",
            Self::Attachments => "attachments",
            Self::ActiveContent => "active-content",
            Self::Annotations => "annotations",
            Self::Forms => "forms",
            Self::HiddenContent => "hidden-content",
        }
    }
}

/// One revision of the file, oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RevisionRecord {
    /// Its length in bytes: the prefix of the file that holds it.
    pub bytes: u64,
    /// Objects it adds (for the first revision, every object).
    pub added: Vec<u32>,
    /// Objects an earlier revision had that it rewrites.
    pub changed: Vec<u32>,
    /// Objects it frees.
    pub freed: Vec<u32>,
    /// The latest revision's pages (0-based) whose dictionary or content streams it rewrites.
    pub pages: Vec<u32>,
    /// Words this revision holds more often than the latest; 0 for the latest.
    pub removed_words: u64,
    /// This revision's pages (0-based) where they are.
    pub removed_on: Vec<u32>,
}

/// What [`Document::reveal`] finds.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Reveal {
    /// The revisions, oldest first; empty for a repaired file.
    pub revisions: Vec<RevisionRecord>,
    /// Whether the file is linearized: its first revision has two cross-reference sections.
    pub linearized: bool,
    /// Whether its cross-reference table had to be rebuilt.
    pub repaired: bool,
    /// Every Info entry, as key and value (a text string decoded, anything else as written).
    pub info: Vec<(String, String)>,
    /// The trailer's `/ID` strings, in hexadecimal.
    pub id: Vec<String>,
    /// What one scan of every object found.
    pub disclosure: Disclosure,
    /// Hidden text, one entry per run with its text ([ADR 0030] §4's six flags; artifacts
    /// aren't hidden). A report counts them and shows their text only on request.
    ///
    /// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
    pub hidden: Vec<Dropped>,
}

impl Reveal {
    /// The categories with findings, in report order.
    #[must_use]
    pub fn found(&self) -> Vec<Category> {
        Category::ALL.into_iter().filter(|c| self.has(*c)).collect()
    }

    /// Whether `category` has a finding.
    #[must_use]
    pub fn has(&self, category: Category) -> bool {
        let d = &self.disclosure;
        match category {
            Category::History => self.revisions.len() > 1 || !d.unreferenced.is_empty(),
            Category::Metadata => {
                !self.info.is_empty()
                    || d.xmp.is_some()
                    || !d.metadata_streams.is_empty()
                    || !d.piece_info.is_empty()
                    || !d.thumbnails.is_empty()
            }
            Category::Attachments => !d.attachments.is_empty(),
            Category::ActiveContent => d.xfa || d.actions.iter().any(counts),
            Category::Annotations => d.annotations.iter().any(|a| !a.is_plain()),
            Category::Forms => d.fields.iter().any(|f| f.value.is_some()),
            Category::HiddenContent => {
                !self.hidden.is_empty() || !d.layers_off.is_empty() || !d.redactions().is_empty()
            }
        }
    }
}

/// Whether an action counts as active content: a script, one that launches, submits or
/// imports, or one a trigger fires on its own. Links a reader follows don't count.
fn counts(action: &Action) -> bool {
    matches!(
        action.kind,
        ActionKind::JavaScript | ActionKind::Launch | ActionKind::SubmitForm | ActionKind::ImportData
    ) || matches!(
        action.trigger,
        Trigger::OpenAction | Trigger::Additional(_) | Trigger::NamedScript(_)
    )
}

/// `bytes` in lowercase hexadecimal.
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|b| [b >> 4, b & 15])
        .filter_map(|d| char::from_digit(u32::from(d), 16))
        .collect()
}

/// The reasons that hide text from a reader ([ADR 0030] §4's six flags).
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
const HIDDEN: [DropReason; 6] = [
    DropReason::Invisible,
    DropReason::OffPage,
    DropReason::Tiny,
    DropReason::Clipped,
    DropReason::LowContrast,
    DropReason::Covered,
];

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl Document {
    /// What the file holds beyond its visible pages ([`Reveal`]). Earlier revisions open with
    /// `options` (its password and limits), never strictly.
    ///
    /// # Errors
    /// `cancelled` (checked between categories and revisions); the reading errors.
    pub fn reveal(&self, options: &OpenOptions, cancel: &CancelToken) -> Result<Reveal> {
        let check = || {
            if cancel.is_cancelled() {
                Err(Error::Cancelled)
            } else {
                Ok(())
            }
        };
        let model = self.ops();
        let disclosure = model.disclosure()?;
        check()?;
        let mut revisions = self.revision_records()?;
        for (revision, words, pages) in earlier_revisions(self, options, cancel)? {
            if let Some(r) = usize::try_from(revision).ok().and_then(|i| revisions.get_mut(i)) {
                r.removed_words = words;
                r.removed_on = pages;
            }
        }
        check()?;
        let sections = model.xref_section_offsets().len();
        let mut options = ExtractOptions::default();
        options.drop.keep_spans = true;
        let mut hidden = laminapdf_extract::hidden_text(model, &options, cancel)?;
        // Artifacts are furniture a tagged file marks, not text hidden from its reader.
        hidden.retain(|d| HIDDEN.contains(&d.reason));
        Ok(Reveal {
            linearized: !model.was_repaired() && revisions.len() == 1 && sections > 1,
            repaired: model.was_repaired(),
            revisions,
            info: self.info_entries()?,
            id: self.file_id(),
            disclosure,
            hidden,
        })
    }

    /// Each revision's objects, split into added and changed, and the pages it rewrites.
    fn revision_records(&self) -> Result<Vec<RevisionRecord>> {
        let model = self.ops();
        let index = model.revision_index()?;
        // Each page's own objects: its dictionary and content streams.
        let mut page_objects: Vec<BTreeSet<u32>> = Vec::new();
        for page in model.pages()?.iter() {
            let mut objects: BTreeSet<u32> = page.contents()?.iter().map(|r| r.num).collect();
            objects.extend(page.cos().obj_ref.map(|r| r.num));
            page_objects.push(objects);
        }
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        let mut out = Vec::new();
        for revision in index {
            let (changed, added): (Vec<u32>, Vec<u32>) = revision.in_use.iter().partition(|n| seen.contains(n));
            let touched: BTreeSet<u32> = changed.iter().chain(&revision.freed).copied().collect();
            let pages = if out.is_empty() {
                Vec::new()
            } else {
                page_objects
                    .iter()
                    .enumerate()
                    .filter(|(_, objects)| !objects.is_disjoint(&touched))
                    .map(|(i, _)| to_u32(i))
                    .collect()
            };
            seen.extend(&revision.in_use);
            out.push(RevisionRecord {
                bytes: revision.end,
                added,
                changed,
                freed: revision.freed,
                pages,
                removed_words: 0,
                removed_on: Vec::new(),
            });
        }
        Ok(out)
    }

    /// Every Info entry, key and value.
    fn info_entries(&self) -> Result<Vec<(String, String)>> {
        let doc = self.ops();
        let Some(info) = doc.trailer().get("Info") else {
            return Ok(Vec::new());
        };
        let Some(info) = laminapdf_cos::tolerant(doc.deref(info))? else {
            return Ok(Vec::new());
        };
        let Some(dict) = info.as_dict() else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for (key, value) in dict.iter() {
            let value = laminapdf_cos::tolerant(doc.deref(value))?;
            let text = match value.as_deref() {
                Some(Object::String(s)) => decode_text_string(&s.bytes),
                Some(Object::Name(n)) => String::from_utf8_lossy(n.as_bytes()).into_owned(),
                Some(other) => String::from_utf8_lossy(&other.to_bytes()).into_owned(),
                None => continue,
            };
            out.push((String::from_utf8_lossy(key.as_bytes()).into_owned(), text));
        }
        Ok(out)
    }

    /// The trailer's `/ID` strings in hexadecimal.
    fn file_id(&self) -> Vec<String> {
        let Some(Object::Array(items)) = self.ops().trailer().get("ID") else {
            return Vec::new();
        };
        items
            .iter()
            .filter_map(|i| match i {
                Object::String(s) => Some(hex(&s.bytes)),
                _ => None,
            })
            .collect()
    }
}
