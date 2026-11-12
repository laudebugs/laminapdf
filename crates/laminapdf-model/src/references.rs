//! What points at a page ([`PageReferences`]): outline items (ISO 32000-2 §12.3.3), link and
//! other annotations with a destination (§12.5.6.5), named destinations (§12.3.2.4),
//! `/OpenAction` (§7.7.2), structure elements and their `MCR`/`OBJR` kids by `/Pg` (§14.7.2,
//! §14.7.5) and form-field widgets (§12.7.4), as typed entries an edit can retarget or remove.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object, Result};

use crate::document::Document;
use crate::id::{OutlineItemId, PageId};
use crate::outline::{ActionKind, Destination, Outline};
use crate::page::Pages;
use crate::structure::kids_of;
use crate::values::read_name;

/// One thing that points at a page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageReference {
    /// The page it points at.
    pub page: PageId,
    /// What points at it, and where the pointer is stored.
    pub kind: ReferenceKind,
}

/// What points at a page.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReferenceKind {
    /// An outline item, through its `/Dest` or a `GoTo` action.
    OutlineItem {
        /// The item.
        item: OutlineItemId,
        /// Where its destination is stored.
        via: Via,
    },
    /// An annotation (a link, or any other with a `/Dest` or a `GoTo` action), through its
    /// destination. Annotations stored as direct dictionaries in `/Annots` are left out.
    Link {
        /// The annotation object.
        annot: ObjRef,
        /// The page whose `/Annots` lists it.
        on: PageId,
        /// Where its destination is stored.
        via: Via,
    },
    /// A named destination whose explicit destination names the page.
    NamedDestination {
        /// The name.
        name: Vec<u8>,
        /// Which store holds it.
        store: NameStore,
    },
    /// The catalog's `/OpenAction`: an explicit destination or a `GoTo` action.
    OpenAction {
        /// Where its destination is stored.
        via: Via,
    },
    /// A structure element whose `/Pg` is the page.
    StructElem {
        /// The element (`None` for a direct one).
        elem: Option<ObjRef>,
    },
    /// A marked-content reference (`MCR`) or object reference (`OBJR`) kid of a structure
    /// element whose own `/Pg` is the page. It is named by its element and its index in the
    /// element's `/K` (counting every item, MCIDs included), and by its own object when it is
    /// indirect.
    StructContentRef {
        /// The element whose `/K` holds it (`None` for a direct element).
        elem: Option<ObjRef>,
        /// Its index in the element's `/K`.
        index: usize,
        /// The kid's dictionary, when it is an indirect object.
        kid: Option<ObjRef>,
    },
    /// A widget annotation of a form field, on the page.
    Widget {
        /// The widget annotation.
        widget: ObjRef,
        /// The terminal field it belongs to (the widget itself for a merged field and widget).
        field: ObjRef,
        /// How the widget names its page.
        by: WidgetLink,
    },
}

/// Where a destination is stored on the object that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Via {
    /// The holder's `/Dest` (for `/OpenAction`, the value itself).
    Dest {
        /// The named destination it goes through, when it holds a name.
        name: Option<Vec<u8>>,
    },
    /// The `/D` of the holder's `GoTo` action (`/A`, or `/OpenAction`).
    GoTo {
        /// The action object, when the action is indirect.
        action: Option<ObjRef>,
        /// The named destination it goes through, when `/D` holds a name.
        name: Option<Vec<u8>>,
    },
}

/// Where a named destination is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NameStore {
    /// The `/Names /Dests` name tree (PDF 1.2).
    NameTree,
    /// The catalog's `/Dests` dictionary (PDF 1.1).
    DestsDict,
}

/// How a widget names its page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WidgetLink {
    /// Its `/P`.
    P,
    /// It has no usable `/P`; the page's `/Annots` lists it.
    Annots,
}

/// Everything that points at a set of pages, found by [`Document::page_references`].
///
/// `GoTo` actions are those of outline items, annotations and `/OpenAction`; actions chained
/// by `/Next` and those in additional-actions (`/AA`) dictionaries are not followed. A
/// destination that names a page through a named destination is reported twice: once on its
/// holder (with the name in [`Via`]) and once as the [`ReferenceKind::NamedDestination`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageReferences {
    entries: Vec<PageReference>,
}

impl PageReferences {
    /// Every reference, grouped by kind in the order of [`ReferenceKind`]'s variants, each
    /// group in document order.
    #[must_use]
    pub fn entries(&self) -> &[PageReference] {
        &self.entries
    }

    /// The references to `page`.
    pub fn to_page<'a>(&'a self, page: &'a PageId) -> impl Iterator<Item = &'a PageReference> + 'a {
        self.entries.iter().filter(move |e| &e.page == page)
    }

    /// The number of references.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing points at the pages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Document {
    /// Everything that points at `pages` ([`PageReferences`]).
    ///
    /// # Errors
    /// Reading errors (the page tree, outline, annotations, names, structure tree and form);
    /// in strict mode a warning they raise.
    pub fn page_references(&self, pages: &[PageId]) -> Result<PageReferences> {
        let all = self.pages()?;
        let outline = all.outline()?;
        let mut finder = Finder {
            doc: self,
            outline: &outline,
            targets: pages.iter().cloned().collect(),
            out: Vec::new(),
        };
        finder.outline_items()?;
        let annots = finder.annotations(&all)?;
        finder.named()?;
        finder.open_action()?;
        finder.structure()?;
        finder.widgets(&all, &annots)?;
        Ok(PageReferences { entries: finder.out })
    }
}

struct Finder<'a, 'd> {
    doc: &'d Document,
    outline: &'a Outline<'d>,
    targets: BTreeSet<PageId>,
    out: Vec<PageReference>,
}

impl Finder<'_, '_> {
    fn push(&mut self, dest: Option<&Destination>, kind: ReferenceKind) {
        if let Some(page) = dest.and_then(|d| d.page.clone()).filter(|p| self.targets.contains(p)) {
            self.out.push(PageReference { page, kind });
        }
    }

    fn outline_items(&mut self) -> Result<()> {
        let outline = self.outline;
        for item in outline.iter() {
            let (Some(id), Some(dict)) = (item.id(), item.dict()) else {
                continue;
            };
            if let Some((dest, via)) = self.target(dict)? {
                self.push(Some(&dest), ReferenceKind::OutlineItem { item: id.clone(), via });
            }
        }
        Ok(())
    }

    /// The annotations with a destination; returns which page lists each annotation.
    fn annotations(&mut self, pages: &Pages<'_>) -> Result<BTreeMap<ObjRef, PageId>> {
        let mut listed = BTreeMap::new();
        for page in pages.iter() {
            for annot in page.annots()? {
                listed.entry(annot).or_insert_with(|| page.id().clone());
                let Some(dict) = self.dict(&Object::Ref(annot))? else {
                    continue;
                };
                if let Some((dest, via)) = self.target(&dict)? {
                    let kind = ReferenceKind::Link {
                        annot,
                        on: page.id().clone(),
                        via,
                    };
                    self.push(Some(&dest), kind);
                }
            }
        }
        Ok(listed)
    }

    fn named(&mut self) -> Result<()> {
        let doc = self.doc;
        if let Some(tree) = doc.name_tree("Dests")? {
            for (name, value) in tree.entries()? {
                let dest = self.outline.destination(&value)?;
                self.push(
                    dest.as_ref(),
                    ReferenceKind::NamedDestination {
                        name,
                        store: NameStore::NameTree,
                    },
                );
            }
        }
        let catalog = doc.catalog()?;
        if let Some(dests) = catalog.get("Dests")
            && let Some(dests) = self.dict(dests)?
        {
            for (name, value) in dests.iter() {
                let dest = self.outline.destination(value)?;
                self.push(
                    dest.as_ref(),
                    ReferenceKind::NamedDestination {
                        name: name.as_bytes().to_vec(),
                        store: NameStore::DestsDict,
                    },
                );
            }
        }
        Ok(())
    }

    fn open_action(&mut self) -> Result<()> {
        let catalog = self.doc.catalog()?;
        let Some(value) = catalog.get("OpenAction") else {
            return Ok(());
        };
        let Some(resolved) = self.doc.read(value)? else {
            return Ok(());
        };
        let found = match &*resolved {
            Object::Array(_) => self
                .outline
                .destination(value)?
                .map(|dest| (Via::Dest { name: None }, dest)),
            Object::Dict(action) => self.go_to(action, value.as_ref())?.map(|(dest, via)| (via, dest)),
            _ => None,
        };
        if let Some((via, dest)) = found {
            self.push(Some(&dest), ReferenceKind::OpenAction { via });
        }
        Ok(())
    }

    /// Structure elements by their own `/Pg`, then the `MCR` and `OBJR` kids that name the
    /// page with a `/Pg` of their own (§14.7.5.2, §14.7.5.3), in document order. Elements
    /// reached twice or deeper than `max_tree_depth` are skipped, as the structure view skips
    /// them.
    fn structure(&mut self) -> Result<()> {
        let doc = self.doc;
        let catalog = doc.catalog()?;
        let Some(root) = catalog.get("StructTreeRoot") else {
            return Ok(());
        };
        let Some(root) = self.dict(root)? else {
            return Ok(());
        };
        let Some(k) = root.get("K") else {
            return Ok(());
        };
        let max_depth = doc.limits().max_tree_depth();
        let mut visited = BTreeSet::new();
        let mut kids_found = Vec::new();
        let mut stack: Vec<(Object, usize)> = kids_of(doc, k)?.into_iter().rev().map(|o| (o, 1)).collect();
        while let Some((obj, depth)) = stack.pop() {
            if depth > max_depth || obj.as_ref().is_some_and(|r| !visited.insert(r.num)) {
                continue;
            }
            let Some(dict) = self.dict(&obj)? else { continue };
            if is_content_ref(&dict) {
                continue;
            }
            let elem = obj.as_ref();
            if let Some(page) = self.own_page(&dict) {
                self.out.push(PageReference {
                    page,
                    kind: ReferenceKind::StructElem { elem },
                });
            }
            let items = match dict.get("K") {
                Some(k) => kids_of(doc, k)?,
                None => Vec::new(),
            };
            let mut children = Vec::new();
            for (index, item) in items.into_iter().enumerate() {
                if matches!(item, Object::Integer(_) | Object::Null) {
                    continue;
                }
                let Some(kid_dict) = self.dict(&item)? else { continue };
                if !is_content_ref(&kid_dict) {
                    children.push((item, depth + 1));
                } else if let Some(page) = self.own_page(&kid_dict) {
                    let kind = ReferenceKind::StructContentRef {
                        elem,
                        index,
                        kid: item.as_ref(),
                    };
                    kids_found.push(PageReference { page, kind });
                }
            }
            stack.extend(children.into_iter().rev());
        }
        self.out.extend(kids_found);
        Ok(())
    }

    /// A dictionary's own `/Pg`, when it names one of the target pages.
    fn own_page(&self, dict: &Dict) -> Option<PageId> {
        dict.get("Pg")
            .and_then(Object::as_ref)
            .map(PageId::object)
            .filter(|p| self.targets.contains(p))
    }

    /// The widgets of `/AcroForm /Fields`' fields, by `/P`, else by the `/Annots` that lists
    /// them (`listed`). Each kid of a field is classified on its own (§12.7.4.2: a kid without
    /// `/T` may still be a field): a widget annotation without `/T` belongs to the field;
    /// anything else is a field and is walked in turn.
    fn widgets(&mut self, pages: &Pages<'_>, listed: &BTreeMap<ObjRef, PageId>) -> Result<()> {
        let doc = self.doc;
        let catalog = doc.catalog()?;
        let Some(form) = catalog.get("AcroForm") else {
            return Ok(());
        };
        let Some(form) = self.dict(form)? else {
            return Ok(());
        };
        let fields = match form.get("Fields") {
            Some(f) => doc.read(f)?.and_then(|a| a.as_array().map(<[Object]>::to_vec)),
            None => None,
        }
        .unwrap_or_default();
        let max_depth = doc.limits().max_tree_depth();
        let mut seen = BTreeSet::new();
        let mut stack: Vec<(Object, usize)> = fields.into_iter().rev().map(|f| (f, 0)).collect();
        while let Some((obj, depth)) = stack.pop() {
            let Some(r) = obj.as_ref() else { continue };
            if depth > max_depth || !seen.insert(r.num) {
                continue;
            }
            let Some(dict) = self.dict(&obj)? else { continue };
            let kids = self.kids(&dict)?;
            if kids.is_empty() {
                // A merged field and widget.
                if is_widget(&dict) {
                    self.widget(pages, listed, r, &dict, r);
                }
                continue;
            }
            let mut fields = Vec::new();
            for (kid, kid_dict) in kids {
                // A kid with `/T` is a field (merged with its widget when it has no kids).
                if !kid_dict.contains_key("T") && is_widget(&kid_dict) {
                    self.widget(pages, listed, kid, &kid_dict, r);
                } else {
                    fields.push((Object::Ref(kid), depth + 1));
                }
            }
            stack.extend(fields.into_iter().rev());
        }
        Ok(())
    }

    /// Reports `widget` (dictionary `dict`, of `field`) when it is on a target page.
    fn widget(
        &mut self,
        pages: &Pages<'_>,
        listed: &BTreeMap<ObjRef, PageId>,
        widget: ObjRef,
        dict: &Dict,
        field: ObjRef,
    ) {
        let by_p = dict
            .get("P")
            .and_then(Object::as_ref)
            .map(PageId::object)
            .filter(|p| pages.index_of(p).is_some());
        let (page, by) = match by_p {
            Some(page) => (Some(page), WidgetLink::P),
            None => (listed.get(&widget).cloned(), WidgetLink::Annots),
        };
        if let Some(page) = page.filter(|p| self.targets.contains(p)) {
            self.out.push(PageReference {
                page,
                kind: ReferenceKind::Widget { widget, field, by },
            });
        }
    }

    /// `/Kids` that are indirect dictionaries, with their dictionaries.
    fn kids(&self, dict: &Dict) -> Result<Vec<(ObjRef, Dict)>> {
        let Some(kids) = dict.get("Kids") else {
            return Ok(Vec::new());
        };
        let Some(kids) = self.doc.read(kids)? else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for kid in kids.as_array().unwrap_or_default() {
            if let (Some(r), Some(d)) = (kid.as_ref(), self.dict(kid)?) {
                out.push((r, d));
            }
        }
        Ok(out)
    }

    /// Where a holder (outline item, annotation) points: its `/Dest` first (§12.3.3 and
    /// §12.5.6.5: `/Dest` and `/A` are exclusive, and one that can't be read counts as
    /// absent), then a `GoTo` action.
    fn target(&self, dict: &Dict) -> Result<Option<(Destination, Via)>> {
        if let Some(dest) = dict.get("Dest")
            && let Some(dest) = self.outline.destination(dest)?
        {
            let name = dest.name.clone();
            return Ok(Some((dest, Via::Dest { name })));
        }
        let Some(action) = dict.get("A") else {
            return Ok(None);
        };
        let Some(resolved) = self.dict(action)? else {
            return Ok(None);
        };
        self.go_to(&resolved, action.as_ref())
    }

    /// A `GoTo` action's destination.
    fn go_to(&self, action: &Dict, object: Option<ObjRef>) -> Result<Option<(Destination, Via)>> {
        let kind = read_name(self.doc, action.get("S"))?;
        if kind.map(|k| ActionKind::from_name(&k)) != Some(ActionKind::GoTo) {
            return Ok(None);
        }
        let Some(d) = action.get("D") else {
            return Ok(None);
        };
        Ok(self.outline.destination(d)?.map(|dest| {
            let name = dest.name.clone();
            (dest, Via::GoTo { action: object, name })
        }))
    }

    /// `value` resolved, when it is a dictionary.
    fn dict(&self, value: &Object) -> Result<Option<Dict>> {
        Ok(self.doc.read(value)?.and_then(|o| o.as_dict().cloned()))
    }
}

/// A marked-content reference or object reference (`/Type /MCR` or `/OBJR`), not an element.
fn is_content_ref(dict: &Dict) -> bool {
    matches!(dict.get_name("Type"), Some(b"MCR" | b"OBJR"))
}

/// A widget annotation: `/Subtype /Widget`, or (as some writers leave the subtype out) a
/// `/Rect` without `/Kids`.
fn is_widget(dict: &Dict) -> bool {
    dict.get_name("Subtype") == Some(b"Widget") || (dict.contains_key("Rect") && !dict.contains_key("Kids"))
}
