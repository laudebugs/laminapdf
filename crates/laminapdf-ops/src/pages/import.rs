//! Pages copied in from another document (E2 plan §2.2, §11): deep copies through
//! [`ObjectCopier`], then what follows them fitted into the target. Annotations point at their
//! copy (`/P`) and get a new `/NM`; links to pages that didn't come along are removed and
//! reported, and the named destinations the others use come with them, renamed on a clash;
//! form fields join the target's form under their own name, renamed `<partial>#<n>` on a clash
//! or, on request, linked to the target's field of that name; the pages lose their tags
//! (`/StructParents`, ISO 32000-2 §14.7.5.4) until the structure trees can be merged.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{Copied, FieldId, FieldNames, ObjectCopier, PageId, TreeWriter, set_catalog_entry};
use laminapdf_syntax::text::{decode_text_string, encode_text_string};

use super::cleanup::{edit_dict, edit_object};
use super::copies::{FieldChange, FieldOutcome};
use super::forms::{edit_list, is_signature_field, list_field};
use crate::error::{Error, Result};

/// What comes along with pages copied from another document.
///
/// Added in 0.2.0.
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
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_annotations(mut self, annotations: bool) -> Self {
        self.annotations = annotations;
        self
    }

    /// With `fields` deciding what happens to a field whose name the target already uses.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_fields(mut self, fields: FieldClash) -> Self {
        self.fields = fields;
        self
    }
}

/// What happens to a copied form field whose fully qualified name the target already uses
/// (E2 plan §11).
///
/// Added in 0.2.0.
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
}

/// Copies `pages` (each once, in the order given) of `src` into `doc` as new page objects,
/// not yet in the page tree. Field names in use are in `names`, which learns the new ones.
pub(crate) fn import(
    doc: &mut laminapdf_model::Document,
    src: &laminapdf_model::Document,
    pages: &[PageId],
    carry: Carry,
    signatures: Signatures,
    names: &mut FieldNames,
) -> Result<Imported> {
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
    let mut dests = Dests::new(src, &copy)?;
    for &copy in &copies {
        fit_page(doc, copy, carry, &removed, &mut dests, &mut out)?;
    }
    dests.write(doc)?;
    raise_version(doc, src)?;
    Ok(out)
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

fn is_widget(dict: &Dict) -> bool {
    dict.get_name("Subtype") == Some(b"Widget")
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

/// What a copied page's annotations need: `/P`, a new `/NM`, no `/StructParent`; links whose
/// destination didn't come along removed. The page loses `/StructParents` and its article
/// beads (`/B`, which belong to threads the target doesn't have).
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
    let mut tagged = dict.remove("StructParents").is_some();
    dict.remove("B");
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
        tagged |= annot.remove("StructParent").is_some();
        annot.insert("P", Object::Ref(copy));
        if let Some(Object::String(nm)) = annot.get("NM") {
            let name = format!("{}-{}", decode_text_string(&nm.bytes), held.map_or(copy.num, |r| r.num));
            annot.insert("NM", text(&name));
        }
        let is_link = annot.get_name("Subtype") == Some(b"Link");
        if is_link && !dests.fit_link(doc, &mut annot)? {
            out.links_removed.push(page.clone());
            continue;
        }
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
    if tagged {
        out.tags_dropped.push(page);
    }
    Ok(())
}

/// The named destinations copied links use: read from the source, written to the target once.
struct Dests<'s, 'c> {
    outline: laminapdf_model::Outline<'s>,
    copier: &'c ObjectCopier<'s>,
    /// Each source name already settled → its name in the target (`None`: no copied page).
    settled: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    /// The destinations to add to the target, by name.
    added: BTreeMap<Vec<u8>, Object>,
    /// The names the target already uses, read when first needed.
    taken: Option<BTreeSet<Vec<u8>>>,
}

impl<'s, 'c> Dests<'s, 'c> {
    fn new(src: &'s laminapdf_model::Document, copier: &'c ObjectCopier<'s>) -> Result<Self> {
        let outline = src.outline()?;
        Ok(Self {
            outline,
            copier,
            settled: BTreeMap::new(),
            added: BTreeMap::new(),
            taken: None,
        })
    }

    /// Fits a copied link's destination (`/Dest`, or a `GoTo` action's `/D`); `false` when it
    /// names a page that didn't come along, so the link goes.
    fn fit_link(&mut self, doc: &mut laminapdf_model::Document, link: &mut Dict) -> Result<bool> {
        if let Some(dest) = link.get("Dest").cloned() {
            return Ok(match self.fit(doc, dest)? {
                Some(dest) => {
                    link.insert("Dest", dest);
                    true
                }
                None => false,
            });
        }
        let (held, mut action) = match link.get("A") {
            Some(Object::Dict(a)) => (None, a.clone()),
            Some(Object::Ref(r)) => match doc.object(r.num)?.as_dict() {
                Some(a) => (Some(*r), a.clone()),
                None => return Ok(true),
            },
            _ => return Ok(true),
        };
        let is_goto = action.get_name("S") == Some(b"GoTo");
        let Some(dest) = action.get("D").cloned().filter(|_| is_goto) else {
            return Ok(true);
        };
        let Some(dest) = self.fit(doc, dest)? else {
            return Ok(false);
        };
        action.insert("D", dest);
        match held {
            Some(r) => edit_object(doc, r, |o| *o = Object::Dict(action))?,
            None => {
                link.insert("A", Object::Dict(action));
            }
        }
        Ok(true)
    }

    /// The copied destination `dest` as the target should hold it: an explicit one kept when
    /// its page came along (the copier wrote the others' page as `null`), a named one renamed
    /// to its copy in the target; `None` when its page didn't come along.
    fn fit(&mut self, doc: &laminapdf_model::Document, dest: Object) -> Result<Option<Object>> {
        let explicit = match &dest {
            Object::Ref(r) => (*doc.object(r.num)?).clone(),
            other => other.clone(),
        };
        match explicit {
            Object::Array(items) => Ok(match items.first() {
                Some(Object::Null) | None => None,
                Some(_) => Some(dest),
            }),
            Object::Dict(d) => match d.get("D").cloned() {
                Some(inner) => self.fit(doc, inner),
                None => Ok(Some(dest)),
            },
            Object::Name(n) => Ok(self
                .named(doc, n.as_bytes())?
                .map(|n| Object::Name(laminapdf_syntax::Name::new(n)))),
            Object::String(s) => Ok(self
                .named(doc, &s.bytes)?
                .map(|n| Object::String(PdfString::literal(n)))),
            _ => Ok(Some(dest)),
        }
    }

    /// The name the target gives the source's named destination `name`, copying it on first
    /// use; `None` when it names no page that came along.
    fn named(&mut self, doc: &laminapdf_model::Document, name: &[u8]) -> Result<Option<Vec<u8>>> {
        if let Some(settled) = self.settled.get(name) {
            return Ok(settled.clone());
        }
        let copied = match self.outline.named_destination_value(name)? {
            Some(Object::Array(mut items)) => match items.first().and_then(Object::as_ref) {
                Some(page) => match self.copier.copy_of(page) {
                    Some(Copied::Object(copy)) => {
                        if let Some(first) = items.first_mut() {
                            *first = Object::Ref(copy);
                        }
                        Some(Object::Array(items))
                    }
                    _ => None,
                },
                None => None,
            },
            _ => None,
        };
        let Some(value) = copied else {
            self.settled.insert(name.to_vec(), None);
            return Ok(None);
        };
        let taken = match &mut self.taken {
            Some(taken) => taken,
            slot => slot.insert(destination_names(doc)?),
        };
        let mut chosen = name.to_vec();
        let mut n: u32 = 2;
        while taken.contains(&chosen) {
            chosen = [name, format!("#{n}").as_bytes()].concat();
            n += 1;
        }
        taken.insert(chosen.clone());
        self.added.insert(chosen.clone(), value);
        self.settled.insert(name.to_vec(), Some(chosen.clone()));
        Ok(Some(chosen))
    }

    /// Adds the copied named destinations to the target's `/Names /Dests` tree.
    fn write(self, doc: &mut laminapdf_model::Document) -> Result<()> {
        if self.added.is_empty() {
            return Ok(());
        }
        let mut entries: BTreeMap<Vec<u8>, Object> = match doc.name_tree("Dests")? {
            Some(tree) => tree.entries()?.into_iter().collect(),
            None => BTreeMap::new(),
        };
        entries.extend(self.added);
        let root = TreeWriter::new().write_names(&mut doc.cos_mut().edit(), &entries)?;
        match doc.catalog()?.get("Names").cloned() {
            Some(Object::Ref(r)) => edit_dict(doc, r, |d| {
                d.insert("Dests", Object::Ref(root));
            }),
            Some(Object::Dict(mut names)) => {
                names.insert("Dests", Object::Ref(root));
                Ok(set_catalog_entry(
                    &mut doc.cos_mut().edit(),
                    "Names",
                    Some(Object::Dict(names)),
                )?)
            }
            _ => {
                let names: Dict = [("Dests", Object::Ref(root))].into_iter().collect();
                Ok(set_catalog_entry(
                    &mut doc.cos_mut().edit(),
                    "Names",
                    Some(Object::Dict(names)),
                )?)
            }
        }
    }
}

/// The names of `doc`'s named destinations: the `/Names /Dests` tree and the PDF 1.1 `/Dests`
/// dictionary.
fn destination_names(doc: &laminapdf_model::Document) -> Result<BTreeSet<Vec<u8>>> {
    let mut names: BTreeSet<Vec<u8>> = match doc.name_tree("Dests")? {
        Some(tree) => tree.entries()?.into_iter().map(|(k, _)| k).collect(),
        None => BTreeSet::new(),
    };
    if let Some(dests) = doc.catalog()?.get("Dests").cloned()
        && let Object::Dict(dests) = doc.deref(&dests)?.into_owned()
    {
        names.extend(dests.iter().map(|(k, _)| k.as_bytes().to_vec()));
    }
    Ok(names)
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

/// `s` as a text string (PDFDocEncoding, else UTF-16BE).
fn text(s: &str) -> Object {
    Object::String(PdfString::literal(encode_text_string(s)))
}
