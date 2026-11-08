//! The marking of one page, run while its content stream is written: which structure
//! element or artifact each drawing belongs to, the marked-content sequences around them
//! (`/P <</MCID 0>> BDC … EMC`, MCIDs from 0 per page, §14.7.5.2) and the page's elements.
//!
//! Sequences open lazily before the first drawing of an element and close before the next
//! element or artifact starts or ends and before every `q`/`Q`, so they never contain one
//! another (§14.7.5.1: structural sequences don't nest) and nest properly with the graphics
//! state; text objects (`BT … ET`) and images (`q … Q`) sit whole inside one.

use pdf_writer::{Content, Name};

use super::artifact::{self, ArtifactKind};
use super::open::ElementId;
use super::{ContentKind, Tag, TagKind};

/// A child of a page element, in reading order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageKid {
    /// Another element of the page (its index).
    Elem(usize),
    /// A marked-content ID of the page.
    Mcid(i32),
    /// The page's link annotation with this index (in drawing order).
    Link(usize),
}

/// Where a page's element comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// Opened and ended on this page.
    Page,
    /// Opened on this page by [`Surface::open_element`](crate::Surface::open_element), to stay
    /// open after it.
    Opened(ElementId),
    /// Opened on an earlier page and resumed here: its content here joins that element, and it
    /// has no parent or place among this page's elements.
    Resumed(ElementId),
}

/// An element opened on the page.
#[derive(Debug, Clone)]
pub(crate) struct PageElem {
    pub(crate) tag: Tag,
    pub(crate) parent: Option<usize>,
    pub(crate) kids: Vec<PageKid>,
    pub(crate) origin: Origin,
}

/// What an open frame is.
#[derive(Debug, Clone, Copy)]
enum Frame {
    Elem(usize),
    Artifact(ArtifactKind),
    /// An element or artifact opened inside an artifact: still the artifact's content.
    Ignored,
}

/// What the page's structure came to.
#[derive(Debug, Default)]
pub(crate) struct PageMarks {
    pub(crate) elems: Vec<PageElem>,
    /// The page's top-level elements, in order.
    pub(crate) roots: Vec<usize>,
    /// The owning element of each MCID (the MCID is the index).
    pub(crate) mcids: Vec<usize>,
    /// The element owning each link annotation (its `OBJR`), in drawing order; `None` for a
    /// link outside the structure (untagged pages, artifacts).
    pub(crate) links: Vec<Option<usize>>,
    /// Drawings outside every element and artifact, by [`ContentKind::ALL`].
    pub(crate) untagged: [usize; 4],
    /// Whether the page used any element or artifact.
    pub(crate) used: bool,
    /// Whether the page resumed an element that isn't open.
    pub(crate) resumed_unknown: bool,
}

/// The marking in progress.
#[derive(Debug)]
pub(crate) struct Marking {
    /// Whether elements are kept and sequences written (off for appended pages, whose
    /// document's structure tree the builder doesn't merge).
    enabled: bool,
    /// Whether a link outside every element gets a `Link` element of its own: only when the
    /// document is tagged, so untagged documents stay as they were.
    tagged: bool,
    /// The page's crop box in user space, for a background artifact's `/BBox`.
    page_box: [f32; 4],
    stack: Vec<Frame>,
    /// Whether a sequence is open (for the innermost frame).
    open: bool,
    /// The elements earlier pages left open, which this page may resume.
    resumable: Vec<(ElementId, Tag)>,
    pub(crate) marks: PageMarks,
}

impl Marking {
    pub(crate) fn new(enabled: bool, tagged: bool, page_box: [f32; 4]) -> Self {
        Self {
            enabled,
            tagged,
            page_box,
            stack: Vec::new(),
            open: false,
            resumable: Vec::new(),
            marks: PageMarks::default(),
        }
    }

    /// The elements earlier pages left open, which this page may resume.
    pub(crate) fn with_resumable(mut self, resumable: Vec<(ElementId, Tag)>) -> Self {
        self.resumable = resumable;
        self
    }

    /// The innermost element or artifact.
    fn context(&self) -> Option<Frame> {
        self.stack.iter().rev().find(|f| !matches!(f, Frame::Ignored)).copied()
    }

    fn in_artifact(&self) -> bool {
        matches!(self.context(), Some(Frame::Artifact(_)))
    }

    /// Closes the open sequence (before `q`, `Q`, or a frame change).
    pub(crate) fn close(&mut self, content: &mut Content) {
        if self.open {
            content.end_marked_content();
            self.open = false;
        }
    }

    /// Opens an element as a child of the innermost one (or of the page).
    fn add_elem(&mut self, tag: Tag, origin: Origin) -> usize {
        let parent = match self.context() {
            Some(Frame::Elem(i)) => Some(i),
            _ => None,
        };
        let index = self.marks.elems.len();
        self.marks.elems.push(PageElem {
            tag,
            parent,
            kids: Vec::new(),
            origin,
        });
        match parent.and_then(|p| self.marks.elems.get_mut(p)) {
            Some(p) => p.kids.push(PageKid::Elem(index)),
            None => self.marks.roots.push(index),
        }
        index
    }

    /// Starts element `tag`.
    pub(crate) fn begin_tag(&mut self, content: &mut Content, tag: &Tag) {
        if !self.enabled {
            return;
        }
        self.close(content);
        self.marks.used = true;
        let frame = if self.in_artifact() {
            Frame::Ignored
        } else {
            Frame::Elem(self.add_elem(tag.clone(), Origin::Page))
        };
        self.stack.push(frame);
    }

    /// Places element `id`, which stays open after the page, as a child of the innermost
    /// element (or of the page). Its content comes with [`Marking::resume`]. Inside an artifact
    /// it is part of the artifact, so it is never opened.
    pub(crate) fn open_element(&mut self, content: &mut Content, id: ElementId, tag: &Tag) {
        if !self.enabled {
            return;
        }
        self.close(content);
        self.marks.used = true;
        if !self.in_artifact() {
            self.add_elem(tag.clone(), Origin::Opened(id));
        }
    }

    /// Starts content of element `id`: opened on this page, or open from an earlier one.
    pub(crate) fn resume(&mut self, content: &mut Content, id: ElementId) {
        if !self.enabled {
            return;
        }
        self.close(content);
        self.marks.used = true;
        if self.in_artifact() {
            self.stack.push(Frame::Ignored);
            return;
        }
        let here = self
            .marks
            .elems
            .iter()
            .position(|e| matches!(e.origin, Origin::Opened(o) | Origin::Resumed(o) if o == id));
        if let Some(index) = here {
            self.stack.push(Frame::Elem(index));
            return;
        }
        let Some((_, tag)) = self.resumable.iter().find(|(o, _)| *o == id) else {
            self.marks.resumed_unknown = true;
            self.stack.push(Frame::Ignored);
            return;
        };
        let index = self.marks.elems.len();
        self.marks.elems.push(PageElem {
            tag: tag.clone(),
            parent: None,
            kids: Vec::new(),
            origin: Origin::Resumed(id),
        });
        self.stack.push(Frame::Elem(index));
    }

    /// Starts an artifact.
    pub(crate) fn begin_artifact(&mut self, content: &mut Content, kind: ArtifactKind) {
        if !self.enabled {
            return;
        }
        self.close(content);
        self.marks.used = true;
        let frame = if self.in_artifact() {
            Frame::Ignored
        } else {
            Frame::Artifact(kind)
        };
        self.stack.push(frame);
    }

    /// Ends the innermost element or artifact (none open: nothing happens).
    pub(crate) fn end(&mut self, content: &mut Content) {
        if !self.enabled {
            return;
        }
        self.close(content);
        self.stack.pop();
    }

    /// Ends everything still open at the end of the page.
    pub(crate) fn finish(mut self, content: &mut Content) -> PageMarks {
        self.close(content);
        self.marks
    }

    /// Makes sure what is drawn next is inside a sequence for the innermost frame; outside
    /// every frame, counts it as untagged content.
    pub(crate) fn mark(&mut self, content: &mut Content, kind: ContentKind) {
        if !self.enabled || self.open {
            return;
        }
        match self.context() {
            None => {
                // `ContentKind::ALL` is in declaration order.
                if let Some(count) = self.marks.untagged.get_mut(kind as usize) {
                    *count += 1;
                }
            }
            Some(Frame::Elem(i)) => {
                let Ok(mcid) = i32::try_from(self.marks.mcids.len()) else {
                    return;
                };
                self.marks.mcids.push(i);
                let Some(elem) = self.marks.elems.get_mut(i) else {
                    return;
                };
                elem.kids.push(PageKid::Mcid(mcid));
                content
                    .begin_marked_content_with_properties(Name(elem.tag.name().as_bytes()))
                    .properties()
                    .identify(mcid);
                self.open = true;
            }
            Some(Frame::Artifact(kind)) => {
                artifact::begin(content, kind, self.page_box);
                self.open = true;
            }
            Some(Frame::Ignored) => {}
        }
    }

    /// Starts the `Link` element a link annotation drawn now belongs to, if it needs one: the
    /// innermost element when it is a `Link`, else a new one inside it. Returns whether it
    /// started one (end it with [`Marking::end`]).
    pub(crate) fn begin_link(&mut self, content: &mut Content) -> bool {
        if !self.enabled || self.in_artifact() {
            return false;
        }
        match self.context() {
            Some(Frame::Elem(i)) if self.marks.elems.get(i).is_some_and(|e| e.tag.kind() == TagKind::Link) => false,
            Some(Frame::Elem(_)) => {
                self.begin_tag(content, &Tag::new(TagKind::Link));
                true
            }
            _ if self.tagged => {
                self.begin_tag(content, &Tag::new(TagKind::Link));
                true
            }
            _ => false,
        }
    }

    /// Records the next link annotation (its `OBJR`) in the innermost element, if that is a
    /// `Link`.
    pub(crate) fn link(&mut self) {
        let link = self.marks.links.len();
        let owner = match self.context() {
            Some(Frame::Elem(i)) if self.enabled => Some(i),
            _ => None,
        };
        match owner.and_then(|i| self.marks.elems.get_mut(i)) {
            Some(elem) => elem.kids.push(PageKid::Link(link)),
            None if self.enabled => {
                if let Some(count) = self.marks.untagged.get_mut(ContentKind::Link as usize) {
                    *count += 1;
                }
            }
            None => {}
        }
        self.marks.links.push(owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(f: impl FnOnce(&mut Marking, &mut Content)) -> (String, PageMarks) {
        let mut content = Content::new();
        let mut m = Marking::new(true, true, [0.0; 4]);
        f(&mut m, &mut content);
        let marks = m.finish(&mut content);
        (String::from_utf8(content.finish().into_vec()).unwrap(), marks)
    }

    #[test]
    fn mcids_count_from_zero_in_drawing_order() {
        let (text, marks) = written(|m, c| {
            m.begin_tag(c, &Tag::new(TagKind::H1));
            m.mark(c, ContentKind::Text);
            m.end(c);
            m.begin_tag(c, &Tag::new(TagKind::P));
            m.mark(c, ContentKind::Text);
            m.end(c);
        });
        assert_eq!(
            (text.as_str(), marks.mcids),
            (
                "/H1 <<\n  /MCID 0\n>> BDC\nEMC\n/P <<\n  /MCID 1\n>> BDC\nEMC",
                vec![0, 1]
            )
        );
    }

    #[test]
    fn a_child_element_closes_its_parents_sequence() {
        let (_, marks) = written(|m, c| {
            m.begin_tag(c, &Tag::new(TagKind::P));
            m.mark(c, ContentKind::Text);
            m.begin_tag(c, &Tag::new(TagKind::Span));
            m.mark(c, ContentKind::Text);
            m.end(c);
            m.mark(c, ContentKind::Text);
            m.end(c);
        });
        let kids = marks.elems.first().map(|e| e.kids.clone()).unwrap_or_default();
        assert_eq!(kids, [PageKid::Mcid(0), PageKid::Elem(1), PageKid::Mcid(2)]);
    }

    #[test]
    fn drawings_in_one_element_share_a_sequence() {
        let (_, marks) = written(|m, c| {
            m.begin_tag(c, &Tag::new(TagKind::Figure));
            m.mark(c, ContentKind::Path);
            m.mark(c, ContentKind::Image);
            m.end(c);
        });
        assert_eq!(marks.mcids, [0]);
    }

    #[test]
    fn an_element_inside_an_artifact_is_part_of_the_artifact() {
        let (text, marks) = written(|m, c| {
            m.begin_artifact(c, ArtifactKind::Layout);
            m.begin_tag(c, &Tag::new(TagKind::P));
            m.mark(c, ContentKind::Path);
            m.end(c);
            m.end(c);
        });
        assert_eq!(
            (text.as_str(), marks.elems.len()),
            ("/Artifact <<\n  /Type /Layout\n>> BDC\nEMC", 0)
        );
    }

    #[test]
    fn content_outside_every_element_is_counted() {
        let (text, marks) = written(|m, c| {
            m.mark(c, ContentKind::Path);
            m.mark(c, ContentKind::Path);
        });
        assert_eq!((text.as_str(), marks.untagged), ("", [2, 0, 0, 0]));
    }

    fn open_id() -> ElementId {
        super::super::open::ElementIds::new(0, 0).next()
    }

    #[test]
    fn a_resumed_element_is_no_root_and_owns_its_mcids() {
        let mut content = Content::new();
        let id = open_id();
        let mut m = Marking::new(true, true, [0.0; 4]).with_resumable(vec![(id, Tag::new(TagKind::P))]);
        m.resume(&mut content, id);
        m.mark(&mut content, ContentKind::Text);
        m.end(&mut content);
        let marks = m.finish(&mut content);
        let origin = marks.elems.first().map(|e| e.origin);
        assert_eq!(
            (origin, marks.roots.len(), marks.mcids),
            (Some(Origin::Resumed(id)), 0, vec![0])
        );
    }

    #[test]
    fn resuming_twice_on_a_page_adds_to_one_element() {
        let (_, marks) = written(|m, c| {
            let id = open_id();
            m.open_element(c, id, &Tag::new(TagKind::P));
            for _ in 0..2 {
                m.resume(c, id);
                m.mark(c, ContentKind::Text);
                m.end(c);
            }
        });
        let kids = marks.elems.first().map(|e| e.kids.clone()).unwrap_or_default();
        assert_eq!((marks.elems.len(), kids), (1, vec![PageKid::Mcid(0), PageKid::Mcid(1)]));
    }

    #[test]
    fn resuming_an_element_that_isnt_open_is_reported() {
        let (_, marks) = written(|m, c| m.resume(c, open_id()));
        assert!(marks.resumed_unknown);
    }

    #[test]
    fn a_link_inside_a_paragraph_gets_its_own_link_element() {
        let (_, marks) = written(|m, c| {
            m.begin_tag(c, &Tag::new(TagKind::P));
            let started = m.begin_link(c);
            m.link();
            if started {
                m.end(c);
            }
            m.end(c);
        });
        assert_eq!((marks.elems.len(), marks.links), (2, vec![Some(1)]));
    }
}
