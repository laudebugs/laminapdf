//! An exchange file applied to a document as one transaction of ordinary ops: an import is one
//! undo step, and the op log replays it like any other edit. A plan, made without changing the
//! document, says what each comment becomes; the preview reports it and the import applies it,
//! so the two always agree.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, Object};
use laminapdf_model::{AnnotId, PageId};
use laminapdf_ops::{
    AddAnnot, AddReply, AnnotCommon, AnnotSpec, Applied, BlobId, DeleteAnnot, FileToAttach, Op, Origin, PopupSpec,
    ReviewState, SetReviewState, Tx,
};
use laminapdf_syntax::{ErrorKind, Location};

use crate::Result;
use crate::exchange::{Comment, Exchange, SkipCause, SkippedComment};

/// What an import does with the comments already in the document: the ones the file names by
/// `/NM` on the same page.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImportMode {
    /// Keeps them as they are, and adds the file's other comments: new replies and statuses
    /// join their threads.
    #[default]
    Merge,
    /// Replaces each with the file's version.
    Replace,
    /// Keeps them, and adds the file's as copies; a copy whose `/NM` is taken gets a new one.
    Add,
}

/// How to import.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImportCommentsOptions {
    /// What to do with the comments already there.
    pub mode: ImportMode,
    /// The document's file name, which the exchange file's (`<f href>`, `/F`) is checked against
    /// when it carries no `/ID`.
    pub file: Option<String>,
}

impl ImportCommentsOptions {
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
    Skipped(SkippedComment),
    /// `interchange-document-mismatch`: the file was made for another PDF; its comments may land
    /// in the wrong places.
    DocumentMismatch {
        /// The PDF the file names, when it names one.
        file: Option<String>,
    },
    /// `annot-name-duplicate`: two comments in the file have the same `/NM` on one page, so one
    /// name finds either.
    DuplicateName {
        /// The page, counted from 0.
        page: u32,
        /// The name.
        name: String,
    },
}

impl ImportWarning {
    /// The stable code: `interchange-skipped`, `interchange-document-mismatch` or
    /// `annot-name-duplicate`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Skipped(_) => "interchange-skipped",
            Self::DocumentMismatch { .. } => "interchange-document-mismatch",
            Self::DuplicateName { .. } => "annot-name-duplicate",
        }
    }

    /// The code's kind (ADR 0026 §2).
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Skipped(_) => ErrorKind::Unsupported,
            Self::DocumentMismatch { .. } | Self::DuplicateName { .. } => ErrorKind::Input,
        }
    }

    /// Where in the document it is: nowhere yet, since the comments aren't in it; a skipped
    /// comment's page and line are in the warning itself.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn location(&self) -> Location {
        Location::NONE
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
    /// The comments the file names that are already here (Merge and Add keep them).
    pub matched: usize,
    /// Of those, the ones whose text the file gives otherwise.
    pub changed: usize,
    /// The comments deleted to be replaced (Replace), each with its replies.
    pub deleted: usize,
    /// The pages the added comments are on, counted from 0, in order.
    pub pages: Vec<u32>,
    /// What to say about it.
    pub warnings: Vec<ImportWarning>,
}

/// What importing `exchange` into `doc` would do, without changing it.
///
/// # Errors
/// Reading the document.
///
/// Added in 0.7.0.
pub fn preview(
    doc: &laminapdf_model::Document,
    exchange: &Exchange,
    options: &ImportCommentsOptions,
) -> Result<ImportSummary> {
    Ok(Plan::new(doc, exchange, options)?.summary)
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
    options: &ImportCommentsOptions,
) -> Result<(ImportSummary, Applied)> {
    let plan = Plan::new(doc, exchange, options)?;
    // A file's bytes go in the blob store first: a transaction reads blobs, it doesn't add them.
    let blobs: Vec<Option<BlobId>> = plan
        .steps
        .iter()
        .map(|s| s.comment.file.as_ref().map(|f| doc.add_blob(f.bytes.clone())))
        .collect();
    let Plan {
        steps,
        doomed,
        existing,
        pages,
        mut summary,
    } = plan;
    let (deleted, applied) = doc.transact(Origin::Local, |tx| {
        let mut run = Run {
            tx,
            pages: &pages,
            existing,
            created: BTreeMap::new(),
        };
        let deleted = run.delete(&doomed)?;
        for (step, blob) in steps.iter().zip(&blobs) {
            run.step(step, *blob)?;
        }
        Ok(deleted)
    })?;
    summary.deleted = deleted;
    Ok((summary, applied))
}

/// What a comment becomes.
enum Action {
    /// Kept as the document has it (Merge, Add): the file's replies to it join it.
    Keep(AnnotId),
    /// Added.
    Add(Kind),
}

/// How an added comment relates to the one it names in `/IRT`.
#[derive(Clone, Copy)]
enum Kind {
    /// A comment that answers nothing.
    Top,
    /// Grouped with it (`/RT /Group`).
    Group,
    /// A reply (`/RT /R`, the default, ISO 32000-2 Table 170).
    Reply,
    /// A review status: a reply with `/State` and `/StateModel`.
    State(ReviewState),
}

/// One comment of the file and what it becomes.
struct Step<'e> {
    comment: &'e Comment,
    action: Action,
    /// The `/NM` it is written with: the file's, or in Add a free one.
    nm: Option<String>,
}

/// An annotation already in the document: its ID and its text.
struct Existing {
    id: AnnotId,
    contents: Option<String>,
}

/// What an import does, worked out before anything changes.
struct Plan<'e> {
    steps: Vec<Step<'e>>,
    /// The comments Replace deletes first.
    doomed: Vec<AnnotId>,
    /// Each page's annotations by `/NM` (or ID), less the doomed.
    existing: Vec<BTreeMap<String, AnnotId>>,
    pages: Vec<PageId>,
    summary: ImportSummary,
}

impl<'e> Plan<'e> {
    fn new(doc: &laminapdf_model::Document, exchange: &'e Exchange, options: &ImportCommentsOptions) -> Result<Self> {
        let all = doc.pages()?;
        let mut pages = Vec::with_capacity(all.len());
        let mut here: Vec<BTreeMap<String, Existing>> = Vec::with_capacity(all.len());
        for page in all.iter() {
            pages.push(page.id().clone());
            // By `/NM`, or by ID for one without, as an export names them, so a file exported
            // from this document finds its comments again.
            here.push(
                doc.annotations(&page)?
                    .into_iter()
                    .map(|a| {
                        let name = a.name.clone().unwrap_or_else(|| a.id.to_string());
                        (
                            name,
                            Existing {
                                id: a.id,
                                contents: a.contents,
                            },
                        )
                    })
                    .collect(),
            );
        }
        let mut summary = ImportSummary {
            warnings: first_warnings(doc, exchange, options),
            ..ImportSummary::default()
        };
        summary.warnings.extend(duplicates(exchange));
        let (order, skipped) = in_order(exchange, &here);
        summary.warnings.extend(skipped.into_iter().map(ImportWarning::Skipped));
        let mut taken: Vec<BTreeSet<String>> = here.iter().map(|names| names.keys().cloned().collect()).collect();
        let mut doomed = Vec::new();
        let mut steps = Vec::with_capacity(order.len());
        let mut added_pages = BTreeSet::new();
        for (comment, kind) in order {
            let page = usize::try_from(comment.page).unwrap_or(usize::MAX);
            let name = comment.name();
            let found = name.as_ref().and_then(|n| here.get(page)?.get(n));
            let (action, nm) = match (options.mode, found) {
                (ImportMode::Merge, Some(existing)) => {
                    summary.matched += 1;
                    let theirs = crate::exchange::text(&comment.entries, "Contents");
                    if theirs.is_some() && theirs != existing.contents {
                        summary.changed += 1;
                    }
                    (Action::Keep(existing.id.clone()), name)
                }
                (ImportMode::Replace, Some(existing)) => {
                    summary.matched += 1;
                    doomed.push(existing.id.clone());
                    (Action::Add(kind), name)
                }
                (ImportMode::Add, found) => {
                    summary.matched += usize::from(found.is_some());
                    let fresh = name.map(|n| fresh_name(n, taken.get(page)));
                    (Action::Add(kind), fresh)
                }
                (_, _) => (Action::Add(kind), name),
            };
            if let (Some(n), Some(names)) = (&nm, taken.get_mut(page)) {
                names.insert(n.clone());
            }
            if matches!(action, Action::Add(_)) {
                summary.added += 1;
                added_pages.insert(comment.page);
            }
            steps.push(Step { comment, action, nm });
        }
        summary.pages = added_pages.into_iter().collect();
        let doomed_set: BTreeSet<&AnnotId> = doomed.iter().collect();
        let existing = here
            .into_iter()
            .map(|names| {
                names
                    .into_iter()
                    .filter(|(_, e)| !doomed_set.contains(&e.id))
                    .map(|(n, e)| (n, e.id))
                    .collect()
            })
            .collect();
        Ok(Self {
            steps,
            doomed,
            existing,
            pages,
            summary,
        })
    }
}

/// `name`, or when it is taken on the page, the first of `name-2`, `name-3`, … that isn't.
fn fresh_name(name: String, taken: Option<&BTreeSet<String>>) -> String {
    let Some(taken) = taken else {
        return name;
    };
    if !taken.contains(&name) {
        return name;
    }
    // One more candidate than names taken: one of them is free.
    (2..=taken.len() + 2)
        .map(|i| format!("{name}-{i}"))
        .find(|n| !taken.contains(n))
        .unwrap_or(name)
}

/// The names the file gives two comments on one page.
fn duplicates(exchange: &Exchange) -> Vec<ImportWarning> {
    let mut seen = BTreeSet::new();
    let mut warned = BTreeSet::new();
    let mut out = Vec::new();
    for c in &exchange.comments {
        let Some(name) = c.name() else { continue };
        let key = (c.page, name);
        if !seen.insert(key.clone()) && warned.insert(key.clone()) {
            out.push(ImportWarning::DuplicateName {
                page: key.0,
                name: key.1,
            });
        }
    }
    out
}

/// The mismatch warning, and the file's own skipped entries.
fn first_warnings(
    doc: &laminapdf_model::Document,
    exchange: &Exchange,
    options: &ImportCommentsOptions,
) -> Vec<ImportWarning> {
    let mut out = Vec::new();
    if mismatched(doc, exchange, options) {
        out.push(ImportWarning::DocumentMismatch {
            file: exchange.file.clone(),
        });
    }
    out.extend(exchange.skipped.iter().cloned().map(ImportWarning::Skipped));
    out
}

/// Whether the file names another PDF: by the permanent `/ID` when both have one, else by the
/// file's name.
fn mismatched(doc: &laminapdf_model::Document, exchange: &Exchange, options: &ImportCommentsOptions) -> bool {
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

/// A text string or a name entry of `dict`: some writers store `/State` and `/StateModel` as
/// names (ISO 32000-2 Table 172 has them as text strings).
fn text_or_name(dict: &Dict, key: &str) -> Option<String> {
    crate::exchange::text(dict, key).or_else(|| dict.get_name(key).map(|n| String::from_utf8_lossy(n).into_owned()))
}

/// What `c` is, given what it names in `/IRT`; `None` when Lamina PDF can't make it.
fn kind_of(c: &Comment) -> Option<Kind> {
    if c.in_reply_to.is_none() {
        return AnnotSpec::from_entries(&c.entries).map(|_| Kind::Top);
    }
    if c.entries.get_name("RT") == Some(b"Group".as_slice()) {
        return AnnotSpec::from_entries(&c.entries).map(|_| Kind::Group);
    }
    if let (Some(model), Some(state)) = (
        text_or_name(&c.entries, "StateModel"),
        text_or_name(&c.entries, "State"),
    ) {
        return review_state(&model, &state).map(Kind::State);
    }
    // A reply Lamina PDF writes is a note; another kind of reply is kept as a comment of its own.
    if c.subtype() == "Text" {
        Some(Kind::Reply)
    } else {
        AnnotSpec::from_entries(&c.entries).map(|_| Kind::Top)
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

/// The comments to import, each after the one it answers, with what each is, and what is
/// skipped: a kind Lamina PDF can't make, a page the document doesn't have, a parent neither
/// the file nor the document has.
fn in_order<'e>(
    exchange: &'e Exchange,
    here: &[BTreeMap<String, Existing>],
) -> (Vec<(&'e Comment, Kind)>, Vec<SkippedComment>) {
    let skip = |c: &Comment, cause| SkippedComment {
        page: Some(c.page),
        kind: c.subtype().to_owned(),
        cause,
        line: c.line,
    };
    let mut skipped = Vec::new();
    let mut waiting = Vec::new();
    for c in &exchange.comments {
        let on_a_page = usize::try_from(c.page).is_ok_and(|p| p < here.len());
        match (on_a_page, kind_of(c)) {
            (false, _) => skipped.push(skip(c, SkipCause::MissingPage)),
            (true, None) => skipped.push(skip(c, SkipCause::Unsupported)),
            (true, Some(kind)) => waiting.push((c, kind)),
        }
    }
    let mut placed: BTreeSet<String> = here.iter().flat_map(|names| names.keys().cloned()).collect();
    let mut order = Vec::new();
    loop {
        let before = order.len();
        waiting.retain(|(c, kind)| {
            let ready = matches!(kind, Kind::Top) || c.in_reply_to.as_ref().is_none_or(|p| placed.contains(p));
            if ready {
                placed.extend(c.name());
                order.push((*c, *kind));
            }
            !ready
        });
        if order.len() == before {
            break;
        }
    }
    skipped.extend(waiting.into_iter().map(|(c, _)| skip(c, SkipCause::MissingParent)));
    (order, skipped)
}

/// One import's transaction.
struct Run<'t, 'd> {
    tx: &'t mut Tx<'d>,
    pages: &'t [PageId],
    /// Each page's annotations by `/NM` (or ID), less those deleted.
    existing: Vec<BTreeMap<String, AnnotId>>,
    /// The annotations this import made or kept, by the `/NM` the file gave them.
    created: BTreeMap<String, AnnotId>,
}

impl Run<'_, '_> {
    fn apply(&mut self, op: impl Into<Op>) -> std::result::Result<Applied, laminapdf_ops::Error> {
        self.tx.apply(op)
    }

    /// Deletes `doomed`, skipping any a deletion before took with it; how many it deleted.
    fn delete(&mut self, doomed: &[AnnotId]) -> std::result::Result<usize, laminapdf_ops::Error> {
        let mut gone: BTreeSet<AnnotId> = BTreeSet::new();
        let mut deleted = 0;
        for id in doomed {
            if gone.contains(id) {
                continue;
            }
            let applied = self.apply(DeleteAnnot::new(id.clone()))?;
            gone.insert(id.clone());
            gone.extend(applied.also_deleted.iter().cloned());
            deleted += 1;
        }
        // A reply deleted with its comment can't be answered any more.
        for names in &mut self.existing {
            names.retain(|_, id| !gone.contains(id));
        }
        Ok(deleted)
    }

    /// The annotation `name` refers to: one this import made or kept, else one already in the
    /// document, on any page, as a reply on another page answers its comment there.
    fn find(&self, name: &str) -> Option<AnnotId> {
        self.created
            .get(name)
            .cloned()
            .or_else(|| self.existing.iter().find_map(|names| names.get(name).cloned()))
    }

    fn step(&mut self, step: &Step<'_>, blob: Option<BlobId>) -> std::result::Result<(), laminapdf_ops::Error> {
        let c = step.comment;
        let kind = match &step.action {
            Action::Keep(id) => {
                if let Some(n) = c.name() {
                    self.created.insert(n, id.clone());
                }
                return Ok(());
            }
            Action::Add(kind) => *kind,
        };
        let parent = c.in_reply_to.as_deref().and_then(|p| self.find(p));
        let common = AnnotCommon::from_entries(&c.entries);
        let page = self.pages.get(usize::try_from(c.page).unwrap_or(usize::MAX)).cloned();
        let op: Op = match (kind, parent, page) {
            (Kind::State(state), Some(target), _) => {
                let mut op = SetReviewState::new(target, state);
                op.author = common.author;
                op.created = common.created;
                op.nm.clone_from(&step.nm);
                op.into()
            }
            (Kind::Reply, Some(target), _) => {
                let mut op = AddReply::new(target, common.contents.unwrap_or_default());
                op.author = common.author;
                op.created = common.created;
                op.nm.clone_from(&step.nm);
                op.into()
            }
            (Kind::Top | Kind::Group, parent, Some(page)) => {
                let group = if matches!(kind, Kind::Group) { parent } else { None };
                match add_op(c, page, group, step.nm.clone(), blob) {
                    Some(op) => op.into(),
                    None => return Ok(()),
                }
            }
            // The plan placed each comment after what it answers, so this doesn't happen.
            _ => return Ok(()),
        };
        let applied = self.apply(op)?;
        if let (Some(n), Some(id)) = (c.name(), applied.created_annotations.first()) {
            self.created.insert(n, id.clone());
        }
        Ok(())
    }
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
    common.popup = c.popup.map(|p| PopupSpec::new(p.rect, p.open));
    if let (AnnotSpec::Note(note), Some(p)) = (&mut spec, c.popup) {
        note.open = p.open;
    }
    let mut op = AddAnnot::new(page, spec);
    op.common = common;
    op.nm = nm;
    Some(op)
}
