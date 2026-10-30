//! The document outline (§12.3.3): `/Outlines` with `/First`, `/Last`, `/Prev`, `/Next`,
//! `/Parent` and `/Count`.

use lamina_cos::{CosMut, Dict, Name, ObjRef, Object, PdfString};
use lamina_syntax::text::encode_text_string;

use super::dest::{LinkTarget, Targets};
use crate::error::Result;

/// One outline item ("bookmark"): a title, where it goes, its children, whether it starts
/// open, and its colour and style (`/C`, `/F`, PDF 1.4).
///
/// ```
/// use lamina_build::{LinkTarget, OutlineEntry};
///
/// let chapter = OutlineEntry::new("Chapter 1", LinkTarget::page(0))
///     .with_child(OutlineEntry::new("Section 1.1", LinkTarget::Named("s1.1".into())))
///     .open(true)
///     .bold();
/// assert_eq!(chapter.children().len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct OutlineEntry {
    title: String,
    target: Option<LinkTarget>,
    children: Vec<OutlineEntry>,
    open: bool,
    color: Option<[f64; 3]>,
    bold: bool,
    italic: bool,
}

impl OutlineEntry {
    /// An item titled `title` going to `target`, closed, with no children.
    #[must_use]
    pub fn new(title: impl Into<String>, target: LinkTarget) -> Self {
        Self {
            target: Some(target),
            ..Self::heading(title)
        }
    }

    /// An item that goes nowhere: a heading for its children.
    #[must_use]
    pub fn heading(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            target: None,
            children: Vec::new(),
            open: false,
            color: None,
            bold: false,
            italic: false,
        }
    }

    /// With `child` added after the existing children.
    #[must_use]
    pub fn with_child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }

    /// With `children` added after the existing children.
    #[must_use]
    pub fn with_children(mut self, children: impl IntoIterator<Item = Self>) -> Self {
        self.children.extend(children);
        self
    }

    /// Whether the item's children are shown when the document opens (default: closed).
    #[must_use]
    pub const fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The title's colour, RGB components in `[0, 1]` (clamped; NaN is 0).
    #[must_use]
    pub fn with_color(mut self, r: f64, g: f64, b: f64) -> Self {
        let unit = |v: f64| if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
        self.color = Some([unit(r), unit(g), unit(b)]);
        self
    }

    /// A bold title.
    #[must_use]
    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// An italic title.
    #[must_use]
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// The title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Where it goes.
    #[must_use]
    pub const fn target(&self) -> Option<&LinkTarget> {
        self.target.as_ref()
    }

    /// The children.
    #[must_use]
    pub fn children(&self) -> &[Self] {
        &self.children
    }

    /// The number of items in this item's subtree, itself included.
    pub(crate) fn len(&self) -> usize {
        1 + self.children.iter().map(Self::len).sum::<usize>()
    }

    /// Whether it uses colour or style (PDF 1.4).
    pub(crate) fn styled(&self) -> bool {
        self.color.is_some() || self.bold || self.italic || self.children.iter().any(Self::styled)
    }

    /// The descendants shown when this item is open: each child, and the descendants of
    /// the open ones.
    fn visible(&self) -> usize {
        self.children
            .iter()
            .map(|c| 1 + if c.open { c.visible() } else { 0 })
            .sum()
    }
}

fn count(n: usize) -> Object {
    Object::Integer(i64::try_from(n).unwrap_or(i64::MAX))
}

/// Writes the outline: the root at `root`, the items at `ids` (pre-order, as
/// [`OutlineEntry::len`] counts them).
pub(crate) fn write(
    cos: &mut CosMut<'_>,
    root: ObjRef,
    entries: &[OutlineEntry],
    ids: &[ObjRef],
    targets: &Targets<'_>,
) -> Result<()> {
    let mut next = ids.iter().copied();
    let (first, last) = siblings(cos, root, entries, &mut next, targets)?;
    let mut d = Dict::new();
    d.insert("Type", Name::from("Outlines"));
    if let (Some(first), Some(last)) = (first, last) {
        d.insert("First", first);
        d.insert("Last", last);
        let shown: usize = entries.iter().map(|e| 1 + if e.open { e.visible() } else { 0 }).sum();
        d.insert("Count", count(shown));
    }
    cos.set(root, Object::Dict(d))?;
    Ok(())
}

/// Writes `entries` as the children of `parent`, numbering them from `ids`; returns the
/// first and last.
fn siblings(
    cos: &mut CosMut<'_>,
    parent: ObjRef,
    entries: &[OutlineEntry],
    ids: &mut impl Iterator<Item = ObjRef>,
    targets: &Targets<'_>,
) -> Result<(Option<ObjRef>, Option<ObjRef>)> {
    // Each item's number comes before its descendants' (pre-order), so take it first.
    let mut numbered = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(r) = ids.next() else { break };
        let children = siblings(cos, r, &entry.children, ids, targets)?;
        numbered.push((entry, r, children));
    }
    for (i, (entry, r, (first, last))) in numbered.iter().enumerate() {
        let mut d = Dict::new();
        d.insert("Title", PdfString::literal(encode_text_string(&entry.title)));
        d.insert("Parent", parent);
        if let Some(prev) = i.checked_sub(1).and_then(|p| numbered.get(p)) {
            d.insert("Prev", prev.1);
        }
        if let Some(next) = numbered.get(i + 1) {
            d.insert("Next", next.1);
        }
        if let (Some(first), Some(last)) = (first, last) {
            d.insert("First", *first);
            d.insert("Last", *last);
            // Closed: minus the descendants that opening it would show.
            let shown = i64::try_from(entry.visible()).unwrap_or(i64::MAX);
            d.insert("Count", if entry.open { shown } else { -shown });
        }
        if let Some(target) = &entry.target {
            targets.apply(&mut d, target)?;
        }
        if let Some([r, g, b]) = entry.color {
            d.insert("C", vec![r.into(), g.into(), b.into()]);
        }
        let flags = i64::from(entry.italic) | (i64::from(entry.bold) << 1);
        if flags != 0 {
            d.insert("F", flags);
        }
        cos.set(*r, Object::Dict(d))?;
    }
    Ok((numbered.first().map(|n| n.1), numbered.last().map(|n| n.1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_counts_open_descendants_only() {
        let leaf = || OutlineEntry::heading("x");
        let closed = OutlineEntry::heading("c").with_child(leaf()).with_child(leaf());
        let open = OutlineEntry::heading("o")
            .open(true)
            .with_child(closed.clone())
            .with_child(leaf());
        assert_eq!((closed.visible(), open.visible(), open.len()), (2, 2, 5));
    }
}
