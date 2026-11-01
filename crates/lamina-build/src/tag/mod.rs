//! Tagged PDF (ISO 32000-2 §14.7–14.8; research 03 §9): structure elements built while the
//! page is drawn ([`Surface::tag`](crate::Surface::tag), [`TextRun::tag`](crate::TextRun::tag),
//! [`Surface::figure`](crate::Surface::figure)), artifacts ([`Surface::artifact`](crate::Surface::artifact)),
//! marked-content IDs per page, the `ParentTree`, `OBJR`s for link annotations and the
//! `RoleMap` for custom names.
//!
//! * `elem`: the document's elements and how they are written at [`Builder::finish`](crate::Builder::finish).
//! * `mcid`: the per-page marking, run while the content stream is written.
//! * `parent_tree`: the number tree from `/StructParents` and `/StructParent` keys.
//! * `artifact`: [`ArtifactKind`] and its property lists.
//! * `rolemap`: custom names mapped to standard types.

pub(crate) mod artifact;
pub(crate) mod elem;
pub(crate) mod mcid;
pub(crate) mod parent_tree;
pub(crate) mod rolemap;

pub use artifact::ArtifactKind;

/// A standard structure type (ISO 32000-1 §14.8.4, the PDF 1.7 set every PDF 2.0 reader
/// knows; research 03 §9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TagKind {
    /// `Document`: the whole document (the builder writes one as the root's only child).
    Document,
    /// `Part`: a large division.
    Part,
    /// `Sect`: a section.
    Sect,
    /// `H1`, a first-level heading.
    H1,
    /// `H2`.
    H2,
    /// `H3`.
    H3,
    /// `H4`.
    H4,
    /// `H5`.
    H5,
    /// `H6`.
    H6,
    /// `P`: a paragraph.
    P,
    /// `L`: a list (of `LI`s).
    L,
    /// `LI`: a list item (a `Lbl` and an `LBody`).
    LI,
    /// `Lbl`: a list item's bullet or number, or a caption's label.
    Lbl,
    /// `LBody`: a list item's content.
    LBody,
    /// `Table` (of `TR`s).
    Table,
    /// `TR`: a table row (of `TH`s and `TD`s).
    TR,
    /// `TH`: a header cell ([`Tag::with_scope`]).
    TH,
    /// `TD`: a data cell.
    TD,
    /// `Figure`: an illustration, which needs alternate text ([`Tag::with_alt`]).
    Figure,
    /// `Caption`.
    Caption,
    /// `Link`: a link's content and its annotation (an `OBJR`).
    Link,
    /// `Span`: inline text with its own attributes (`/Lang`, `/ActualText`).
    Span,
    /// `Quote`: an inline quotation.
    Quote,
    /// `Note`: a footnote or endnote.
    Note,
    /// `Code`: computer code.
    Code,
}

impl TagKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 25] = [
        Self::Document,
        Self::Part,
        Self::Sect,
        Self::H1,
        Self::H2,
        Self::H3,
        Self::H4,
        Self::H5,
        Self::H6,
        Self::P,
        Self::L,
        Self::LI,
        Self::Lbl,
        Self::LBody,
        Self::Table,
        Self::TR,
        Self::TH,
        Self::TD,
        Self::Figure,
        Self::Caption,
        Self::Link,
        Self::Span,
        Self::Quote,
        Self::Note,
        Self::Code,
    ];

    /// The structure type's name (`/S`), such as `"H1"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Document => "Document",
            Self::Part => "Part",
            Self::Sect => "Sect",
            Self::H1 => "H1",
            Self::H2 => "H2",
            Self::H3 => "H3",
            Self::H4 => "H4",
            Self::H5 => "H5",
            Self::H6 => "H6",
            Self::P => "P",
            Self::L => "L",
            Self::LI => "LI",
            Self::Lbl => "Lbl",
            Self::LBody => "LBody",
            Self::Table => "Table",
            Self::TR => "TR",
            Self::TH => "TH",
            Self::TD => "TD",
            Self::Figure => "Figure",
            Self::Caption => "Caption",
            Self::Link => "Link",
            Self::Span => "Span",
            Self::Quote => "Quote",
            Self::Note => "Note",
            Self::Code => "Code",
        }
    }

    /// The kind named `name` (exactly, as [`TagKind::as_str`] spells it).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == name)
    }
}

/// Which header cells a `TH` heads (the `/Scope` attribute, §14.8.5.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TableScope {
    /// The cells of its row.
    Row,
    /// The cells of its column.
    Column,
    /// Both.
    Both,
}

impl TableScope {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Row => "Row",
            Self::Column => "Column",
            Self::Both => "Both",
        }
    }
}

/// A structure element to open: its type and attributes.
///
/// ```
/// use lamina_build::{TableScope, Tag, TagKind};
///
/// let heading = Tag::new(TagKind::H1).with_lang("en-GB");
/// let logo = Tag::new(TagKind::Figure).with_alt("The Acme logo");
/// let header = Tag::new(TagKind::TH).with_scope(TableScope::Column);
/// let custom = Tag::custom("InvoiceTotal", TagKind::P); // written with a RoleMap entry
/// assert_eq!((custom.name(), custom.kind()), ("InvoiceTotal", TagKind::P));
/// assert_eq!(logo.alt(), Some("The Acme logo"));
/// # let _ = (heading, header);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    kind: TagKind,
    custom: Option<String>,
    alt: Option<String>,
    actual_text: Option<String>,
    expansion: Option<String>,
    lang: Option<String>,
    scope: Option<TableScope>,
}

impl From<TagKind> for Tag {
    fn from(kind: TagKind) -> Self {
        Self::new(kind)
    }
}

impl Tag {
    /// An element of a standard type.
    #[must_use]
    pub const fn new(kind: TagKind) -> Self {
        Self {
            kind,
            custom: None,
            alt: None,
            actual_text: None,
            expansion: None,
            lang: None,
            scope: None,
        }
    }

    /// An element with a custom type `name`, which the `RoleMap` maps to `role` (§14.8.3).
    /// Standard types are never remapped: a `name` that is one of [`TagKind`]'s is that kind,
    /// and any other standard type of PDF 1.7 or 2.0 (`Div`, `BlockQuote`, `TOC`, `THead`,
    /// `Formula`, `H7`, …) is written as `/S` unmapped, with `role` only saying how the builder
    /// treats it (a `Formula` written with the role `Figure` needs alternate text).
    #[must_use]
    pub fn custom(name: impl Into<String>, role: TagKind) -> Self {
        let name = name.into();
        match TagKind::from_name(&name) {
            Some(kind) => Self::new(kind),
            None => Self {
                custom: Some(name),
                ..Self::new(role)
            },
        }
    }

    /// `/Alt`: the alternate description (§14.9.3), required for a `Figure` by PDF/UA.
    #[must_use]
    pub fn with_alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// `/ActualText`: the exact text the content stands for (§14.9.4).
    #[must_use]
    pub fn with_actual_text(mut self, text: impl Into<String>) -> Self {
        self.actual_text = Some(text.into());
        self
    }

    /// `/E`: the expansion of an abbreviation (§14.9.5).
    #[must_use]
    pub fn with_expansion(mut self, expansion: impl Into<String>) -> Self {
        self.expansion = Some(expansion.into());
        self
    }

    /// `/Lang`: the element's natural language, a BCP 47 tag (§14.9.2).
    #[must_use]
    pub fn with_lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }

    /// The `/Scope` table attribute (for a `TH`).
    #[must_use]
    pub const fn with_scope(mut self, scope: TableScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// The standard type: the element's own, or the one a custom name maps to.
    #[must_use]
    pub const fn kind(&self) -> TagKind {
        self.kind
    }

    /// The name written as `/S` and in the content stream: the custom name, else the type's.
    #[must_use]
    pub fn name(&self) -> &str {
        self.custom.as_deref().unwrap_or(self.kind.as_str())
    }

    /// The custom name, if any.
    #[must_use]
    pub fn custom_name(&self) -> Option<&str> {
        self.custom.as_deref()
    }

    /// `/Alt`, if set.
    #[must_use]
    pub fn alt(&self) -> Option<&str> {
        self.alt.as_deref()
    }

    /// `/ActualText`, if set.
    #[must_use]
    pub fn actual_text(&self) -> Option<&str> {
        self.actual_text.as_deref()
    }

    /// `/E`, if set.
    #[must_use]
    pub fn expansion(&self) -> Option<&str> {
        self.expansion.as_deref()
    }

    /// `/Lang`, if set.
    #[must_use]
    pub fn lang(&self) -> Option<&str> {
        self.lang.as_deref()
    }

    /// `/Scope`, if set.
    #[must_use]
    pub const fn scope(&self) -> Option<TableScope> {
        self.scope
    }

    /// Whether the element is a figure without an alternate description (`/Alt` or
    /// `/ActualText`), which PDF/UA rejects.
    pub(crate) fn lacks_alt(&self) -> bool {
        self.kind == TagKind::Figure && self.alt.is_none() && self.actual_text.is_none()
    }
}

/// What was drawn outside every structure element and artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ContentKind {
    /// A painted path (a clip alone draws nothing, so it isn't counted).
    Path,
    /// An image.
    Image,
    /// A text run or text box.
    Text,
    /// A link annotation without a `Link` element (`/StructParent`): drawn inside an
    /// artifact, or on a page finished before the document was tagged.
    Link,
}

impl ContentKind {
    /// In declaration order (`kind as usize` indexes it).
    pub(crate) const ALL: [Self; 4] = [Self::Path, Self::Image, Self::Text, Self::Link];

    /// `"path"`, `"image"`, `"text"` or `"link"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Image => "image",
            Self::Text => "text",
            Self::Link => "link",
        }
    }
}

/// What a tagged document lacks, found while its pages were drawn
/// ([`Builder::tagging_diagnostics`](crate::Builder::tagging_diagnostics)). Conformance profiles
/// ([B8b]) turn these into errors; the builder itself writes the document either way.
///
/// The builder counts on every page whether or not the document is tagged yet, so a document
/// that becomes tagged only when a later page uses an element still reports everything its
/// earlier pages drew and linked as [`TagDiagnostic::Untagged`]: the list is empty only when
/// every drawing and every link annotation has structure (or is an artifact's drawing).
///
/// [B8b]: https://github.com/laudebugs/lamina/blob/main/docs/todos/done/B8b-conformance-profiles-and-verapdf.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TagDiagnostic {
    /// Content drawn outside every element and artifact on page `page` (0-based), `count`
    /// times.
    Untagged {
        /// The page's index.
        page: u32,
        /// What was drawn.
        content: ContentKind,
        /// How many times.
        count: usize,
    },
    /// A `Figure` on page `page` without `/Alt` or `/ActualText`.
    MissingAlt {
        /// The page's index.
        page: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_is_found_by_its_name() {
        assert!(TagKind::ALL.iter().all(|k| TagKind::from_name(k.as_str()) == Some(*k)));
    }

    #[test]
    fn a_custom_name_that_is_a_standard_type_is_that_type() {
        let tag = Tag::custom("H2", TagKind::P);
        assert_eq!((tag.kind(), tag.custom_name()), (TagKind::H2, None));
    }

    #[test]
    fn a_figure_with_actual_text_has_an_alternate_description() {
        assert!(!Tag::new(TagKind::Figure).with_actual_text("A").lacks_alt());
    }
}
