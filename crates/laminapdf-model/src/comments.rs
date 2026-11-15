//! Comments as threads (ISO 32000-2 §12.5.6.2, §12.5.6.4): each markup annotation that answers
//! nothing, with the replies that answer it (`/IRT` with `/RT /R`), the annotations grouped with
//! it (`/RT /Group`) and the review states set on it (Text annotations with `/StateModel` and
//! `/State`), read across every page.
//!
//! Other apps write threads loosely, so nothing is dropped: a reply whose parent is missing, or a
//! ring of `/IRT`s with no start, becomes a thread of its own marked [`BrokenThread`]. A state
//! reads whether `/StateModel` and `/State` are names or strings, as 04 §2.3 found writers store
//! them.

use std::collections::BTreeMap;

use laminapdf_cos::{Dict, Object, Result};

use crate::annot::{Annot, ReplyType};
use crate::date::PdfDate;
use crate::document::Document;
use crate::id::{AnnotId, PageId};

/// A review state that a Text annotation sets on the annotation it replies to (Table 175).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct AnnotState {
    /// `/StateModel`; when it is missing, the model the state's name belongs to.
    pub model: StateModel,
    /// `/State` as written (`Accepted`, `Rejected`, `Cancelled`, `Completed`, `None`, `Marked`,
    /// `Unmarked`, or another name a writer chose); the model's default (Table 175) when absent.
    pub state: String,
}

/// The state model a review state belongs to (Table 175).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StateModel {
    /// `Review`: `None`, `Accepted`, `Rejected`, `Cancelled`, `Completed`.
    Review,
    /// `Marked`: `Marked` or `Unmarked`, as a reviewer's check mark.
    Marked,
    /// Any other model, as written.
    Other(String),
}

impl StateModel {
    fn from_text(text: &str) -> Self {
        match text {
            "Review" => Self::Review,
            "Marked" => Self::Marked,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// Why a thread doesn't start at a comment that answers nothing.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BrokenThread {
    /// The comment replies to an annotation no page lists.
    MissingParent,
    /// The comment is part of a ring of `/IRT`s; the thread starts at the ring's annotation that
    /// comes first in the document.
    Cycle,
}

/// An annotation of a thread, with the page it is on: a reply can be on another page than its
/// comment.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ThreadMember {
    /// The page's position, from 0.
    pub page_index: usize,
    /// The annotation.
    pub annot: Annot,
}

/// One comment and everything that answers it.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CommentThread {
    /// The page the comment is on.
    pub page: PageId,
    /// The page's position, from 0.
    pub page_index: usize,
    /// The comment.
    pub comment: Annot,
    /// The annotations grouped with it (`/RT /Group`), shown, moved and deleted with it, in
    /// document order.
    pub group: Vec<ThreadMember>,
    /// Every reply in the thread, replies to replies included, in document order (pages in
    /// order, then each page's `/Annots`). Each one's [`Annot::in_reply_to`] names what it
    /// answers.
    pub replies: Vec<ThreadMember>,
    /// Every review state set in the thread, on the comment or on a reply, in document order.
    pub states: Vec<ThreadMember>,
    /// Set when the thread doesn't start at a comment that answers nothing.
    pub broken: Option<BrokenThread>,
}

impl CommentThread {
    /// The latest state of `model` set on the comment itself: the one with the latest
    /// `/CreationDate` (else `/M`), and between two without dates, the later in the document.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn latest_state(&self, model: &StateModel) -> Option<&Annot> {
        self.states
            .iter()
            .map(|m| &m.annot)
            .filter(|a| a.in_reply_to.as_ref() == Some(&self.comment.id))
            .filter(|a| a.state.as_ref().is_some_and(|s| s.model == *model))
            .reduce(|latest, a| if is_later(a, latest) { a } else { latest })
    }
}

/// Whether `a`, later in the document than `than`, is the later state: by date when both have
/// one, else by position.
fn is_later(a: &Annot, than: &Annot) -> bool {
    match (date(a), date(than)) {
        (Some(a), Some(than)) => a >= than,
        _ => true,
    }
}

fn date(a: &Annot) -> Option<PdfDate> {
    a.created
        .as_deref()
        .or(a.modified.as_deref())
        .and_then(|d| d.parse().ok())
}

/// What an annotation is to its thread.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Comment,
    Reply,
    Group,
    State,
}

fn role(a: &Annot) -> Role {
    match (&a.in_reply_to, a.reply_type, &a.state) {
        (None, _, _) => Role::Comment,
        (Some(_), _, Some(_)) => Role::State,
        (Some(_), ReplyType::Group, None) => Role::Group,
        (Some(_), ReplyType::Reply, None) => Role::Reply,
    }
}

impl Document {
    /// Every page's comments as threads, in document order of their comments.
    ///
    /// Every markup annotation is in exactly one thread; Popups, links, widgets and the other
    /// annotations that aren't markup are in none.
    ///
    /// # Errors
    /// As [`Document::annotations`].
    ///
    /// Added in 0.3.0.
    pub fn comment_threads(&self) -> Result<Vec<CommentThread>> {
        let pages = self.pages()?;
        // Every markup annotation, in document order, with its page.
        let mut all: Vec<(usize, PageId, Annot)> = Vec::new();
        for (index, page) in pages.iter().enumerate() {
            for annot in self.annotations(&page)? {
                if annot.subtype.is_markup() {
                    all.push((index, page.id().clone(), annot));
                }
            }
        }
        let position: BTreeMap<&AnnotId, usize> = all.iter().enumerate().map(|(i, (_, _, a))| (&a.id, i)).collect();
        let starts: Vec<(usize, Option<BrokenThread>)> =
            (0..all.len()).map(|i| thread_start(i, &all, &position)).collect();
        let mut threads: Vec<CommentThread> = Vec::new();
        let mut slot: BTreeMap<usize, usize> = BTreeMap::new();
        for (i, (page_index, page, annot)) in all.iter().enumerate() {
            let Some((start, broken)) = starts.get(i).copied() else {
                continue;
            };
            if start != i {
                continue;
            }
            slot.insert(i, threads.len());
            threads.push(CommentThread {
                page: page.clone(),
                page_index: *page_index,
                comment: annot.clone(),
                group: Vec::new(),
                replies: Vec::new(),
                states: Vec::new(),
                broken,
            });
        }
        for (i, (page_index, _, annot)) in all.iter().enumerate() {
            let Some(thread) = starts
                .get(i)
                .filter(|(start, _)| *start != i)
                .and_then(|(start, _)| slot.get(start))
                .and_then(|t| threads.get_mut(*t))
            else {
                continue;
            };
            let member = ThreadMember {
                page_index: *page_index,
                annot: annot.clone(),
            };
            match role(annot) {
                Role::Group => thread.group.push(member),
                Role::State => thread.states.push(member),
                Role::Reply | Role::Comment => thread.replies.push(member),
            }
        }
        Ok(threads)
    }

    /// `/StateModel` and `/State` of a Text annotation that replies to another; `None` unless it
    /// has either, and unless it replies to something (Table 175 asks for `/IRT`).
    pub(crate) fn annot_state(&self, dict: &Dict) -> Result<Option<AnnotState>> {
        if dict.get("IRT").is_none() {
            return Ok(None);
        }
        let model = self.name_or_text(dict.get("StateModel"))?;
        let state = self.name_or_text(dict.get("State"))?;
        Ok(match (model, state) {
            (None, None) => None,
            (Some(model), state) => {
                let model = StateModel::from_text(&model);
                let state = state.unwrap_or_else(|| default_state(&model).to_owned());
                Some(AnnotState { model, state })
            }
            (None, Some(state)) => Some(AnnotState {
                model: if matches!(state.as_str(), "Marked" | "Unmarked") {
                    StateModel::Marked
                } else {
                    StateModel::Review
                },
                state,
            }),
        })
    }

    /// A name or a text string, as text.
    fn name_or_text(&self, value: Option<&Object>) -> Result<Option<String>> {
        let Some(value) = value else { return Ok(None) };
        Ok(match self.read(value)?.as_deref() {
            Some(Object::Name(n)) => Some(String::from_utf8_lossy(n.as_bytes()).into_owned()),
            Some(Object::String(s)) => Some(laminapdf_syntax::text::decode_text_string(&s.bytes)),
            _ => None,
        })
    }
}

/// The state Table 175 gives a model when `/State` is absent.
const fn default_state(model: &StateModel) -> &'static str {
    match model {
        StateModel::Marked => "Unmarked",
        StateModel::Review | StateModel::Other(_) => "None",
    }
}

/// Where the thread of `all[i]` starts: the first annotation up its `/IRT` chain that answers
/// nothing, else the last one reached before a missing parent, else the first in the document of
/// the ring the chain runs into.
fn thread_start(
    i: usize,
    all: &[(usize, PageId, Annot)],
    position: &BTreeMap<&AnnotId, usize>,
) -> (usize, Option<BrokenThread>) {
    let mut seen = vec![i];
    let mut at = i;
    loop {
        let Some(parent) = all.get(at).and_then(|(_, _, a)| a.in_reply_to.as_ref()) else {
            return (at, None);
        };
        let Some(&next) = position.get(parent) else {
            return (at, Some(BrokenThread::MissingParent));
        };
        if let Some(from) = seen.iter().position(|&s| s == next) {
            let ring = seen.get(from..).unwrap_or_default();
            return (ring.iter().copied().min().unwrap_or(next), Some(BrokenThread::Cycle));
        }
        seen.push(next);
        at = next;
    }
}
