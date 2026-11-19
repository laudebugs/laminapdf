//! An exchange file applied to a document as one transaction of ordinary ops (E5 plan §10): an
//! import is one undo step, and the op log replays it like any other edit. Merge updates the
//! comments it finds by `/NM` on the same page and adds the rest; Replace first deletes every
//! comment on the pages the file names; Add makes everything new.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::Object;
use laminapdf_model::{AnnotId, AnnotSubtype, PageId, UserSpaceRect};
use laminapdf_ops::{
    AddAnnot, AddReply, AnnotCommon, AnnotSpec, Applied, BlobId, DeleteAnnot, FileToAttach, Op, Origin, PopupSpec,
    ReviewState, SetReviewState, Tx, UpdateAnnot,
};

use crate::Result;
use crate::exchange::{Comment, Exchange, SkipReason, Skipped, text};

/// What an import does with the comments already in the document (C06).
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImportMode {
    /// Keeps them: a comment with the same `/NM` on the same page takes the file's text, and its
    /// new replies and statuses are added; the rest are added.
    #[default]
    Merge,
    /// Deletes every comment on the pages the file names, then adds the file's.
    Replace,
    /// Adds every comment as a copy; one whose `/NM` is taken gets a new one.
    Add,
}

/// How to import.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportOptions {
    /// What to do with the comments already there.
    pub mode: ImportMode,
    /// The document's file name, which the exchange file's (`<f href>`, `/F`) is checked against
    /// when it carries no `/ID`.
    pub file: Option<String>,
}

impl ImportOptions {
    /// Options for `mode`, with no file name.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(mode: ImportMode) -> Self {
        Self { mode, file: None }
    }
}

/// Something an import should say before or after it runs.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportWarning {
    /// `interchange-skipped`: a comment or field the import leaves out.
    Skipped(Skipped),
    /// `interchange-document-mismatch`: the file was made for another PDF; its comments may land
    /// in the wrong places.
    DocumentMismatch {
        /// The PDF the file names, when it names one.
        file: Option<String>,
    },
}

impl ImportWarning {
    /// The stable code: `interchange-skipped` or `interchange-document-mismatch`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Skipped(_) => "interchange-skipped",
            Self::DocumentMismatch { .. } => "interchange-document-mismatch",
        }
    }
}

/// What an import does, or did.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportSummary {
    /// The comments added (replies and statuses included).
    pub added: usize,
    /// The comments found by `/NM` and kept, in Merge.
    pub matched: usize,
    /// The comments deleted first, in Replace.
    pub deleted: usize,
    /// The pages the added comments are on, counted from 0, in order.
    pub pages: Vec<u32>,
    /// What to say about it.
    pub warnings: Vec<ImportWarning>,
}

/// What importing `exchange` into `doc` would do, without changing it: for C06's summary.
///
/// # Errors
/// Reading the document.
///
/// Added in 0.7.0.
pub fn preview(doc: &laminapdf_model::Document, exchange: &Exchange, options: &ImportOptions) -> Result<ImportSummary> {
    let pages = page_ids(doc)?;
    let existing = names_by_page(doc, &pages)?;
    let mut summary = ImportSummary {
        warnings: first_warnings(doc, exchange, options),
        ..ImportSummary::default()
    };
    let (order, skipped) = in_order(exchange, pages.len(), &existing, options.mode);
    summary.warnings.extend(skipped.into_iter().map(ImportWarning::Skipped));
    for c in order {
        let found = options.mode == ImportMode::Merge && is_present(&existing, c);
        if found {
            summary.matched += 1;
        } else {
            summary.added += 1;
            summary.pages.push(c.page);
        }
    }
    summary.pages.sort_unstable();
    summary.pages.dedup();
    if options.mode == ImportMode::Replace {
        summary.deleted = deletable(doc, &pages, &named_pages(exchange))?.len();
    }
    Ok(summary)
}

/// Imports `exchange` into `doc` as one transaction (one undo step). Each comment's appearance is
/// drawn by the document's generator, a Popup kept where the file places one.
///
/// # Errors
/// The ops' errors (`appearance-unavailable` with no generator, a refused edit on a signed
/// file); nothing is changed then. Reading the document.
///
/// Added in 0.7.0.
pub fn import(
    doc: &mut laminapdf_ops::Document,
    exchange: &Exchange,
    options: &ImportOptions,
) -> Result<(ImportSummary, Applied)> {
    let pages = page_ids(doc)?;
    let existing = names_by_page(doc, &pages)?;
    let mut summary = ImportSummary {
        warnings: first_warnings(doc, exchange, options),
        ..ImportSummary::default()
    };
    let (order, skipped) = in_order(exchange, pages.len(), &existing, options.mode);
    summary.warnings.extend(skipped.into_iter().map(ImportWarning::Skipped));
    let doomed = if options.mode == ImportMode::Replace {
        deletable(doc, &pages, &named_pages(exchange))?
    } else {
        Vec::new()
    };
    // A file's bytes go in the blob store first: a transaction reads blobs, it doesn't add them.
    let blobs: Vec<Option<BlobId>> = order
        .iter()
        .map(|c| c.file.as_ref().map(|f| doc.add_blob(f.bytes.clone())))
        .collect();
    let mode = options.mode;
    let (counts, applied) = doc.transact(Origin::Local, |tx| {
        let mut run = Run {
            tx,
            pages: &pages,
            existing,
            created: BTreeMap::new(),
            counts: Counts::default(),
            mode,
        };
        run.delete(&doomed)?;
        for (c, blob) in order.iter().zip(&blobs) {
            run.comment(c, *blob)?;
        }
        Ok(run.counts)
    })?;
    summary.added = counts.added;
    summary.matched = counts.matched;
    summary.deleted = counts.deleted;
    summary.pages = counts.pages.into_iter().collect();
    Ok((summary, applied))
}

/// Every page's ID, in order.
fn page_ids(doc: &laminapdf_model::Document) -> Result<Vec<PageId>> {
    Ok(doc.pages()?.iter().map(|p| p.id().clone()).collect())
}

/// Each page's annotations by `/NM`, and by their ID for one without, as an export names them,
/// so a file exported from this document finds its comments again.
fn names_by_page(doc: &laminapdf_model::Document, pages: &[PageId]) -> Result<Vec<BTreeMap<String, AnnotId>>> {
    let all = doc.pages()?;
    let mut out = Vec::with_capacity(pages.len());
    for page in all.iter() {
        let names = doc
            .annotations(&page)?
            .into_iter()
            .map(|a| (a.name.clone().unwrap_or_else(|| a.id.to_string()), a.id))
            .collect();
        out.push(names);
    }
    Ok(out)
}

/// Whether a comment with `c`'s `/NM` is on its page already.
fn is_present(existing: &[BTreeMap<String, AnnotId>], c: &Comment) -> bool {
    let page = usize::try_from(c.page).unwrap_or(usize::MAX);
    c.name()
        .is_some_and(|n| existing.get(page).is_some_and(|names| names.contains_key(&n)))
}

/// The pages the file has comments for.
fn named_pages(exchange: &Exchange) -> BTreeSet<u32> {
    exchange.comments.iter().map(|c| c.page).collect()
}

/// The comments Replace deletes: every comment on the pages the file names, broken replies
/// included. A deletion takes the replies, statuses, group and Popup with it, which
/// [`Run::delete`] then skips.
fn deletable(doc: &laminapdf_model::Document, pages: &[PageId], named: &BTreeSet<u32>) -> Result<Vec<AnnotId>> {
    let all = doc.pages()?;
    let mut out = Vec::new();
    for (index, page) in all.iter().enumerate() {
        if !u32::try_from(index).is_ok_and(|i| named.contains(&i)) || pages.get(index).is_none() {
            continue;
        }
        out.extend(
            doc.annotations(&page)?
                .into_iter()
                .filter(|a| is_comment(&a.subtype))
                .map(|a| a.id),
        );
    }
    Ok(out)
}

/// The kinds an import treats as comments.
fn is_comment(subtype: &AnnotSubtype) -> bool {
    crate::map::IMPORTED.iter().any(|s| s.as_bytes() == subtype.name())
}

/// The mismatch warning, and the file's own skipped entries.
fn first_warnings(doc: &laminapdf_model::Document, exchange: &Exchange, options: &ImportOptions) -> Vec<ImportWarning> {
    let mut out: Vec<ImportWarning> = exchange.skipped.iter().cloned().map(ImportWarning::Skipped).collect();
    if mismatched(doc, exchange, options) {
        out.insert(
            0,
            ImportWarning::DocumentMismatch {
                file: exchange.file.clone(),
            },
        );
    }
    out
}

/// Whether the file names another PDF: by the permanent `/ID` when both have one, else by the
/// file's name.
fn mismatched(doc: &laminapdf_model::Document, exchange: &Exchange, options: &ImportOptions) -> bool {
    let ours = match doc.cos().trailer().get("ID").and_then(Object::as_array) {
        Some([Object::String(a), ..]) => Some(a.bytes.clone()),
        _ => None,
    };
    match (&exchange.ids, ours) {
        (Some([theirs, _]), Some(ours)) => *theirs != ours,
        _ => match (&exchange.file, &options.file) {
            (Some(theirs), Some(ours)) => !base_name(theirs).eq_ignore_ascii_case(base_name(ours)),
            _ => false,
        },
    }
}

/// A path's last part.
fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The comments to import, each after the one it answers, and what is skipped: a page the
/// document doesn't have, a parent neither the file nor the document has.
fn in_order<'e>(
    exchange: &'e Exchange,
    page_count: usize,
    existing: &[BTreeMap<String, AnnotId>],
    mode: ImportMode,
) -> (Vec<&'e Comment>, Vec<Skipped>) {
    let mut skipped = Vec::new();
    let skip = |c: &Comment, reason| Skipped {
        page: Some(c.page),
        kind: c.subtype().to_owned(),
        reason,
        line: c.line,
    };
    let mut waiting: Vec<&Comment> = Vec::new();
    for c in &exchange.comments {
        if usize::try_from(c.page).map_or(true, |p| p >= page_count) {
            skipped.push(skip(c, SkipReason::MissingPage));
        } else if c.in_reply_to.is_none() && AnnotSpec::from_entries(&c.entries).is_none() {
            skipped.push(skip(c, SkipReason::Unsupported));
        } else {
            waiting.push(c);
        }
    }
    // The document's names count as placed in Merge, where a reply may answer a comment already
    // there.
    let mut placed: BTreeSet<String> = if mode == ImportMode::Merge {
        existing.iter().flat_map(|names| names.keys().cloned()).collect()
    } else {
        BTreeSet::new()
    };
    let mut order = Vec::new();
    loop {
        let before = order.len();
        waiting.retain(|c| {
            let ready = c.in_reply_to.as_ref().is_none_or(|p| placed.contains(p));
            if ready {
                placed.extend(c.name());
                order.push(*c);
            }
            !ready
        });
        if order.len() == before {
            break;
        }
    }
    skipped.extend(waiting.into_iter().map(|c| skip(c, SkipReason::MissingParent)));
    (order, skipped)
}

/// What the transaction did.
#[derive(Default)]
struct Counts {
    added: usize,
    matched: usize,
    deleted: usize,
    pages: BTreeSet<u32>,
}

/// One import's transaction.
struct Run<'t, 'd> {
    tx: &'t mut Tx<'d>,
    pages: &'t [PageId],
    /// Each page's annotations by `/NM` before the import.
    existing: Vec<BTreeMap<String, AnnotId>>,
    /// The annotations this import made, by the `/NM` the file gave them.
    created: BTreeMap<String, AnnotId>,
    counts: Counts,
    mode: ImportMode,
}

impl Run<'_, '_> {
    fn apply(&mut self, op: impl Into<Op>) -> std::result::Result<Applied, laminapdf_ops::Error> {
        self.tx.apply(op)
    }

    /// Deletes `doomed`, skipping any a deletion before took with it.
    fn delete(&mut self, doomed: &[AnnotId]) -> std::result::Result<(), laminapdf_ops::Error> {
        let mut gone: BTreeSet<AnnotId> = BTreeSet::new();
        for id in doomed {
            if gone.contains(id) {
                continue;
            }
            let applied = self.apply(DeleteAnnot::new(id.clone()))?;
            gone.insert(id.clone());
            gone.extend(applied.also_deleted.iter().cloned());
            self.counts.deleted += 1;
        }
        Ok(())
    }

    /// The annotation `name` refers to: one this import made, else, in Merge, one on the page.
    fn find(&self, name: &str) -> Option<AnnotId> {
        self.created.get(name).cloned().or_else(|| {
            (self.mode == ImportMode::Merge)
                .then(|| self.existing.iter().find_map(|names| names.get(name).cloned()))
                .flatten()
        })
    }

    fn comment(&mut self, c: &Comment, blob: Option<BlobId>) -> std::result::Result<(), laminapdf_ops::Error> {
        let page = usize::try_from(c.page).unwrap_or(usize::MAX);
        let name = c.name();
        if self.mode == ImportMode::Merge
            && let Some(id) = name.as_ref().and_then(|n| self.existing.get(page)?.get(n).cloned())
        {
            self.merge(c, &id)?;
            self.counts.matched += 1;
            if let Some(n) = name {
                self.created.insert(n, id);
            }
            return Ok(());
        }
        let parent = c.in_reply_to.as_deref().and_then(|p| self.find(p));
        let nm = self.fresh_name(page, name.clone());
        let applied = match (parent, reply_kind(c)) {
            (Some(target), Reply::State(state)) => {
                let common = AnnotCommon::from_entries(&c.entries);
                let mut op = SetReviewState::new(target, state);
                op.author = common.author;
                op.created = common.created;
                op.nm = nm;
                self.apply(op)?
            }
            (Some(target), Reply::Answer) => {
                let common = AnnotCommon::from_entries(&c.entries);
                let mut op = AddReply::new(target, common.contents.unwrap_or_default());
                op.author = common.author;
                op.created = common.created;
                op.nm = nm;
                self.apply(op)?
            }
            (group, Reply::Group | Reply::None) => {
                let Some(page_id) = self.pages.get(page).cloned() else {
                    return Ok(());
                };
                let Some(op) = add_op(c, page_id, group, nm, blob) else {
                    return Ok(());
                };
                self.apply(op)?
            }
            (None, Reply::State(_) | Reply::Answer) => return Ok(()),
        };
        if let (Some(n), Some(id)) = (name, applied.created_annotations.first()) {
            self.created.insert(n, id.clone());
        }
        self.counts.added += 1;
        self.counts.pages.insert(c.page);
        Ok(())
    }

    /// Merge into a comment already there: its text, author and subject from the file.
    fn merge(&mut self, c: &Comment, id: &AnnotId) -> std::result::Result<(), laminapdf_ops::Error> {
        let theirs = AnnotCommon::from_entries(&c.entries);
        let mut op = UpdateAnnot::new(id.clone());
        // A text box's contents are its text, which its spec carries; leave those.
        if c.subtype() != "FreeText" {
            op.contents = theirs.contents.map(Some);
        }
        op.author = theirs.author.map(Some);
        op.subject = theirs.subject.map(Some);
        op.modified = theirs.modified;
        if op == UpdateAnnot::new(id.clone()) {
            return Ok(());
        }
        self.apply(op)?;
        Ok(())
    }

    /// `name`, or in Add mode, when it is taken on the page, the first of `name-2`, `name-3`, …
    /// that isn't.
    fn fresh_name(&self, page: usize, name: Option<String>) -> Option<String> {
        let name = name?;
        if self.mode != ImportMode::Add {
            return Some(name);
        }
        let taken = |n: &str| {
            self.existing.get(page).is_some_and(|names| names.contains_key(n)) || self.created.contains_key(n)
        };
        if !taken(&name) {
            return Some(name);
        }
        // One more candidate than names taken: one of them is free.
        let bound = self.existing.get(page).map_or(0, BTreeMap::len) + self.created.len() + 2;
        (2..=bound).map(|i| format!("{name}-{i}")).find(|n| !taken(n))
    }
}

/// How a comment relates to the one it names in `/IRT`.
enum Reply {
    /// It answers nothing.
    None,
    /// A reply (`/RT /R`, the default).
    Answer,
    /// A review status: a reply with `/State` and `/StateModel`.
    State(ReviewState),
    /// Grouped with it (`/RT /Group`).
    Group,
}

fn reply_kind(c: &Comment) -> Reply {
    if c.in_reply_to.is_none() {
        return Reply::None;
    }
    if c.entries.get_name("RT") == Some(b"Group".as_slice()) {
        return Reply::Group;
    }
    if let (Some(model), Some(state)) = (text(&c.entries, "StateModel"), text(&c.entries, "State")) {
        return review_state(&model, &state).map_or(Reply::None, Reply::State);
    }
    if c.subtype() == "Text" {
        Reply::Answer
    } else {
        Reply::None
    }
}

/// The review state `model` and `state` name.
fn review_state(model: &str, state: &str) -> Option<ReviewState> {
    [
        ReviewState::None,
        ReviewState::Accepted,
        ReviewState::Rejected,
        ReviewState::Cancelled,
        ReviewState::Completed,
        ReviewState::Marked,
        ReviewState::Unmarked,
    ]
    .into_iter()
    .find(|s| s.model().eq_ignore_ascii_case(model) && s.name().eq_ignore_ascii_case(state))
}

/// The `AddAnnot` that makes `c` on `page`, grouped with `group` when given.
fn add_op(
    c: &Comment,
    page: PageId,
    group: Option<AnnotId>,
    nm: Option<String>,
    blob: Option<BlobId>,
) -> Option<AddAnnot> {
    let mut spec = AnnotSpec::from_entries(&c.entries)?;
    if let (AnnotSpec::FileAttachment(attachment), Some(file), Some(blob)) = (&mut spec, &c.file, blob) {
        let mut to_attach = FileToAttach::new(file.name.clone(), blob, file.mime.clone());
        to_attach.description.clone_from(&file.description);
        to_attach.created = file.created.as_deref().and_then(|d| d.parse().ok());
        to_attach.modified = file.modified.as_deref().and_then(|d| d.parse().ok());
        attachment.file = to_attach;
    }
    let mut common = AnnotCommon::from_entries(&c.entries);
    common.group_with = group;
    common.popup = c.popup.map(|p| {
        PopupSpec::new(
            UserSpaceRect::from_corners(p.rect[0], p.rect[1], p.rect[2], p.rect[3]),
            p.open,
        )
    });
    if let (AnnotSpec::Note(note), Some(p)) = (&mut spec, c.popup) {
        note.open = p.open;
    }
    let mut op = AddAnnot::new(page, spec);
    op.common = common;
    op.nm = nm;
    Some(op)
}
