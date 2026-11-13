//! The outline ops ([`AddOutlineItem`], [`UpdateOutlineItem`], [`MoveOutlineItem`],
//! [`DeleteOutlineItem`]) and the named-destination ops ([`SetNamedDestination`],
//! [`RenameNamedDestination`], [`DeleteNamedDestination`]): bookmarks and the names links jump
//! to (ISO 32000-2 §12.3.3, §12.3.2.4).
//!
//! The rules every op keeps:
//!
//! * The outline's lists stay whole after each op (`/First`, `/Last`, `/Next`, `/Prev`,
//!   `/Parent`, and each `/Count`: visible descendants for an open item, minus those that
//!   would show when opened for a closed one), through [`laminapdf_model::OutlineTree`]. An
//!   outline read with broken links is rewritten whole by the first op that touches it, and
//!   the op says so ([`Applied::outline_repaired`](crate::Applied::outline_repaired)).
//! * A name stays in the form it was found in: a string in the `/Names /Dests` tree, written
//!   again through [`laminapdf_model::TreeWriter`], or a name object in the catalog's `/Dests`
//!   dictionary. New names go into the tree. Moving a name between the two would break every
//!   use that names it in the other form, since a name object resolves only in `/Dests`.
//! * A web address must be absolute, with the scheme `http`, `https` or `mailto`, and ASCII;
//!   anything else (`javascript:` first) fails with `link-uri-refused`.

mod names;
mod target;
#[cfg(test)]
mod tests;
mod types;

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{OutlineItemId, OutlineTree, OutlineTreeError, SiblingPosition};
use laminapdf_syntax::text::encode_text_string;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;

pub use names::{DeleteNamedDestination, DestinationUse, RenameNamedDestination, SetNamedDestination};
pub use types::{ChildItems, DestinationView, ExplicitDestination, OutlineStyle, OutlineTarget, RenameLinks, Rgb};

/// Adds a bookmark under `parent` (`null`: the top level) at `at`, with its title, target and
/// style, open or closed. The new item's ID is in
/// [`Applied::created_outline_items`](crate::Applied::created_outline_items).
///
/// On the wire: `{ "op": "addOutlineItem", "args": { "parent": null, "at": "last", "title":
/// "Chapter 1", "target": { "destination": { "page": "p3.0", "view": { "kind": "fit" } } } } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct AddOutlineItem {
    /// The parent item; `None` for the top level.
    #[serde(default, with = "wire::opt_outline_item_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string | null", optional))]
    pub parent: Option<OutlineItemId>,
    /// Where among the parent's children.
    #[serde(with = "wire::sibling_position")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = r#""first" | "last" | { before: string } | { after: string }"#)
    )]
    pub at: SiblingPosition,
    /// The title.
    pub title: String,
    /// Where the bookmark goes.
    pub target: OutlineTarget,
    /// Its colour and type style.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub style: OutlineStyle,
    /// Whether its children show (it has none yet; the state holds for when it gets some).
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub open: bool,
}

impl AddOutlineItem {
    /// The op that adds `title`, pointing at `target`, at `at` under `parent`, unstyled and
    /// closed.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(
        parent: Option<OutlineItemId>,
        at: SiblingPosition,
        title: impl Into<String>,
        target: OutlineTarget,
    ) -> Self {
        Self {
            parent,
            at,
            title: title.into(),
            target,
            style: OutlineStyle::default(),
            open: false,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let mut tree = OutlineTree::read(doc)?;
        if let Some(parent) = &self.parent {
            require(&tree, parent)?;
        }
        let mut dict = Dict::new();
        dict.insert("Title", title(&self.title));
        target::write(doc, &mut dict, &self.target)?;
        self.style.write(&mut dict);
        let r = doc.cos_mut().edit().create(Object::Dict(dict))?;
        let id = tree.insert(r, self.parent.as_ref(), &self.at).map_err(tree_error)?;
        tree.set_open(&id, self.open).map_err(tree_error)?;
        finish(
            doc,
            tree,
            Effect {
                created_outline_items: vec![id],
                ..Effect::default()
            },
        )
    }
}

/// Changes a bookmark's title, target, style or open state. An absent field is left alone. A
/// target held as another action (JavaScript, `Launch`, …) is replaced only when `target` is
/// given; the item's other entries are kept.
///
/// On the wire: `{ "op": "updateOutlineItem", "args": { "item": "o12.0", "title": "Results",
/// "open": true } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct UpdateOutlineItem {
    /// The item.
    #[serde(with = "wire::outline_item_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub item: OutlineItemId,
    /// A new title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub title: Option<String>,
    /// A new target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub target: Option<OutlineTarget>,
    /// A new style (colour and type style together).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub style: Option<OutlineStyle>,
    /// Open (children shown) or closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub open: Option<bool>,
}

impl UpdateOutlineItem {
    /// The op that changes nothing yet on `item`; set the fields to change.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(item: OutlineItemId) -> Self {
        Self {
            item,
            title: None,
            target: None,
            style: None,
            open: None,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let mut tree = OutlineTree::read(doc)?;
        let r = require(&tree, &self.item)?;
        let current = doc.resolve(r)?;
        let mut dict = current.as_dict().cloned().unwrap_or_default();
        if let Some(t) = &self.title {
            dict.insert("Title", title(t));
        }
        if let Some(t) = &self.target {
            target::write(doc, &mut dict, t)?;
        }
        if let Some(style) = &self.style {
            style.write(&mut dict);
        }
        if current.as_dict() != Some(&dict) {
            doc.cos_mut().edit().set(r, Object::Dict(dict))?;
        }
        if let Some(open) = self.open {
            tree.set_open(&self.item, open).map_err(tree_error)?;
        }
        finish(doc, tree, Effect::default())
    }
}

/// Moves a bookmark, with its children, under `parent` (`null`: the top level) at `at`. The
/// item keeps its ID.
///
/// On the wire: `{ "op": "moveOutlineItem", "args": { "item": "o12.0", "parent": "o9.0", "at":
/// { "after": "o10.0" } } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct MoveOutlineItem {
    /// The item.
    #[serde(with = "wire::outline_item_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub item: OutlineItemId,
    /// The new parent; `None` for the top level.
    #[serde(default, with = "wire::opt_outline_item_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string | null", optional))]
    pub parent: Option<OutlineItemId>,
    /// Where among the new parent's children.
    #[serde(with = "wire::sibling_position")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = r#""first" | "last" | { before: string } | { after: string }"#)
    )]
    pub at: SiblingPosition,
}

impl MoveOutlineItem {
    /// The op that moves `item` to `at` under `parent`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(item: OutlineItemId, parent: Option<OutlineItemId>, at: SiblingPosition) -> Self {
        Self { item, parent, at }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let mut tree = OutlineTree::read(doc)?;
        require(&tree, &self.item)?;
        tree.move_item(&self.item, self.parent.as_ref(), &self.at)
            .map_err(tree_error)?;
        finish(doc, tree, Effect::default())
    }
}

/// Deletes a bookmark, and its children with it or moved up into its place.
///
/// On the wire: `{ "op": "deleteOutlineItem", "args": { "item": "o12.0", "children":
/// "promote" } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DeleteOutlineItem {
    /// The item.
    #[serde(with = "wire::outline_item_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub item: OutlineItemId,
    /// What happens to its children.
    pub children: ChildItems,
}

impl DeleteOutlineItem {
    /// The op that deletes `item`, doing `children` with its children.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(item: OutlineItemId, children: ChildItems) -> Self {
        Self { item, children }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let mut tree = OutlineTree::read(doc)?;
        require(&tree, &self.item)?;
        tree.remove(&self.item, self.children == ChildItems::Promote)
            .map_err(tree_error)?;
        finish(doc, tree, Effect::default())
    }
}

/// The item's object, refusing an item the tree hasn't and one stored as a direct dictionary,
/// whose ID changes when the tree is written.
fn require(tree: &OutlineTree, id: &OutlineItemId) -> Result<ObjRef> {
    if !tree.contains(id) {
        return Err(Error::UnknownOutlineItem(id.clone()));
    }
    id.as_lamina_id()
        .obj_ref()
        .ok_or_else(|| Error::DirectOutlineItem(id.clone()))
}

/// Writes the tree's links and reports whether they had to be repaired.
fn finish(doc: &mut laminapdf_model::Document, mut tree: OutlineTree, mut effect: Effect) -> Result<Effect> {
    effect.outline_repaired = tree.repaired();
    tree.write(&mut doc.cos_mut().edit())?;
    Ok(effect)
}

fn tree_error(e: OutlineTreeError) -> Error {
    match e {
        OutlineTreeError::NotASibling(id) => Error::NotASibling(id),
        OutlineTreeError::IntoItself(id) => Error::OutlineIntoItself(id),
        OutlineTreeError::UnknownItem(id) => Error::UnknownOutlineItem(id),
    }
}

/// A text string (§7.9.2.2): PDFDocEncoding when it holds the text, else UTF-16BE.
fn title(text: &str) -> Object {
    Object::String(PdfString::literal(encode_text_string(text)))
}
