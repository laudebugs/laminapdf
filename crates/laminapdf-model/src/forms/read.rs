//! Reading the form: the walk of `/AcroForm /Fields` with the values each field inherits
//! (ISO 32000-2 §12.7.4.1, Tables 226–228), then the pages' widgets it didn't reach.
//!
//! Reading is tolerant (ADR 0003): a node that can't be read is skipped, the walk stops at
//! `Limits::max_tree_depth` and visits each object once, a value of the wrong type counts as
//! absent, and a widget `/Fields` lists in place of its field stands for that field (`roots`).

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object, Result, absent_if_malformed};
use laminapdf_syntax::text::decode_text_string;

use super::{ChoiceOption, FieldFlags, FieldKind, FieldValue, Form, OrphanWidget, TerminalField, Widget};
use crate::annot::AnnotFlags;
use crate::document::Document;
use crate::id::{AnnotId, FieldId, PageId};
use crate::page::read_rect;
use crate::values::{array_items, dict_at, name_bytes_text, read_name, text_of};

/// What a field inherits from the fields above it (§12.7.4.1), and the defaults the form sets
/// (`/DA`, `/Q`, Table 224).
#[derive(Debug, Clone, Default)]
struct Inherited {
    name: Option<String>,
    ft: Option<Vec<u8>>,
    ff: Option<u32>,
    v: Option<Object>,
    dv: Option<Object>,
    da: Option<String>,
    q: Option<u8>,
    max_len: Option<u32>,
}

/// A node on the walk.
struct Node {
    /// The field or widget object.
    object: ObjRef,
    /// What it inherits.
    inherited: Inherited,
    /// The node whose `/Kids` listed it; `None` for an entry of `/Fields`.
    listed_by: Option<ObjRef>,
    /// Its depth below `/Fields`.
    depth: usize,
}

/// A terminal field met on the walk, before shared names are merged.
struct Terminal {
    field: TerminalField,
    typed: bool,
}

impl Document {
    /// The interactive form ([`Form`]): every terminal field of `/AcroForm /Fields` with what it
    /// inherits, its widgets and their pages, and what the tree and the pages disagree on.
    ///
    /// # Errors
    /// Unavailable input, hard limits and strict-mode refusals reading the catalog, the field
    /// tree or the pages.
    ///
    /// Added in 0.6.0.
    pub fn form(&self) -> Result<Form> {
        let pages = widget_pages(self)?;
        let mut form = Form::default();
        let acro = match self.catalog()?.get("AcroForm") {
            Some(v) => self.read(v)?.and_then(|o| o.as_dict().cloned()),
            None => None,
        };
        let mut reached = BTreeSet::new();
        if let Some(acro) = &acro {
            form.need_appearances = matches!(
                acro.get("NeedAppearances")
                    .map(|v| self.read(v))
                    .transpose()?
                    .flatten()
                    .as_deref(),
                Some(Object::Bool(true))
            );
            form.has_xfa = acro.contains_key("XFA");
            let defaults = Inherited {
                da: text(self, acro.get("DA"))?,
                q: quadding(self, acro.get("Q"))?,
                ..Inherited::default()
            };
            let terminals = walk(self, acro, &defaults, &pages, &mut form, &mut reached)?;
            merge_shared(terminals, &mut form);
        }
        for (page, widgets) in &pages.by_page {
            for &(widget, field) in widgets {
                if !reached.contains(&widget.num) {
                    form.orphan_widgets.push(OrphanWidget {
                        widget: AnnotId::object(widget),
                        page: page.clone(),
                        field: (field != widget).then(|| FieldId::object(field)),
                    });
                }
            }
        }
        Ok(form)
    }
}

/// The page each widget is on, and each page's widgets with their fields, from the pages'
/// `/Annots`.
#[derive(Default)]
struct WidgetPages {
    page_of: BTreeMap<u32, PageId>,
    by_page: Vec<(PageId, Vec<(ObjRef, ObjRef)>)>,
}

fn widget_pages(doc: &Document) -> Result<WidgetPages> {
    let mut out = WidgetPages::default();
    for page in doc.pages()?.iter() {
        let widgets = doc.page_widgets(page.id())?;
        for w in &widgets {
            out.page_of.entry(w.widget.num).or_insert_with(|| page.id().clone());
        }
        out.by_page
            .push((page.id().clone(), widgets.iter().map(|w| (w.widget, w.field)).collect()));
    }
    Ok(out)
}

/// Where the walk starts: a node per field `/Fields` lists, and the widgets it lists in a
/// field's place, by the field's object number.
struct Roots {
    nodes: Vec<Node>,
    listed_widgets: BTreeMap<u32, Vec<ObjRef>>,
}

/// The nodes the walk starts from, one per field, in the order of `/Fields`.
///
/// ISO 32000-2 §12.7.4 has `/Fields` hold the root fields, but some writers list a field's
/// widgets there instead (pdf.js's `opt_demo.pdf`). Reading is tolerant (ADR 0003), as qpdf's
/// and pdf.js's are: an entry that is only a widget, with neither `/T` nor `/FT` of its own
/// (`listed_widget`), stands for the field its `/Parent` names, which inherits from the fields
/// above it, and the walk reaches that field once however many of its widgets are listed. An
/// entry with `/T` or `/FT` keeps being read as the field it says it is.
fn roots(doc: &Document, acro: &Dict, defaults: &Inherited) -> Result<Roots> {
    let mut nodes = Vec::new();
    let mut listed_widgets: BTreeMap<u32, Vec<ObjRef>> = BTreeMap::new();
    let mut placed = BTreeSet::new();
    for entry in refs(doc, acro.get("Fields"))? {
        let chain = match dict_at(doc, entry)? {
            Some(dict) if listed_widget(&dict) => parent_chain(doc, entry, &dict)?,
            _ => Vec::new(),
        };
        // Only a parent that has a field's own keys (Table 226: `/T`, the partial name, or
        // `/FT`, the type) is taken for the field: a `/Parent` naming the page tree or another
        // object that isn't one leaves the entry as the untyped node it is, and reported so.
        let chain = match chain.first() {
            Some((_, parent)) if parent.contains_key("T") || parent.contains_key("FT") => chain,
            _ => Vec::new(),
        };
        let Some(((field, _), above)) = chain.split_first() else {
            if placed.insert(entry.num) {
                nodes.push(Node {
                    object: entry,
                    inherited: defaults.clone(),
                    listed_by: None,
                    depth: 0,
                });
            }
            continue;
        };
        listed_widgets.entry(field.num).or_default().push(entry);
        if placed.insert(field.num) {
            let mut inherited = defaults.clone();
            for (_, dict) in above.iter().rev() {
                inherited = inherit(doc, dict, inherited)?;
            }
            nodes.push(Node {
                object: *field,
                inherited,
                listed_by: None,
                depth: above.len(),
            });
        }
    }
    Ok(Roots { nodes, listed_widgets })
}

/// Whether a `/Fields` entry is a widget and nothing of a field: no `/T` (`is_field_node`)
/// and no `/FT`.
fn listed_widget(dict: &Dict) -> bool {
    !is_field_node(dict) && !dict.contains_key("FT")
}

/// The fields up `widget`'s `/Parent` chain, its parent first, with their dictionaries: each
/// once (a loop ends the climb), as deep as `Limits::max_tree_depth`, and up to an object that
/// can't be read.
fn parent_chain(doc: &Document, widget: ObjRef, dict: &Dict) -> Result<Vec<(ObjRef, Dict)>> {
    let max_depth = doc.limits().max_tree_depth();
    let mut seen = BTreeSet::from([widget.num]);
    let mut chain = Vec::new();
    let mut next = dict.get("Parent").and_then(Object::as_ref);
    while let Some(r) = next {
        if chain.len() >= max_depth || !seen.insert(r.num) {
            break;
        }
        let Some(d) = dict_at(doc, r)? else { break };
        next = d.get("Parent").and_then(Object::as_ref);
        chain.push((r, d));
    }
    Ok(chain)
}

/// Walks `/Fields` depth first, recording mismatched parents and untyped and widgetless fields
/// in `form` and every object reached in `reached`; returns the terminal fields in order.
fn walk(
    doc: &Document,
    acro: &Dict,
    defaults: &Inherited,
    pages: &WidgetPages,
    form: &mut Form,
    reached: &mut BTreeSet<u32>,
) -> Result<Vec<Terminal>> {
    let max_depth = doc.limits().max_tree_depth();
    let Roots {
        nodes,
        mut listed_widgets,
    } = roots(doc, acro, defaults)?;
    let mut stack: Vec<Node> = nodes.into_iter().rev().collect();
    let mut out = Vec::new();
    while let Some(node) = stack.pop() {
        let Node {
            object: r,
            inherited,
            listed_by,
            depth,
        } = node;
        if depth >= max_depth || !reached.insert(r.num) {
            continue;
        }
        let Some(dict) = dict_at(doc, r)? else { continue };
        if let Some(listed_by) = listed_by
            && dict.get("Parent").and_then(Object::as_ref) != Some(listed_by)
        {
            form.parent_mismatches.push(FieldId::object(r));
        }
        let own = inherit(doc, &dict, inherited)?;
        let kids = refs(doc, dict.get("Kids"))?;
        let mut kid_dicts = Vec::with_capacity(kids.len());
        for kid in kids {
            if let Some(d) = dict_at(doc, kid)? {
                kid_dicts.push((kid, d));
            }
        }
        if kid_dicts.iter().any(|(_, d)| is_field_node(d)) {
            for (kid, _) in kid_dicts.into_iter().rev() {
                stack.push(Node {
                    object: kid,
                    inherited: own.clone(),
                    listed_by: Some(r),
                    depth: depth + 1,
                });
            }
            continue;
        }
        let mut widgets = Vec::new();
        if kid_dicts.is_empty() && is_merged_widget(&dict) {
            widgets.push(widget(doc, r, &dict, pages)?);
        }
        for (kid, d) in &kid_dicts {
            if d.get("Parent").and_then(Object::as_ref) != Some(r) {
                form.parent_mismatches.push(FieldId::object(*kid));
            }
            reached.insert(kid.num);
            widgets.push(widget(doc, *kid, d, pages)?);
        }
        // The widgets `/Fields` listed in this field's place, unless its `/Kids` held them too.
        for listed in listed_widgets.remove(&r.num).unwrap_or_default() {
            if reached.insert(listed.num)
                && let Some(d) = dict_at(doc, listed)?
            {
                widgets.push(widget(doc, listed, &d, pages)?);
            }
        }
        let terminal = terminal(doc, r, &dict, own, widgets)?;
        if terminal.field.widgets.iter().all(|w| w.page.is_none()) {
            form.widgetless.push(terminal.field.id.clone());
        }
        if !terminal.typed {
            form.untyped.push(terminal.field.id.clone());
        }
        out.push(terminal);
    }
    Ok(out)
}

/// What `dict` inherits: its own `/T`, `/FT`, `/Ff`, `/V`, `/DV`, `/DA`, `/Q` and `/MaxLen`
/// where it has them, else `parent`'s.
fn inherit(doc: &Document, dict: &Dict, parent: Inherited) -> Result<Inherited> {
    let name = match text(doc, dict.get("T"))? {
        Some(partial) => Some(
            parent
                .name
                .map_or_else(|| partial.clone(), |p| format!("{p}.{partial}")),
        ),
        None => parent.name,
    };
    Ok(Inherited {
        name,
        ft: read_name(doc, dict.get("FT"))?.or(parent.ft),
        ff: flags(doc, dict.get("Ff"))?.or(parent.ff),
        v: resolved(doc, dict.get("V"))?.or(parent.v),
        dv: resolved(doc, dict.get("DV"))?.or(parent.dv),
        da: text(doc, dict.get("DA"))?.or(parent.da),
        q: quadding(doc, dict.get("Q"))?.or(parent.q),
        max_len: count(doc, dict.get("MaxLen"))?.or(parent.max_len),
    })
}

/// The terminal field `r` (dictionary `dict`) with what it inherits and its widgets.
fn terminal(doc: &Document, r: ObjRef, dict: &Dict, own: Inherited, widgets: Vec<Widget>) -> Result<Terminal> {
    let flags = FieldFlags(own.ff.unwrap_or(0));
    let kind = own.ft.as_deref().and_then(|ft| FieldKind::of(ft, flags));
    let value_kind = kind.unwrap_or(FieldKind::Text);
    let value = own
        .v
        .as_ref()
        .map(|v| value_of(doc, v, value_kind))
        .transpose()?
        .flatten();
    let default_value = own
        .dv
        .as_ref()
        .map(|v| value_of(doc, v, value_kind))
        .transpose()?
        .flatten();
    let signed = kind == Some(FieldKind::Signature) && matches!(own.v, Some(Object::Dict(_)));
    Ok(Terminal {
        typed: kind.is_some(),
        field: TerminalField {
            id: FieldId::object(r),
            shared: Vec::new(),
            name: own.name.unwrap_or_default(),
            tooltip: text(doc, dict.get("TU"))?,
            kind: value_kind,
            flags,
            value,
            default_value,
            default_appearance: own.da,
            quadding: own.q.unwrap_or(0),
            max_len: own.max_len,
            options: options(doc, dict.get("Opt"))?,
            signed,
            widgets,
        },
    })
}

/// `v`, a `/V` or `/DV`, as the value of a field of `kind`; `None` when it is of the wrong type
/// (or a push button's or signature's).
fn value_of(doc: &Document, v: &Object, kind: FieldKind) -> Result<Option<FieldValue>> {
    Ok(match kind {
        FieldKind::Text => match v {
            // A text stream that can't be decoded counts as absent (ADR 0003); unavailable
            // input and hard limits still end the read.
            Object::Stream(s) => {
                absent_if_malformed(doc.decode_stream(s))?.map(|bytes| FieldValue::Text(decode_text_string(&bytes)))
            }
            other => text_of(other).map(FieldValue::Text),
        },
        FieldKind::CheckBox | FieldKind::RadioGroup => match v {
            Object::Name(n) if n.as_bytes() == b"Off" => Some(FieldValue::Button(None)),
            Object::Name(n) => Some(FieldValue::Button(Some(name_bytes_text(n.as_bytes())))),
            _ => None,
        },
        FieldKind::ComboBox | FieldKind::ListBox => match v {
            Object::Array(items) => {
                let mut values = Vec::new();
                for item in items {
                    if let Some(s) = doc.read(item)?.as_deref().and_then(text_of) {
                        values.push(s);
                    }
                }
                Some(FieldValue::Choice(values))
            }
            other => text_of(other).map(|s| FieldValue::Choice(vec![s])),
        },
        FieldKind::PushButton | FieldKind::Signature => None,
    })
}

/// `/Opt` (§12.7.5.4, Table 233; §12.7.5.2.4 for buttons): each entry a text string, or an
/// array of the export value and the text shown. An entry of another shape is left out.
fn options(doc: &Document, value: Option<&Object>) -> Result<Vec<ChoiceOption>> {
    let Some(opt) = resolved(doc, value)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for item in opt.as_array().unwrap_or_default() {
        let Some(item) = doc.read(item)? else { continue };
        match &*item {
            Object::Array(pair) => {
                let mut texts = Vec::with_capacity(2);
                for part in pair.iter().take(2) {
                    texts.push(doc.read(part)?.as_deref().and_then(text_of));
                }
                if let [Some(export), Some(display)] = texts.as_slice() {
                    out.push(ChoiceOption::new(export.clone(), display.clone()));
                }
            }
            other => {
                if let Some(s) = text_of(other) {
                    out.push(ChoiceOption::new(s.clone(), s));
                }
            }
        }
    }
    Ok(out)
}

/// The widget `r` (dictionary `dict`).
fn widget(doc: &Document, r: ObjRef, dict: &Dict, pages: &WidgetPages) -> Result<Widget> {
    let rect = match dict.get("Rect") {
        Some(v) => read_rect(doc, v)?,
        None => None,
    };
    let flags = flags(doc, dict.get("F"))?.unwrap_or(0);
    Ok(Widget {
        id: AnnotId::object(r),
        page: pages.page_of.get(&r.num).cloned(),
        rect,
        flags: AnnotFlags(flags),
        on_state: on_state(doc, dict)?,
        state: read_name(doc, dict.get("AS"))?.map(|n| name_bytes_text(&n)),
    })
}

/// The first name of the normal appearance dictionary (`/AP /N`) that isn't `Off`: a button
/// widget's on-state (§12.7.5.2.3).
fn on_state(doc: &Document, dict: &Dict) -> Result<Option<String>> {
    let Some(ap) = resolved(doc, dict.get("AP"))? else {
        return Ok(None);
    };
    let Some(normal) = resolved(doc, ap.as_dict().and_then(|d| d.get("N")))? else {
        return Ok(None);
    };
    Ok(normal
        .as_dict()
        .and_then(|n| n.iter().map(|(k, _)| k).find(|k| k.as_bytes() != b"Off"))
        .map(|k| name_bytes_text(k.as_bytes())))
}

/// Merges terminal fields that share a fully qualified name into the first met (§12.7.4.2),
/// and puts the typed ones in `form`.
fn merge_shared(terminals: Vec<Terminal>, form: &mut Form) {
    let mut by_name: BTreeMap<String, usize> = BTreeMap::new();
    for Terminal { field, typed } in terminals {
        if !typed {
            continue;
        }
        if let Some(first) = by_name.get(&field.name).and_then(|&i| form.fields.get_mut(i)) {
            first.shared.push(field.id);
            first.widgets.extend(field.widgets);
            continue;
        }
        by_name.insert(field.name.clone(), form.fields.len());
        form.fields.push(field);
    }
}

/// Whether a kid is a field rather than its parent's widget: it has a partial name, or isn't a
/// widget annotation (§12.7.4: a field's kids are all fields or all widgets).
fn is_field_node(dict: &Dict) -> bool {
    dict.contains_key("T") || !is_merged_widget(dict)
}

/// Whether `dict` is (also) a widget annotation: `/Subtype /Widget`, or, as some writers leave
/// the subtype out, no subtype and a `/Rect`.
fn is_merged_widget(dict: &Dict) -> bool {
    dict.get_name("Subtype")
        .map_or_else(|| dict.contains_key("Rect"), |s| s == b"Widget")
}

/// The object `value` resolves to, if any.
fn resolved(doc: &Document, value: Option<&Object>) -> Result<Option<Object>> {
    let Some(value) = value else { return Ok(None) };
    Ok(doc.read(value)?.map(|o| (*o).clone()))
}

/// The text string `value` resolves to.
fn text(doc: &Document, value: Option<&Object>) -> Result<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    Ok(doc.read(value)?.as_deref().and_then(text_of))
}

/// A flags integer (`/Ff`, `/F`), its low 32 bits: some writers store bit 32 as a negative
/// number.
fn flags(doc: &Document, value: Option<&Object>) -> Result<Option<u32>> {
    let Some(value) = value else { return Ok(None) };
    Ok(doc
        .read(value)?
        .and_then(|o| o.as_int())
        .and_then(|n| u32::try_from(n.rem_euclid(1 << 32)).ok()))
}

/// A non-negative integer (`/MaxLen`).
fn count(doc: &Document, value: Option<&Object>) -> Result<Option<u32>> {
    let Some(value) = value else { return Ok(None) };
    Ok(doc
        .read(value)?
        .and_then(|o| o.as_int())
        .and_then(|n| u32::try_from(n).ok()))
}

/// `/Q`, the quadding: 0, 1 or 2; any other value counts as absent.
fn quadding(doc: &Document, value: Option<&Object>) -> Result<Option<u8>> {
    let Some(value) = value else { return Ok(None) };
    Ok(doc
        .read(value)?
        .and_then(|o| o.as_int())
        .and_then(|q| u8::try_from(q).ok())
        .filter(|q| *q <= 2))
}

/// The references among the items of the array `value`.
fn refs(doc: &Document, value: Option<&Object>) -> Result<Vec<ObjRef>> {
    Ok(array_items(doc, value)?.iter().filter_map(Object::as_ref).collect())
}
