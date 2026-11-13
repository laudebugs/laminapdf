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
        let store = if stores.in_dests_dict(key) {
            Store::DestsDict
        } else {
            Store::Tree
        };
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
    /// A link annotation on a page.
    Link {
        /// The page.
        #[serde(with = "crate::wire::page_id")]
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        page: PageId,
        /// The link's ID (`a12.0`).
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        annot: String,
    },
    /// The catalog's `/OpenAction`: what the file shows when it opens.
    OpenAction,
}

fn non_empty(name: &str) -> Result<&[u8]> {
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
    fn find(&self, name: &[u8]) -> Option<Store> {
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

/// Where in its object a use names the destination.
#[derive(Debug, Clone, Copy)]
enum Place {
    /// The object's `/Dest`.
    Dest,
    /// The object is a `GoTo` action; its `/D`.
    ActionD,
    /// The object's `key` holds a `GoTo` action dictionary; its `/D`.
    InlineAction(&'static str),
}

struct Use {
    holder: DestinationUse,
    object: ObjRef,
    place: Place,
}

impl Use {
    fn rename(&self, doc: &mut laminapdf_model::Document, to: &[u8], store: Store) -> Result<()> {
        let Some(mut dict) = doc.resolve(self.object)?.as_dict().cloned() else {
            return Ok(());
        };
        let value = name_value(to, store);
        match self.place {
            Place::Dest => dict.insert("Dest", value),
            Place::ActionD => dict.insert("D", value),
            Place::InlineAction(key) => {
                let Some(Object::Dict(mut action)) = dict.get(key).cloned() else {
                    return Ok(());
                };
                action.insert("D", value);
                dict.insert(key, Object::Dict(action));
            }
        }
        doc.cos_mut().edit().set(self.object, Object::Dict(dict))?;
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

/// Where `dict` (an indirect object `r`) names the destination, through `/Dest` or a `GoTo`
/// action under `key`.
fn place_in(
    doc: &laminapdf_model::Document,
    r: ObjRef,
    dict: &Dict,
    key: &'static str,
    name: &[u8],
    store: Store,
) -> Result<Option<(ObjRef, Place)>> {
    if key == "A" && names(dict.get("Dest"), name, store) {
        return Ok(Some((r, Place::Dest)));
    }
    let is_go_to = |a: &Dict| a.get_name("S") == Some(b"GoTo") && names(a.get("D"), name, store);
    Ok(match dict.get(key) {
        Some(Object::Ref(a)) => doc
            .resolve(*a)?
            .as_dict()
            .filter(|a| is_go_to(a))
            .map(|_| (*a, Place::ActionD)),
        Some(Object::Dict(a)) if is_go_to(a) => Some((r, Place::InlineAction(key))),
        _ => None,
    })
}

/// The bookmarks, links and open action that name `name` in `store`'s form. Bookmarks and
/// links stored as direct dictionaries are left out.
fn uses(doc: &laminapdf_model::Document, name: &[u8], store: Store) -> Result<Vec<Use>> {
    let mut out = Vec::new();
    for item in doc.outline()?.iter() {
        let Some(id) = item.id() else { continue };
        let Some(r) = id.as_lamina_id().obj_ref() else { continue };
        let Some(dict) = doc.resolve(r)?.as_dict().cloned() else {
            continue;
        };
        if let Some((object, place)) = place_in(doc, r, &dict, "A", name, store)? {
            out.push(Use {
                holder: DestinationUse::OutlineItem { item: id.clone() },
                object,
                place,
            });
        }
    }
    let pages = doc.pages()?;
    for page in pages.iter() {
        for annot in page.annots()? {
            let Some(dict) = doc.resolve(annot)?.as_dict().cloned() else {
                continue;
            };
            if dict.get_name("Subtype") != Some(b"Link") {
                continue;
            }
            if let Some((object, place)) = place_in(doc, annot, &dict, "A", name, store)? {
                out.push(Use {
                    holder: DestinationUse::Link {
                        page: page.id().clone(),
                        annot: AnnotId::object(annot).to_string(),
                    },
                    object,
                    place,
                });
            }
        }
    }
    if let Some(Object::Ref(root)) = doc.trailer().get("Root").cloned() {
        let catalog = doc.catalog()?;
        if let Some((object, place)) = place_in(doc, root, &catalog, "OpenAction", name, store)? {
            out.push(Use {
                holder: DestinationUse::OpenAction,
                object,
                place,
            });
        }
    }
    Ok(out)
}
