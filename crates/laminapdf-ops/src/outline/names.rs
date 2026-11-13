//! The named-destination ops and the two places a name lives (§12.3.2.4): a string key of the
//! `/Names /Dests` tree, or a name-object key of the catalog's `/Dests` dictionary.

use std::collections::BTreeMap;

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{AnnotId, OutlineItemId, PageId, TreeWriter, set_catalog_entry};
use serde::{Deserialize, Serialize};

use super::target::explicit;
use super::types::{ExplicitDestination, RenameLinks};
use crate::error::{Error, Result};
use crate::op::Effect;

/// Adds a named destination, or points an existing one at `destination`. A name the catalog's
/// `/Dests` dictionary holds stays there; any other goes into the `/Names /Dests` tree.
///
/// On the wire: `{ "op": "setNamedDestination", "args": { "name": "results", "destination": {
/// "page": "p7.0", "view": { "kind": "fitH", "top": 700 } } } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetNamedDestination {
    /// The name.
    pub name: String,
    /// Where it points.
    pub destination: ExplicitDestination,
}

impl SetNamedDestination {
    /// The op that points `name` at `destination`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(name: impl Into<String>, destination: ExplicitDestination) -> Self {
        Self {
            name: name.into(),
            destination,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let key = non_empty(&self.name)?;
        let value = explicit(doc, &self.destination)?;
        let mut stores = Stores::read(doc)?;
        // Where readers find the name now (the tree first), else the tree.
        let store = stores.find(key).unwrap_or(Store::Tree);
        stores.put(store, key.to_vec(), value);
        stores.write(doc, store)?;
        Ok(Effect::default())
    }
}

/// Renames a named destination, keeping it where it is. With `links: follow` (the default) the
/// bookmarks, links and `GoTo` actions of this file that name it are renamed too, in the form
/// they named it in.
///
/// On the wire: `{ "op": "renameNamedDestination", "args": { "from": "sec2", "to": "methods" } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct RenameNamedDestination {
    /// The current name.
    pub from: String,
    /// The new name; no destination may have it yet.
    pub to: String,
    /// Whether the uses follow.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub links: RenameLinks,
}

impl RenameNamedDestination {
    /// The op that renames `from` to `to`, and its uses with it.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            links: RenameLinks::Follow,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let (from, to) = (non_empty(&self.from)?, non_empty(&self.to)?);
        let mut stores = Stores::read(doc)?;
        let store = stores
            .find(from)
            .ok_or_else(|| Error::UnknownNamedDestination(self.from.clone()))?;
        if from == to {
            return Ok(Effect::default());
        }
        if stores.find(to).is_some() {
            return Err(Error::NamedDestinationExists(self.to.clone()));
        }
        let value = stores.take(store, from).unwrap_or(Object::Null);
        stores.put(store, to.to_vec(), value);
        stores.write(doc, store)?;
        if self.links == RenameLinks::Follow {
            for found in uses(doc, from, store)? {
                found.rename(doc, to, store)?;
            }
        }
        Ok(Effect::default())
    }
}

/// Deletes a named destination. Its uses in this file are left as they are and reported
/// ([`Applied::destination_uses`](crate::Applied::destination_uses)): a bookmark or link that
/// names it no longer goes anywhere.
///
/// On the wire: `{ "op": "deleteNamedDestination", "args": { "name": "draft-notes" } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DeleteNamedDestination {
    /// The name.
    pub name: String,
}

impl DeleteNamedDestination {
    /// The op that deletes `name`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let key = non_empty(&self.name)?;
        let mut stores = Stores::read(doc)?;
        let store = stores
            .find(key)
            .ok_or_else(|| Error::UnknownNamedDestination(self.name.clone()))?;
        stores.take(store, key);
        stores.write(doc, store)?;
        let found = uses(doc, key, store)?;
        Ok(Effect {
            destination_uses: found.into_iter().map(|u| u.holder).collect(),
            ..Effect::default()
        })
    }
}

/// Something in the file that names a destination.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum DestinationUse {
    /// A bookmark.
    OutlineItem {
        /// The bookmark.
        #[serde(with = "crate::wire::outline_item_id")]
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        item: OutlineItemId,
    },
    /// An annotation on a page: a link, or another whose actions go to the name.
    Annotation {
        /// The page.
        #[serde(with = "crate::wire::page_id")]
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        page: PageId,
        /// The annotation (`a12.0`).
        #[serde(with = "crate::wire::annot_id")]
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        annot: AnnotId,
    },
    /// The catalog: what the file shows when it opens (`/OpenAction`), or a document action
    /// (`/AA`).
    Catalog,
}

pub(super) fn non_empty(name: &str) -> Result<&[u8]> {
    if name.is_empty() {
        Err(Error::EmptyDestinationName)
    } else {
        Ok(name.as_bytes())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Store {
    Tree,
    DestsDict,
}

/// Where the catalog's `/Dests` dictionary is: its own object, or inside the catalog.
#[derive(Debug, Clone, Copy)]
enum Holder {
    Object(ObjRef),
    Catalog,
}

/// Both stores' entries, as read.
pub(super) struct Stores {
    tree: BTreeMap<Vec<u8>, Object>,
    dests: Option<(Holder, Dict)>,
}

impl Stores {
    pub(super) fn read(doc: &laminapdf_model::Document) -> Result<Self> {
        let tree = match doc.name_tree("Dests")? {
            Some(tree) => tree.entries()?.into_iter().collect(),
            None => BTreeMap::new(),
        };
        let dests = match doc.catalog()?.get("Dests") {
            Some(Object::Ref(r)) => doc.resolve(*r)?.as_dict().map(|d| (Holder::Object(*r), d.clone())),
            Some(Object::Dict(d)) => Some((Holder::Catalog, d.clone())),
            _ => None,
        };
        Ok(Self { tree, dests })
    }

    pub(super) fn in_dests_dict(&self, name: &[u8]) -> bool {
        self.dests
            .as_ref()
            .is_some_and(|(_, d)| d.iter().any(|(k, _)| k.as_bytes() == name))
    }

    /// The store that holds `name`: the tree first, as readers look there first.
    pub(super) fn find(&self, name: &[u8]) -> Option<Store> {
        if self.tree.contains_key(name) {
            Some(Store::Tree)
        } else if self.in_dests_dict(name) {
            Some(Store::DestsDict)
        } else {
            None
        }
    }

    fn put(&mut self, store: Store, name: Vec<u8>, value: Object) {
        match store {
            Store::Tree => {
                self.tree.insert(name, value);
            }
            Store::DestsDict => {
                let (_, dict) = self.dests.get_or_insert_with(|| (Holder::Catalog, Dict::new()));
                dict.insert(laminapdf_cos::Name(name), value);
            }
        }
    }

    fn take(&mut self, store: Store, name: &[u8]) -> Option<Object> {
        match store {
            Store::Tree => self.tree.remove(name),
            Store::DestsDict => {
                let (_, dict) = self.dests.as_mut()?;
                let key = std::str::from_utf8(name).ok()?;
                dict.remove(key)
            }
        }
    }

    /// Writes `store` back: the tree whole through [`TreeWriter`] (or `/Dests` taken out of
    /// `/Names` when it is empty), the dictionary in its place.
    fn write(&self, doc: &mut laminapdf_model::Document, store: Store) -> Result<()> {
        let names = doc.catalog()?.get("Names").cloned();
        let mut cos = doc.cos_mut().edit();
        match store {
            Store::Tree => {
                let root = if self.tree.is_empty() {
                    None
                } else {
                    Some(Object::Ref(TreeWriter::new().write_names(&mut cos, &self.tree)?))
                };
                match names {
                    Some(Object::Ref(r)) => {
                        let mut dict = cos.get(r)?.and_then(|o| o.as_dict().cloned()).unwrap_or_default();
                        put(&mut dict, "Dests", root);
                        cos.set(r, Object::Dict(dict))?;
                    }
                    Some(Object::Dict(mut dict)) => {
                        put(&mut dict, "Dests", root);
                        set_catalog_entry(&mut cos, "Names", Some(Object::Dict(dict)))?;
                    }
                    _ => {
                        if let Some(root) = root {
                            let dict: Dict = [("Dests", root)].into_iter().collect();
                            set_catalog_entry(&mut cos, "Names", Some(Object::Dict(dict)))?;
                        }
                    }
                }
            }
            Store::DestsDict => match &self.dests {
                Some((Holder::Object(r), dict)) => cos.set(*r, Object::Dict(dict.clone()))?,
                Some((Holder::Catalog, dict)) => {
                    set_catalog_entry(&mut cos, "Dests", Some(Object::Dict(dict.clone())))?;
                }
                None => {}
            },
        }
        Ok(())
    }
}

fn put(dict: &mut Dict, key: &str, value: Option<Object>) {
    match value {
        Some(v) => dict.insert(key, v),
        None => {
            dict.remove(key);
        }
    }
}

/// One step from an object to the value a use names the destination in.
#[derive(Debug, Clone)]
enum Step {
    Key(Vec<u8>),
    Index(usize),
}

/// A place that names the destination: an indirect object and the path inside it.
struct Use {
    holder: DestinationUse,
    object: ObjRef,
    path: Vec<Step>,
}

impl Use {
    fn rename(&self, doc: &mut laminapdf_model::Document, to: &[u8], store: Store) -> Result<()> {
        let mut root = (*doc.resolve(self.object)?).clone();
        let mut at = &mut root;
        for step in &self.path {
            let next = match (step, at) {
                (Step::Key(k), Object::Dict(d)) => std::str::from_utf8(k).ok().and_then(|k| d.get_mut(k)),
                (Step::Index(i), Object::Array(a)) => a.get_mut(*i),
                _ => None,
            };
            let Some(next) = next else { return Ok(()) };
            at = next;
        }
        *at = name_value(to, store);
        doc.cos_mut().edit().set(self.object, root)?;
        Ok(())
    }
}

/// The name as a use of `store` writes it: a string for the tree, a name object for `/Dests`.
fn name_value(name: &[u8], store: Store) -> Object {
    match store {
        Store::Tree => Object::String(PdfString::literal(name.to_vec())),
        Store::DestsDict => Object::Name(laminapdf_cos::Name(name.to_vec())),
    }
}

fn names(value: Option<&Object>, name: &[u8], store: Store) -> bool {
    match (value, store) {
        (Some(Object::String(s)), Store::Tree) => s.bytes == name,
        (Some(Object::Name(n)), Store::DestsDict) => n.as_bytes() == name,
        _ => false,
    }
}

/// How deep `/Next` chains are followed, against a chain built to go on for ever.
const MAX_ACTIONS: usize = 64;

/// Finds every use of one name in one form.
struct Finder<'a> {
    doc: &'a laminapdf_model::Document,
    name: &'a [u8],
    store: Store,
    /// The indirect actions visited, so a `/Next` cycle ends.
    seen: std::collections::BTreeSet<ObjRef>,
    out: Vec<Use>,
}

impl Finder<'_> {
    /// The uses inside `dict` (the object `r`, or a dictionary at `path` in it) through `/Dest`,
    /// `/A` and each `/AA` trigger, for `holder`.
    fn holder(&mut self, holder: &DestinationUse, r: ObjRef, dict: &Dict, path: &[Step], keys: &[&str]) -> Result<()> {
        let at = |key: &str| [path, &[Step::Key(key.as_bytes().to_vec())]].concat();
        if names(dict.get("Dest"), self.name, self.store) {
            self.found(holder, r, at("Dest"));
        }
        for key in keys {
            if let Some(action) = dict.get(key) {
                self.action(holder, r, action, &at(key), 0)?;
            }
        }
        // `/AA` holds one action per trigger, inside this object or in its own.
        let triggers = match dict.get("AA") {
            Some(Object::Dict(aa)) => Some((r, at("AA"), aa.clone())),
            Some(Object::Ref(aa)) => self.doc.resolve(*aa)?.as_dict().map(|d| (*aa, Vec::new(), d.clone())),
            _ => None,
        };
        if let Some((object, base, aa)) = triggers {
            for (trigger, action) in aa.iter() {
                let mut p = base.clone();
                p.push(Step::Key(trigger.as_bytes().to_vec()));
                self.action(holder, object, action, &p, 0)?;
            }
        }
        Ok(())
    }

    /// The uses in `value`, an action (or an array of them) at `path` in object `r`, and in the
    /// actions its `/Next` runs after it.
    fn action(
        &mut self,
        holder: &DestinationUse,
        r: ObjRef,
        value: &Object,
        path: &[Step],
        depth: usize,
    ) -> Result<()> {
        if depth > MAX_ACTIONS {
            return Ok(());
        }
        match value {
            Object::Ref(a) => {
                if !self.seen.insert(*a) {
                    return Ok(());
                }
                let resolved = self.doc.resolve(*a)?;
                self.action(holder, *a, &resolved, &[], depth + 1)
            }
            Object::Array(list) => {
                for (i, v) in list.iter().enumerate() {
                    let mut p = path.to_vec();
                    p.push(Step::Index(i));
                    self.action(holder, r, v, &p, depth + 1)?;
                }
                Ok(())
            }
            Object::Dict(action) => {
                if action.get_name("S") == Some(b"GoTo") && names(action.get("D"), self.name, self.store) {
                    let mut p = path.to_vec();
                    p.push(Step::Key(b"D".to_vec()));
                    self.found(holder, r, p);
                }
                if let Some(next) = action.get("Next") {
                    let mut p = path.to_vec();
                    p.push(Step::Key(b"Next".to_vec()));
                    self.action(holder, r, next, &p, depth + 1)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn found(&mut self, holder: &DestinationUse, object: ObjRef, path: Vec<Step>) {
        self.out.push(Use {
            holder: holder.clone(),
            object,
            path,
        });
    }
}

/// Every place that names `name` in `store`'s form: bookmarks (`/Dest`, `/A`), annotations
/// (`/Dest`, `/A`, `/AA`), the catalog's `/OpenAction` and `/AA`, and the actions each runs after
/// through `/Next`. Bookmarks and annotations stored as direct dictionaries are left out.
fn uses(doc: &laminapdf_model::Document, name: &[u8], store: Store) -> Result<Vec<Use>> {
    let mut finder = Finder {
        doc,
        name,
        store,
        seen: std::collections::BTreeSet::new(),
        out: Vec::new(),
    };
    for item in doc.outline()?.iter() {
        let Some(id) = item.id() else { continue };
        let Some(r) = id.as_lamina_id().obj_ref() else { continue };
        let Some(dict) = doc.resolve(r)?.as_dict().cloned() else {
            continue;
        };
        finder.holder(&DestinationUse::OutlineItem { item: id.clone() }, r, &dict, &[], &["A"])?;
    }
    let pages = doc.pages()?;
    for page in pages.iter() {
        for annot in page.annots()? {
            let Some(dict) = doc.resolve(annot)?.as_dict().cloned() else {
                continue;
            };
            let holder = DestinationUse::Annotation {
                page: page.id().clone(),
                annot: AnnotId::object(annot),
            };
            finder.holder(&holder, annot, &dict, &[], &["A"])?;
        }
    }
    if let Some(Object::Ref(root)) = doc.trailer().get("Root").cloned() {
        let catalog = doc.catalog()?;
        finder.holder(&DestinationUse::Catalog, root, &catalog, &[], &["OpenAction"])?;
    }
    Ok(finder.out)
}
