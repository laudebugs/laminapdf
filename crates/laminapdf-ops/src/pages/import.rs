//! Pages copied in from another document (E2 plan §2.2, §11): deep copies through
//! [`ObjectCopier`], then what follows them fitted into the target. Annotations point at their
//! copy (`/P`) and get a new `/NM`; links to pages that didn't come along are removed and
//! reported, and the named destinations the others use come with them, renamed on a clash;
//! form fields join the target's form under their own name, renamed `<partial>#<n>` on a clash
//! or, on request, linked to the target's field of that name; the pages' tags join the target's
//! structure tree or are dropped, as the caller's [`Tags`] says ([`import_tags`]).

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{Copied, FieldId, FieldNames, ObjectCopier, PageId, is_widget, set_catalog_entry};
use laminapdf_syntax::text::decode_text_string;

use super::cleanup::{edit_dict, edit_object};
use super::copies::{FieldChange, FieldOutcome, renew_nm, text};
use super::dests::Dests;
use super::forms::{edit_list, is_signature_field, list_field};
use super::import_tags::{StructureRename, Tags, import_tags};
use crate::class::ModificationClass;
use crate::error::{Error, Result};
use crate::widgets::{TouchedWidget, WidgetChange};

/// What comes along with pages copied from another document.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Carry {
    /// Whether the pages' annotations other than form fields come along (links, comments,
    /// markup); `true` when left out.
    #[serde(default = "yes")]
    pub annotations: bool,
    /// What happens to a form field whose name the target already uses; `rename` when left
    /// out.
    #[serde(default)]
    pub fields: FieldClash,
}

const fn yes() -> bool {
    true
}

impl Default for Carry {
    fn default() -> Self {
        Self {
            annotations: true,
            fields: FieldClash::Rename,
        }
    }
}

impl Carry {
    /// With `annotations` deciding whether annotations other than form fields come along.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn with_annotations(mut self, annotations: bool) -> Self {
        self.annotations = annotations;
        self
    }

    /// With `fields` deciding what happens to a field whose name the target already uses.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn with_fields(mut self, fields: FieldClash) -> Self {
        self.fields = fields;
        self
    }
}

/// What happens to a copied form field whose fully qualified name the target already uses
/// (E2 plan §11).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum FieldClash {
    /// The copy is renamed `<partial>#<n>`, so it keeps a value of its own.
    #[default]
    Rename,
    /// The copy's widgets join the target's field of that name and share its value, when both
    /// fields are of the same type and hold the same value and the target's has its widgets as
    /// kids; otherwise the copy is renamed.
    Link,
}

/// What to do with signature fields on the copied pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Signatures {
    /// Leave their widgets off the copies (a page copied into a document).
    Skip,
    /// Copy them unsigned (a new document made from pages: E2 plan §6).
    Unsign,
}

/// What an import copied and what it changed on the way.
#[derive(Debug, Default)]
pub(crate) struct Imported {
    /// The new page objects, one per source page, in order; not yet in the page tree.
    pub(crate) copies: Vec<ObjRef>,
    /// Fields renamed, linked or left out.
    pub(crate) fields: Vec<FieldChange>,
    /// The copy holding each link removed because its destination didn't come along.
    pub(crate) links_removed: Vec<PageId>,
    /// The copies whose source pages were tagged.
    pub(crate) tags_dropped: Vec<PageId>,
    /// The target's fields that copied widgets joined ([`FieldClash::Link`]): their `/Kids`
    /// changed, which the save planner checks against field locks.
    pub(crate) widgets: Vec<TouchedWidget>,
    /// The structure names that came along under another name.
    pub(crate) structure_renamed: Vec<StructureRename>,
    /// The source's named destinations that came along under another name: (source name,
    /// name in the target).
    pub(crate) destinations_renamed: Vec<(Vec<u8>, Vec<u8>)>,
    /// The copies brought into a file that claims PDF/A from a source that doesn't make the same
    /// claim: nothing checks what they bring (fonts, colour spaces) against it.
    pub(crate) pdfa_unchecked: Vec<PageId>,
}

/// Copies `pages` (each once, in the order given) of `src` into `doc` as new page objects,
/// not yet in the page tree. Field names in use are in `names`, which learns the new ones.
pub(crate) fn import(
    doc: &mut laminapdf_model::Document,
    src: &laminapdf_model::Document,
    pages: &[PageId],
    carry: Carry,
    signatures: Signatures,
    tags: &Tags,
    names: &mut FieldNames,
) -> Result<Imported> {
    import_with(doc, src, pages, (carry, signatures, tags), names, |_, _, _| {
        Ok(Vec::new())
    })
}

/// [`import`], then `after` while the copier and the destinations are still at hand, before
/// the named destinations are written: a merge copies the source's outline there, so its items
/// go to the copies and share the links' renamed destinations. `after` returns the objects it
/// created that nothing reaches yet, which the sweep of unreached copies then keeps.
pub(crate) fn import_with<F>(
    doc: &mut laminapdf_model::Document,
    src: &laminapdf_model::Document,
    pages: &[PageId],
    (carry, signatures, tags): (Carry, Signatures, &Tags),
    names: &mut FieldNames,
    after: F,
) -> Result<Imported>
where
    F: for<'s, 'c> FnOnce(
        &mut laminapdf_model::Document,
        &'c ObjectCopier<'s>,
        &mut Dests<'s, 'c>,
    ) -> Result<Vec<ObjRef>>,
{
    let tree = src.pages()?;
    let mut seen = BTreeSet::new();
    let mut chosen = Vec::with_capacity(pages.len());
    for id in pages {
        if seen.insert(id) {
            chosen.push(tree.by_id(id).ok_or_else(|| Error::UnknownPage(id.clone()))?);
        }
    }
    if chosen.is_empty() {
        return Ok(Imported::default());
    }
    let cos_pages: Vec<&laminapdf_cos::Page> = chosen.iter().map(laminapdf_model::Page::cos).collect();
    let first_new = doc.next_number();
    let mut copy = ObjectCopier::new(src, *doc.limits());
    let copies = copy.copy_pages(&mut doc.cos_mut().edit(), &cos_pages)?;
    let mut out = Imported {
        copies: copies.clone(),
        ..Imported::default()
    };
    let sources: Vec<PageId> = chosen.iter().map(|p| p.id().clone()).collect();
    let removed = fit_fields(
        doc,
        src,
        &copy,
        (&sources, &copies),
        (carry.fields, signatures),
        names,
        &mut out,
    )?;
    if !out.fields.is_empty() || doc.catalog()?.contains_key("AcroForm") {
        fit_form(doc, src, &mut copy)?;
    }
    let mut dests = Dests::new(src, &copy)?;
    for &copy in &copies {
        fit_page(doc, copy, carry, &removed, &mut dests, &mut out)?;
    }
    let kept = after(doc, &copy, &mut dests)?;
    out.destinations_renamed = dests.renamed();
    dests.write(doc)?;
    let tagged = import_tags(doc, src, &mut copy, (first_new, &copies), tags)?;
    out.tags_dropped = tagged.dropped;
    out.structure_renamed = tagged.renamed;
    sweep(doc, first_new, &[copies.as_slice(), kept.as_slice()].concat())?;
    raise_version(doc, src)?;
    if let Some(claim) = pdfa_claim(doc)?
        && pdfa_claim(src)? != Some(claim)
    {
        out.pdfa_unchecked = copies.iter().map(|&c| PageId::object(c)).collect();
    }
    Ok(out)
}

/// The PDF/A part and conformance level `doc`'s XMP metadata claims, when it claims one.
fn pdfa_claim(doc: &laminapdf_model::Document) -> Result<Option<(u8, Option<String>)>> {
    Ok(match doc.xmp()? {
        laminapdf_model::xmp::PacketState::Parsed(packet) => packet
            .pdfa_part()
            .map(|part| (part, packet.pdfa_conformance().map(|c| c.to_ascii_uppercase()))),
        _ => None,
    })
}

/// Gives the target's form the source form's default appearance (`/DA`) when it has none, and
/// the source's default resources (`/DR`) under names the target's lack: a copied field with
/// variable text needs both to draw its value again (ISO 32000-2 §12.7.4.3). A source form
/// that asks viewers to draw its fields (`/NeedAppearances true`) makes the target's ask too.
fn fit_form(
    doc: &mut laminapdf_model::Document,
    src: &laminapdf_model::Document,
    copy: &mut ObjectCopier<'_>,
) -> Result<()> {
    let Some(source_form) = form_of(src)? else {
        return Ok(());
    };
    let Some(target_form) = form_of(doc)? else {
        return Ok(());
    };
    let mut form = target_form.clone();
    // Fields whose appearances the source left to the viewer still need them drawn here.
    if let Some(need) = source_form.get("NeedAppearances")
        && matches!(*src.deref(need)?, Object::Bool(true))
    {
        form.insert("NeedAppearances", Object::Bool(true));
    }
    if !form.contains_key("DA")
        && let Some(da) = source_form.get("DA")
    {
        form.insert("DA", src.deref(da)?.into_owned());
    }
    if let Some(source_dr) = source_form
        .get("DR")
        .map(|dr| src.deref(dr))
        .transpose()?
        .map(laminapdf_cos::Resolved::into_owned)
        && let Object::Dict(source_dr) = source_dr
    {
        let mut dr = match form.get("DR").map(|dr| doc.deref(dr)).transpose()? {
            Some(dr) => dr.into_owned().as_dict().cloned().unwrap_or_default(),
            None => Dict::new(),
        };
        for (category, entries) in source_dr.iter() {
            let Object::Dict(entries) = src.deref(entries)?.into_owned() else {
                continue;
            };
            let mut ours = match dr
                .iter()
                .find(|(k, _)| *k == category)
                .map(|(_, e)| doc.deref(e))
                .transpose()?
            {
                Some(e) => e.into_owned().as_dict().cloned().unwrap_or_default(),
                None => Dict::new(),
            };
            for (name, value) in entries.iter() {
                if !ours.iter().any(|(k, _)| k == name) {
                    let copied = copy.copy_value(&mut doc.cos_mut().edit(), value)?;
                    ours.insert(name.clone(), copied);
                }
            }
            dr.insert(category.clone(), Object::Dict(ours));
        }
        form.insert("DR", Object::Dict(dr));
    }
    if form == target_form {
        return Ok(());
    }
    match doc.catalog()?.get("AcroForm").cloned() {
        Some(Object::Ref(r)) => edit_object(doc, r, |o| *o = Object::Dict(form)),
        _ => Ok(set_catalog_entry(
            &mut doc.cos_mut().edit(),
            "AcroForm",
            Some(Object::Dict(form)),
        )?),
    }
}

/// `doc`'s interactive form dictionary, resolved.
fn form_of(doc: &laminapdf_model::Document) -> Result<Option<Dict>> {
    let Some(form) = doc.catalog()?.get("AcroForm").cloned() else {
        return Ok(None);
    };
    Ok(doc.deref(&form)?.into_owned().as_dict().cloned())
}

/// Deletes the objects this import created that nothing reaches any more (annotations, fields
/// and signatures left off, links removed), so a save never writes what the caller left out,
///.
fn sweep(doc: &mut laminapdf_model::Document, first_new: u64, copies: &[ObjRef]) -> Result<()> {
    let created: BTreeSet<u32> = doc
        .overlay()
        .iter()
        .map(|(num, _)| num)
        .filter(|&num| u64::from(num) >= first_new)
        .collect();
    let mut queue: Vec<u32> = copies.iter().map(|r| r.num).collect();
    // What the target's own objects (the form, the name tree, the page tree) point at.
    for (num, entry) in doc.overlay().iter() {
        if created.contains(&num) {
            continue;
        }
        if let Some(object) = entry.object() {
            refs_in(object, 0, &mut |r| queue.push(r.num));
        }
    }
    if let Some(trailer) = doc.overlay().trailer() {
        refs_in(&Object::Dict(trailer.clone()), 0, &mut |r| queue.push(r.num));
    }
    let mut reached = BTreeSet::new();
    while let Some(num) = queue.pop() {
        if !created.contains(&num) || !reached.insert(num) {
            continue;
        }
        let object = doc.object(num)?;
        refs_in(&object, 0, &mut |r| queue.push(r.num));
    }
    for &num in &created {
        if !reached.contains(&num) {
            doc.cos_mut().edit().delete(ObjRef::new(num, 0))?;
        }
    }
    Ok(())
}

/// Calls `f` with every reference in `object`, `depth` levels down at most 64.
fn refs_in(object: &Object, depth: usize, f: &mut impl FnMut(ObjRef)) {
    if depth > 64 {
        return;
    }
    match object {
        Object::Ref(r) => f(*r),
        Object::Array(items) => {
            for item in items {
                refs_in(item, depth + 1, f);
            }
        }
        Object::Dict(dict) => {
            for (_, value) in dict.iter() {
                refs_in(value, depth + 1, f);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                refs_in(value, depth + 1, f);
            }
        }
        _ => {}
    }
}

/// Fits the copies' form fields into `doc`'s form; returns the copied widgets left off the
/// pages (signatures, with [`Signatures::Skip`]).
fn fit_fields(
    doc: &mut laminapdf_model::Document,
    src: &laminapdf_model::Document,
    copy: &ObjectCopier<'_>,
    (sources, pages): (&[PageId], &[ObjRef]),
    (clash, signatures): (FieldClash, Signatures),
    names: &mut FieldNames,
    out: &mut Imported,
) -> Result<BTreeSet<ObjRef>> {
    let mut removed = BTreeSet::new();
    let mut not_copied = BTreeSet::new();
    let mut roots: BTreeMap<ObjRef, PageId> = BTreeMap::new();
    for (source, &page_copy) in sources.iter().zip(pages) {
        let page = PageId::object(page_copy);
        for w in src.page_widgets(source)? {
            let Some(Copied::Object(widget)) = copy.copy_of(w.widget) else {
                continue;
            };
            if is_signature_field(src, w.field)? {
                match signatures {
                    Signatures::Skip => {
                        removed.insert(widget);
                        if not_copied.insert(w.field) {
                            out.fields.push(FieldChange {
                                field: FieldId::object(w.field),
                                page: page.clone(),
                                outcome: FieldOutcome::NotCopied,
                            });
                        }
                        continue;
                    }
                    Signatures::Unsign => {
                        if let Some(Copied::Object(field)) = copy.copy_of(w.field) {
                            edit_dict(doc, field, |d| {
                                d.remove("V");
                            })?;
                        }
                    }
                }
            }
            roots.entry(root_of(src, w.field)?).or_insert_with(|| page.clone());
        }
    }
    for (root_source, page) in roots {
        let Some(Copied::Object(root)) = copy.copy_of(root_source) else {
            continue;
        };
        if !prune(doc, root, &removed, 0)? {
            continue;
        }
        let Some(name) = partial_name(doc, root)? else {
            list_field(doc, None, root)?;
            continue;
        };
        if !names.clashes(&name) {
            names.insert(name);
            list_field(doc, None, root)?;
            continue;
        }
        if clash == FieldClash::Link
            && let Some(existing) = linkable(doc, root, &name)?
        {
            link(doc, root, existing)?;
            out.widgets.push(TouchedWidget::new(
                FieldId::object(existing),
                WidgetChange::Reshown,
                ModificationClass::PageStructure,
            ));
            out.fields.push(FieldChange {
                field: FieldId::object(root_source),
                page,
                outcome: FieldOutcome::Linked {
                    to: FieldId::object(existing),
                },
            });
            continue;
        }
        let renamed = names.unique_partial(None, &name);
        edit_dict(doc, root, |d| {
            d.insert("T", text(&renamed));
        })?;
        list_field(doc, None, root)?;
        out.fields.push(FieldChange {
            field: FieldId::object(root_source),
            page,
            outcome: FieldOutcome::Renamed {
                to: FieldId::object(root),
                name: renamed,
            },
        });
    }
    Ok(removed)
}

/// The top of `field`'s `/Parent` chain in `doc`.
fn root_of(doc: &laminapdf_model::Document, field: ObjRef) -> Result<ObjRef> {
    let max_depth = doc.limits().max_tree_depth();
    let mut at = field;
    let mut seen = BTreeSet::new();
    while seen.len() < max_depth && seen.insert(at.num) {
        match doc.object(at.num)?.as_dict().and_then(|d| d.get("Parent")) {
            Some(Object::Ref(parent)) => at = *parent,
            _ => break,
        }
    }
    Ok(at)
}

/// Keeps under the copied field `node` only what the copied pages show: kids that came along
/// (the copier writes the others as `null`) and aren't in `removed`. Returns whether `node`
/// still shows a widget.
fn prune(doc: &mut laminapdf_model::Document, node: ObjRef, removed: &BTreeSet<ObjRef>, depth: usize) -> Result<bool> {
    if removed.contains(&node) || depth >= doc.limits().max_tree_depth() {
        return Ok(false);
    }
    let Some(dict) = doc.object(node.num)?.as_dict().cloned() else {
        return Ok(false);
    };
    let kids = match dict.get("Kids") {
        Some(Object::Array(kids)) => kids.clone(),
        Some(Object::Ref(r)) => doc
            .object(r.num)?
            .as_array()
            .map(<[Object]>::to_vec)
            .unwrap_or_default(),
        _ => return Ok(is_widget(&dict)),
    };
    let mut kept = Vec::with_capacity(kids.len());
    for kid in kids {
        if let Object::Ref(r) = kid
            && prune(doc, r, removed, depth + 1)?
        {
            kept.push(Object::Ref(r));
        }
    }
    if kept.is_empty() {
        return Ok(false);
    }
    edit_dict(doc, node, |d| {
        d.insert("Kids", Object::Array(kept));
    })?;
    Ok(true)
}

/// `field`'s partial name (`/T`), if it has one.
fn partial_name(doc: &laminapdf_model::Document, field: ObjRef) -> Result<Option<String>> {
    Ok(match doc.object(field.num)?.as_dict().and_then(|d| d.get("T")) {
        Some(Object::String(t)) => Some(decode_text_string(&t.bytes)),
        _ => None,
    })
}

/// The target's field named `name` that the copied field `root` can join: both terminal, of
/// the same type and value, the target's with its widgets as kids.
fn linkable(doc: &laminapdf_model::Document, root: ObjRef, name: &str) -> Result<Option<ObjRef>> {
    let Some(existing) = doc.field_by_name(name)? else {
        return Ok(None);
    };
    let has_widget_kids = |r: ObjRef| -> Result<Option<bool>> {
        let Some(dict) = doc.object(r.num)?.as_dict().cloned() else {
            return Ok(None);
        };
        let kids = match dict.get("Kids").map(|k| doc.deref(k)).transpose()?.as_deref() {
            Some(Object::Array(kids)) => kids.clone(),
            _ => return Ok(Some(false)),
        };
        for kid in kids {
            let Some(r) = kid.as_ref() else { continue };
            if doc.object(r.num)?.as_dict().is_none_or(|d| d.contains_key("T")) {
                return Ok(None);
            }
        }
        Ok(Some(true))
    };
    if has_widget_kids(existing)? != Some(true)
        || has_widget_kids(root)?.is_none()
        || inherited(doc, root, "FT")? != inherited(doc, existing, "FT")?
        || inherited(doc, root, "V")? != inherited(doc, existing, "V")?
    {
        return Ok(None);
    }
    Ok(Some(existing))
}

/// `key` of field `field`, or of the nearest ancestor that has it, resolved.
fn inherited(doc: &laminapdf_model::Document, field: ObjRef, key: &str) -> Result<Option<Object>> {
    let max_depth = doc.limits().max_tree_depth();
    let mut at = Some(field);
    let mut seen = BTreeSet::new();
    while let Some(r) = at {
        if seen.len() >= max_depth || !seen.insert(r.num) {
            break;
        }
        let Some(dict) = doc.object(r.num)?.as_dict().cloned() else {
            break;
        };
        if let Some(value) = dict.get(key) {
            return Ok(match doc.deref(value)?.into_owned() {
                Object::Null => None,
                value => Some(value),
            });
        }
        at = dict.get("Parent").and_then(Object::as_ref);
    }
    Ok(None)
}

/// The keys of a field dictionary, taken off a widget merged with its field when it joins
/// another field (ISO 32000-2 §12.7.4, Table 226, and the variable-text keys it may keep).
const FIELD_KEYS: [&str; 11] = ["T", "TU", "TM", "FT", "Ff", "V", "DV", "Opt", "MaxLen", "Lock", "SV"];

/// Makes the widgets of copied field `root` kids of the target's field `existing`.
fn link(doc: &mut laminapdf_model::Document, root: ObjRef, existing: ObjRef) -> Result<()> {
    let dict = doc.object(root.num)?.as_dict().cloned().unwrap_or_default();
    let widgets: Vec<ObjRef> = if let Some(Object::Array(kids)) = dict.get("Kids") {
        kids.iter().filter_map(Object::as_ref).collect()
    } else {
        edit_dict(doc, root, |d| {
            for key in FIELD_KEYS {
                d.remove(key);
            }
        })?;
        vec![root]
    };
    for &widget in &widgets {
        edit_dict(doc, widget, |d| {
            d.insert("Parent", Object::Ref(existing));
        })?;
    }
    edit_list(doc, existing, "Kids", |items| {
        items.extend(widgets.iter().map(|&w| Object::Ref(w)));
    })
}

/// What a copied page's annotations need: `/P`, a new `/NM`; links whose destination didn't
/// come along removed. The page loses its article beads (`/B`, which belong to threads the
/// target doesn't have). Structure keys stay for [`import_tags`] to renumber or remove.
fn fit_page(
    doc: &mut laminapdf_model::Document,
    copy: ObjRef,
    carry: Carry,
    removed: &BTreeSet<ObjRef>,
    dests: &mut Dests<'_, '_>,
    out: &mut Imported,
) -> Result<()> {
    let page = PageId::object(copy);
    let Some(mut dict) = doc.object(copy.num)?.as_dict().cloned() else {
        return Ok(());
    };
    dict.remove("B");
    dests.fit_triggers(doc, &mut dict)?;
    let annots = match dict.get("Annots") {
        Some(Object::Array(items)) => items.clone(),
        Some(Object::Ref(r)) => doc
            .object(r.num)?
            .as_array()
            .map(<[Object]>::to_vec)
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let mut list = Vec::with_capacity(annots.len());
    for item in annots {
        let (held, mut annot) = match item {
            Object::Ref(r) if !removed.contains(&r) => match doc.object(r.num)?.as_dict() {
                Some(d) => (Some(r), d.clone()),
                None => continue,
            },
            Object::Dict(d) => (None, d),
            _ => continue,
        };
        if !is_widget(&annot) && !carry.annotations {
            continue;
        }
        annot.insert("P", Object::Ref(copy));
        renew_nm(&mut annot, held.map_or(copy.num, |r| r.num));
        let is_link = annot.get_name("Subtype") == Some(b"Link");
        if is_link && !dests.fit_link(doc, &mut annot)? {
            out.links_removed.push(page.clone());
            continue;
        }
        if !is_link && let Some(action) = annot.get("A").cloned() {
            match dests.fit_action(doc, action, 0)? {
                Some(action) => annot.insert("A", action),
                None => {
                    annot.remove("A");
                }
            }
        }
        dests.fit_triggers(doc, &mut annot)?;
        match held {
            Some(r) => {
                doc.cos_mut().edit().set(r, Object::Dict(annot))?;
                list.push(Object::Ref(r));
            }
            None => list.push(Object::Dict(annot)),
        }
    }
    if list.is_empty() {
        dict.remove("Annots");
    } else {
        dict.insert("Annots", Object::Array(list));
    }
    doc.cos_mut().edit().set(copy, Object::Dict(dict))?;
    Ok(())
}

/// Raises `doc`'s version to `src`'s when the source's is newer (ISO 32000-2 §7.7.2).
fn raise_version(doc: &mut laminapdf_model::Document, src: &laminapdf_model::Document) -> Result<()> {
    let parse = |v: &str| -> Option<(u32, u32)> {
        let (major, minor) = v.split_once('.')?;
        Some((major.parse().ok()?, minor.parse().ok()?))
    };
    let (Some(theirs), ours) = (src.version()?, doc.version()?) else {
        return Ok(());
    };
    let newer = match (parse(&theirs), ours.as_deref().and_then(parse)) {
        (Some(t), Some(o)) => t > o,
        (Some(_), None) => true,
        _ => false,
    };
    if newer {
        set_catalog_entry(&mut doc.cos_mut().edit(), "Version", Some(Object::name(&theirs)))?;
    }
    Ok(())
}
