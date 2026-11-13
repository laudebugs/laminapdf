//! The destinations and actions of pages copied in from another document (E2 plan §2.2): a
//! link or action going to a page that came along is kept (the copier points it at the copy),
//! one going to a page left behind is removed, and the named destinations kept ones use are
//! copied into the target's `/Names /Dests` tree, renamed `<name>#<n>` on a clash.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, Object, PdfString};
use laminapdf_model::{Copied, ObjectCopier, TreeWriter, set_catalog_entry};

use super::cleanup::{edit_dict, edit_object};
use crate::error::Result;

/// The named destinations copied links use: read from the source, written to the target once.
pub(super) struct Dests<'s, 'c> {
    src: &'s laminapdf_model::Document,
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
    pub(super) fn new(src: &'s laminapdf_model::Document, copier: &'c ObjectCopier<'s>) -> Result<Self> {
        let outline = src.outline()?;
        Ok(Self {
            src,
            outline,
            copier,
            settled: BTreeMap::new(),
            added: BTreeMap::new(),
            taken: None,
        })
    }

    /// Fits a copied link's destination (`/Dest`, or its action's, `/Next` chain included);
    /// `false` when it names a page that didn't come along, so the link goes.
    pub(super) fn fit_link(&mut self, doc: &mut laminapdf_model::Document, link: &mut Dict) -> Result<bool> {
        if let Some(dest) = link.get("Dest").cloned() {
            return Ok(match self.fit(doc, dest)? {
                Some(dest) => {
                    link.insert("Dest", dest);
                    true
                }
                None => false,
            });
        }
        let Some(action) = link.get("A").cloned() else {
            return Ok(true);
        };
        Ok(match self.fit_action(doc, action, 0)? {
            Some(action) => {
                link.insert("A", action);
                true
            }
            None => false,
        })
    }

    /// Fits the actions of `holder`'s `/AA` (an annotation's or a page's triggers), dropping each
    /// that goes to a page that didn't come along.
    pub(super) fn fit_triggers(&mut self, doc: &mut laminapdf_model::Document, holder: &mut Dict) -> Result<()> {
        let Some(Object::Dict(triggers)) = holder
            .get("AA")
            .map(|aa| doc.deref(aa))
            .transpose()?
            .map(laminapdf_cos::Resolved::into_owned)
        else {
            return Ok(());
        };
        let mut kept = Dict::new();
        for (trigger, action) in triggers.iter() {
            if let Some(action) = self.fit_action(doc, action.clone(), 0)? {
                kept.insert(trigger.clone(), action);
            }
        }
        if kept.is_empty() {
            holder.remove("AA");
        } else {
            holder.insert("AA", Object::Dict(kept));
        }
        Ok(())
    }

    /// The copied action `action` as the target should hold it: a `GoTo` with its destination
    /// fitted, its `/Next` actions fitted in turn (those that go nowhere dropped); `None` when
    /// it is a `GoTo` to a page that didn't come along. An action stored as its own object is
    /// edited in place.
    pub(super) fn fit_action(
        &mut self,
        doc: &mut laminapdf_model::Document,
        action: Object,
        depth: usize,
    ) -> Result<Option<Object>> {
        if depth > doc.limits().max_nesting_depth() {
            return Ok(Some(action));
        }
        let (held, mut dict) = match &action {
            Object::Dict(d) => (None, d.clone()),
            Object::Ref(r) => match doc.object(r.num)?.as_dict() {
                Some(d) => (Some(*r), d.clone()),
                None => return Ok(Some(action)),
            },
            _ => return Ok(Some(action)),
        };
        if dict.get_name("S") == Some(b"GoTo")
            && let Some(dest) = dict.get("D").cloned()
        {
            match self.fit(doc, dest)? {
                Some(dest) => dict.insert("D", dest),
                None => return Ok(None),
            }
        }
        match dict.get("Next").cloned() {
            Some(Object::Array(next)) => {
                let mut kept = Vec::with_capacity(next.len());
                for item in next {
                    if let Some(item) = self.fit_action(doc, item, depth + 1)? {
                        kept.push(item);
                    }
                }
                dict.insert("Next", Object::Array(kept));
            }
            Some(next) => match self.fit_action(doc, next, depth + 1)? {
                Some(next) => dict.insert("Next", next),
                None => {
                    dict.remove("Next");
                }
            },
            None => {}
        }
        Ok(Some(match held {
            Some(r) => {
                edit_object(doc, r, |o| *o = Object::Dict(dict))?;
                Object::Ref(r)
            }
            None => Object::Dict(dict),
        }))
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
            // A name object is looked up in the catalog's `/Dests` dictionary and a string in the
            // `/Names /Dests` tree (§12.3.2.4); the copy goes into the tree, so a string names it.
            Object::Name(n) => Ok(self
                .named(doc, n.as_bytes())?
                .map(|n| Object::String(PdfString::literal(n)))),
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
                        // The view's operands may be indirect: they become direct values here,
                        // since their numbers mean nothing in the target.
                        for item in items.iter_mut().skip(1) {
                            if let Object::Ref(r) = item {
                                *item = (*self.src.resolve(*r)?).clone();
                            }
                        }
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

    /// The source destination `dest`, read from the source (an outline item's, which the copier
    /// didn't copy), as the target should hold it: an explicit one pointed at the copy of its
    /// page with its view's operands made direct, a named one renamed as links' are; `None` when
    /// its page didn't come along or it isn't a destination.
    pub(super) fn fit_source(&mut self, doc: &laminapdf_model::Document, dest: &Object) -> Result<Option<Object>> {
        match (*self.src.deref(dest)?).clone() {
            Object::Array(mut items) => {
                let Some(Copied::Object(copy)) = items
                    .first()
                    .and_then(Object::as_ref)
                    .and_then(|p| self.copier.copy_of(p))
                else {
                    return Ok(None);
                };
                for item in items.iter_mut().skip(1) {
                    if let Object::Ref(r) = item {
                        *item = (*self.src.resolve(*r)?).clone();
                    }
                }
                if let Some(first) = items.first_mut() {
                    *first = Object::Ref(copy);
                }
                Ok(Some(Object::Array(items)))
            }
            Object::Dict(d) => match d.get("D") {
                Some(inner) => self.fit_source(doc, inner),
                None => Ok(None),
            },
            Object::Name(n) => Ok(self
                .named(doc, n.as_bytes())?
                .map(|n| Object::String(PdfString::literal(n)))),
            Object::String(s) => Ok(self
                .named(doc, &s.bytes)?
                .map(|n| Object::String(PdfString::literal(n)))),
            _ => Ok(None),
        }
    }

    /// The source's names that came along under another name: (source name, target name).
    pub(super) fn renamed(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.settled
            .iter()
            .filter_map(|(from, to)| match to {
                Some(to) if to != from => Some((from.clone(), to.clone())),
                _ => None,
            })
            .collect()
    }

    /// Adds the copied named destinations to the target's `/Names /Dests` tree.
    pub(super) fn write(self, doc: &mut laminapdf_model::Document) -> Result<()> {
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
