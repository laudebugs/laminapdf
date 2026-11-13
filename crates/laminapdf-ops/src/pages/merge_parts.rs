//! The document-level parts a merge brings in besides pages ([`MergeDocuments`](super::merge::MergeDocuments)):
//! the source's bookmarks, copied while the copier still maps its pages (ISO 32000-2 §12.3.3);
//! its optional content groups, added to the target's `/OCProperties` (§8.11.4); and the labels
//! its pages showed (§12.4.2).

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{
    Copied, LabelRange, LabelStyle, ObjectCopier, OutlineItem, PageId, PageLabels, set_catalog_entry,
};

use super::cleanup::edit_object;
use super::dests::Dests;
use super::page_ids;
use crate::error::Result;

/// A bookmark copied into the target, not yet in its outline.
pub(super) struct CopiedItem {
    /// The new item dictionary.
    pub(super) item: ObjRef,
    /// Whether it was open in the source.
    pub(super) open: bool,
    /// Its copied children, in order.
    pub(super) children: Vec<CopiedItem>,
}

impl CopiedItem {
    /// These items' objects and their descendants', which nothing reaches until they join the
    /// outline.
    pub(super) fn objects(items: &[Self]) -> Vec<ObjRef> {
        let mut out = Vec::new();
        for item in items {
            out.push(item.item);
            out.extend(Self::objects(&item.children));
        }
        out
    }
}

/// Copies the source's bookmarks into `doc` as new item objects: each keeps its title, colour
/// and style, and its destination or `GoTo` action goes to the copy of its page (named ones
/// renamed as links' are) or its `URI` action comes as it was. A bookmark whose page didn't
/// come along is left out, unless one of its children came, which keeps it without a target.
/// Returns the copied top-level items and how many were left out.
pub(super) fn copy_outline(
    doc: &mut laminapdf_model::Document,
    src: &laminapdf_model::Document,
    dests: &mut Dests<'_, '_>,
) -> Result<(Vec<CopiedItem>, usize)> {
    let outline = src.outline()?;
    let mut dropped = 0;
    let items = copy_items(doc, src, dests, outline.roots(), &mut dropped)?;
    Ok((items, dropped))
}

fn copy_items<'o, 'd: 'o>(
    doc: &mut laminapdf_model::Document,
    src: &laminapdf_model::Document,
    dests: &mut Dests<'_, '_>,
    items: impl Iterator<Item = OutlineItem<'o, 'd>>,
    dropped: &mut usize,
) -> Result<Vec<CopiedItem>> {
    let mut out = Vec::new();
    for item in items {
        let children = copy_items(doc, src, dests, item.children(), dropped)?;
        let source = match item.id().and_then(|id| id.as_lamina_id().obj_ref()) {
            Some(r) => src.resolve(r)?.as_dict().cloned(),
            None => None,
        };
        let Some(source) = source else {
            // An item the copier can't reach (stored inline): its children take its place.
            out.extend(children);
            continue;
        };
        let mut dict = Dict::new();
        for key in ["Title", "C", "F"] {
            if let Some(value) = source.get(key) {
                dict.insert(key, src.deref(value)?.into_owned());
            }
        }
        let wanted = source.contains_key("Dest") || source.contains_key("A");
        let target = match (source.get("Dest"), source.get("A")) {
            (Some(dest), _) => dests.fit_source(doc, dest)?.map(|d| ("Dest", d)),
            (None, Some(action)) => fit_action(doc, src, dests, action)?.map(|a| ("A", a)),
            (None, None) => None,
        };
        if let Some((key, value)) = target {
            dict.insert(key, value);
        } else if wanted && children.is_empty() {
            *dropped += 1;
            continue;
        }
        let r = doc.cos_mut().edit().create(Object::Dict(dict))?;
        out.push(CopiedItem {
            item: r,
            open: item.is_open()?,
            children,
        });
    }
    Ok(out)
}

/// A bookmark's action as the target holds it: a `GoTo` to the copy of its page, a `URI`
/// as it was; `None` for one whose page didn't come along, or any other action, which a
/// merge doesn't bring in.
fn fit_action(
    doc: &laminapdf_model::Document,
    src: &laminapdf_model::Document,
    dests: &mut Dests<'_, '_>,
    action: &Object,
) -> Result<Option<Object>> {
    let Object::Dict(action) = src.deref(action)?.into_owned() else {
        return Ok(None);
    };
    Ok(match action.get_name("S") {
        Some(b"GoTo") => match action.get("D") {
            Some(d) => dests
                .fit_source(doc, d)?
                .map(|d| Object::Dict([("S", Object::name("GoTo")), ("D", d)].into_iter().collect())),
            None => None,
        },
        Some(b"URI") => match action.get("URI") {
            Some(uri) => {
                let uri = src.deref(uri)?.into_owned();
                Some(Object::Dict(
                    [("S", Object::name("URI")), ("URI", uri)].into_iter().collect(),
                ))
            }
            None => None,
        },
        _ => None,
    })
}

/// A source's optional content groups that its copied pages use, in the target.
pub(super) struct SourceLayers {
    /// The copied groups, in the source's `/OCGs` order.
    ocgs: Vec<ObjRef>,
    /// Those the source showed by default.
    on: Vec<ObjRef>,
    /// Those it hid.
    off: Vec<ObjRef>,
    /// Its `/Order` (the layers panel's tree) with only copied groups.
    order: Vec<Object>,
}

/// The source's optional content groups that came along with its pages (the copier copies the
/// groups a page's resources and annotations use), with their default visibility and order;
/// `None` when none came.
pub(super) fn source_layers(
    src: &laminapdf_model::Document,
    copier: &ObjectCopier<'_>,
) -> Result<Option<SourceLayers>> {
    let Some(props) = src.catalog()?.get("OCProperties").cloned() else {
        return Ok(None);
    };
    let Object::Dict(props) = src.deref(&props)?.into_owned() else {
        return Ok(None);
    };
    let copy_of = |r: ObjRef| match copier.copy_of(r) {
        Some(Copied::Object(c)) => Some(c),
        _ => None,
    };
    let groups: Vec<ObjRef> = refs(src, props.get("OCGs"))?;
    let ocgs: Vec<ObjRef> = groups.iter().filter_map(|&g| copy_of(g)).collect();
    if ocgs.is_empty() {
        return Ok(None);
    }
    let config = match props.get("D") {
        Some(d) => src.deref(d)?.into_owned().as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    let off: BTreeSet<ObjRef> = if config.get_name("BaseState") == Some(b"OFF") {
        let on: BTreeSet<ObjRef> = refs(src, config.get("ON"))?.into_iter().collect();
        groups.iter().copied().filter(|g| !on.contains(g)).collect()
    } else {
        refs(src, config.get("OFF"))?.into_iter().collect()
    };
    let (mut on_copies, mut off_copies) = (Vec::new(), Vec::new());
    for &g in &groups {
        if let Some(c) = copy_of(g) {
            if off.contains(&g) {
                off_copies.push(c);
            } else {
                on_copies.push(c);
            }
        }
    }
    let order = match config.get("Order") {
        Some(order) => map_order(src, &src.deref(order)?.into_owned(), &copy_of, 0)?,
        None => ocgs.iter().map(|&g| Object::Ref(g)).collect(),
    };
    Ok(Some(SourceLayers {
        ocgs,
        on: on_copies,
        off: off_copies,
        order,
    }))
}

/// The references in an array (itself perhaps a reference), in order.
fn refs(doc: &laminapdf_model::Document, value: Option<&Object>) -> Result<Vec<ObjRef>> {
    let Some(value) = value else { return Ok(Vec::new()) };
    Ok(match doc.deref(value)?.into_owned() {
        Object::Array(items) => items.iter().filter_map(Object::as_ref).collect(),
        _ => Vec::new(),
    })
}

/// An `/Order` array with each group replaced by its copy, groups that didn't come along left
/// out, and nested arrays (a label string, then groups) kept while they hold a group.
fn map_order(
    src: &laminapdf_model::Document,
    order: &Object,
    copied: &impl Fn(ObjRef) -> Option<ObjRef>,
    depth: usize,
) -> Result<Vec<Object>> {
    let Object::Array(items) = order else {
        return Ok(Vec::new());
    };
    if depth > 32 {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for item in items {
        match item {
            Object::Ref(r) => match src.resolve(*r)?.as_ref() {
                Object::Array(_) => {
                    let inner = map_order(src, &*src.resolve(*r)?, copied, depth + 1)?;
                    if inner.iter().any(|o| !matches!(o, Object::String(_))) {
                        out.push(Object::Array(inner));
                    }
                }
                _ => {
                    if let Some(c) = copied(*r) {
                        out.push(Object::Ref(c));
                    }
                }
            },
            Object::Array(_) => {
                let inner = map_order(src, item, copied, depth + 1)?;
                if inner.iter().any(|o| !matches!(o, Object::String(_))) {
                    out.push(Object::Array(inner));
                }
            }
            Object::String(_) => out.push(item.clone()),
            _ => {}
        }
    }
    Ok(out)
}

/// Adds a source's copied groups to the target's `/OCProperties`: to `/OCGs`, to the default
/// configuration's `/Order` after the target's own, and to its `/ON` or `/OFF` list so each
/// shows as it did in the source.
pub(super) fn merge_layers(doc: &mut laminapdf_model::Document, layers: &SourceLayers) -> Result<()> {
    let entry = doc.catalog()?.get("OCProperties").cloned();
    let mut props = match &entry {
        Some(e) => doc.deref(e)?.into_owned().as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    let mut ocgs: Vec<Object> = match props.get("OCGs") {
        Some(o) => match doc.deref(o)?.into_owned() {
            Object::Array(items) => items,
            _ => Vec::new(),
        },
        None => Vec::new(),
    };
    let had: Vec<Object> = ocgs.clone();
    for &g in &layers.ocgs {
        if !ocgs.contains(&Object::Ref(g)) {
            ocgs.push(Object::Ref(g));
        }
    }
    let config_entry = props.get("D").cloned();
    let mut config = match &config_entry {
        Some(d) => doc.deref(d)?.into_owned().as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    let base_off = config.get_name("BaseState") == Some(b"OFF");
    let (key, add) = if base_off {
        ("ON", &layers.on)
    } else {
        ("OFF", &layers.off)
    };
    if !add.is_empty() {
        let mut list = array(doc, config.get(key))?;
        list.extend(add.iter().map(|&g| Object::Ref(g)));
        config.insert(key, Object::Array(list));
    }
    let mut order = match config.get("Order") {
        Some(o) => array(doc, Some(o))?,
        None => had,
    };
    order.extend(layers.order.iter().cloned());
    config.insert("Order", Object::Array(order));
    match config_entry {
        Some(Object::Ref(r)) => edit_object(doc, r, |o| *o = Object::Dict(config))?,
        _ => {
            props.insert("D", Object::Dict(config));
        }
    }
    props.insert("OCGs", Object::Array(ocgs));
    match entry {
        Some(Object::Ref(r)) => edit_object(doc, r, |o| *o = Object::Dict(props)),
        _ => Ok(set_catalog_entry(
            &mut doc.cos_mut().edit(),
            "OCProperties",
            Some(Object::Dict(props)),
        )?),
    }
}

/// An array value (perhaps a reference), resolved; empty when absent or not an array.
fn array(doc: &laminapdf_model::Document, value: Option<&Object>) -> Result<Vec<Object>> {
    let Some(value) = value else { return Ok(Vec::new()) };
    Ok(match doc.deref(value)?.into_owned() {
        Object::Array(items) => items,
        _ => Vec::new(),
    })
}

/// One page's label: its style, prefix and number (§12.4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Label {
    style: Option<LabelStyle>,
    prefix: String,
    number: u64,
}

impl Label {
    /// The label of page `index` under `labels`; a page before every range, or in a document
    /// without labels, shows its number, `index + 1`.
    fn of(labels: &PageLabels, index: usize) -> Self {
        let i = index as u64;
        match labels.ranges().iter().rev().find(|r| r.start <= i) {
            Some(r) => Self {
                style: r.style,
                prefix: r.prefix.clone(),
                number: r.first.saturating_add(i - r.start),
            },
            None => Self {
                style: Some(LabelStyle::Decimal),
                prefix: String::new(),
                number: i.saturating_add(1),
            },
        }
    }

    /// Whether this label follows `previous` in one range: the same style and prefix, and the
    /// next number (any number for a prefix-only range).
    fn follows(&self, previous: &Self) -> bool {
        self.style == previous.style
            && self.prefix == previous.prefix
            && (self.style.is_none() || self.number == previous.number.saturating_add(1))
    }
}

/// The labels a source's chosen pages showed there.
pub(super) struct SourceLabels {
    /// Whether the source had labels.
    labelled: bool,
    /// Each chosen page's label, in the order the pages were chosen.
    labels: Vec<Label>,
}

impl SourceLabels {
    /// The labels `pages` show in `src`.
    pub(super) fn of(src: &laminapdf_model::Document, pages: &[PageId]) -> Result<Self> {
        let labels = src.page_labels()?;
        let all = page_ids(src)?;
        let index = positions(&all);
        Ok(Self {
            labelled: !labels.is_empty(),
            labels: pages
                .iter()
                .map(|p| Label::of(&labels, index.get(p).copied().unwrap_or(0)))
                .collect(),
        })
    }

    /// The `i`th chosen page's label, and whether its source had labels.
    pub(super) fn get(&self, i: usize) -> Option<(Label, bool)> {
        self.labels.get(i).map(|l| (l.clone(), self.labelled))
    }
}

/// Writes the page labels of the merged document: each page of the target keeps the label it
/// showed (`labels_before` over `before`), each copy the label it showed in its source
/// (`copies`, by the copy's ID), and ranges start wherever the style, the prefix or the run of
/// numbers breaks. Nothing is written when neither the target nor any source had labels.
pub(super) fn merge_labels(
    doc: &mut laminapdf_model::Document,
    labels_before: &PageLabels,
    before: &[PageId],
    after: &[PageId],
    copies: &[(PageId, Option<(Label, bool)>)],
) -> Result<()> {
    let labelled = !labels_before.is_empty() || copies.iter().any(|(_, l)| l.as_ref().is_some_and(|(_, s)| *s));
    if !labelled {
        return Ok(());
    }
    let label_of: BTreeMap<&PageId, &Option<(Label, bool)>> = copies.iter().map(|(id, l)| (id, l)).collect();
    let index = positions(before);
    let mut ranges: Vec<LabelRange> = Vec::new();
    let mut previous: Option<Label> = None;
    for (k, page) in after.iter().enumerate() {
        let label = match label_of.get(page) {
            Some(Some((label, _))) => label.clone(),
            _ => Label::of(labels_before, index.get(page).copied().unwrap_or(k)),
        };
        if previous.as_ref().is_none_or(|p| !label.follows(p)) {
            ranges.push(LabelRange::new(
                k as u64,
                label.style,
                label.prefix.clone(),
                label.number,
            ));
        }
        previous = Some(label);
    }
    let merged = PageLabels::from_ranges(ranges);
    if merged != doc.page_labels()? {
        merged.write(&mut doc.cos_mut().edit())?;
    }
    Ok(())
}

/// Each page's index in `pages`.
fn positions(pages: &[PageId]) -> BTreeMap<&PageId, usize> {
    pages.iter().enumerate().map(|(i, p)| (p, i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(style: Option<LabelStyle>, prefix: &str, number: u64) -> Label {
        Label {
            style,
            prefix: prefix.to_owned(),
            number,
        }
    }

    #[test]
    fn a_page_before_every_range_shows_its_number() {
        let labels = PageLabels::from_ranges(vec![LabelRange::new(2, Some(LabelStyle::UpperRoman), "", 1)]);
        assert_eq!(Label::of(&labels, 1), label(Some(LabelStyle::Decimal), "", 2));
        assert_eq!(Label::of(&labels, 3), label(Some(LabelStyle::UpperRoman), "", 2));
    }

    #[test]
    fn a_label_follows_the_next_number_of_its_style_and_prefix() {
        let one = label(Some(LabelStyle::Decimal), "A-", 1);
        assert!(label(Some(LabelStyle::Decimal), "A-", 2).follows(&one));
        assert!(!label(Some(LabelStyle::Decimal), "A-", 3).follows(&one));
        assert!(!label(Some(LabelStyle::Decimal), "B-", 2).follows(&one));
        assert!(label(None, "Cover", 9).follows(&label(None, "Cover", 1)));
    }
}
