//! Name trees and number trees (ISO 32000-2 §7.9.6, §7.9.7): lookup and ordered iteration.
//!
//! Both walks are iterative and tolerant: a kid that is missing, malformed, reached twice or
//! deeper than [`Limits::max_tree_depth`](lamina_cos::Limits::max_tree_depth) is skipped with
//! [`Warning::NameTreeNodeSkipped`]; so is a key of the wrong type. `/Limits` is only used to
//! prune a lookup, and only when it is well formed; a node without it is always searched.

use std::collections::BTreeSet;

use lamina_cos::{Object, Result, Warning};

use crate::document::{Document, object_number};

/// Which kind of tree: the key array's name and how to read a key.
trait TreeKind {
    type Key: Ord + Clone;
    const ENTRIES: &'static str;
    fn key(obj: &Object) -> Option<Self::Key>;
}

struct Names;
struct Nums;

impl TreeKind for Names {
    type Key = Vec<u8>;
    const ENTRIES: &'static str = "Names";
    fn key(obj: &Object) -> Option<Vec<u8>> {
        match obj {
            Object::String(s) => Some(s.bytes.clone()),
            // Some writers use names; accept them (they are byte strings too).
            Object::Name(n) => Some(n.as_bytes().to_vec()),
            _ => None,
        }
    }
}

impl TreeKind for Nums {
    type Key = i64;
    const ENTRIES: &'static str = "Nums";
    fn key(obj: &Object) -> Option<i64> {
        obj.as_int()
    }
}

/// Walks the tree at `root` in tree order, collecting the entries whose key is `target` (or
/// every entry when `target` is `None`).
fn walk<K: TreeKind>(doc: &Document, root: &Object, target: Option<&K::Key>) -> Result<Vec<(K::Key, Object)>> {
    let max_depth = doc.limits().max_tree_depth();
    let mut visited = BTreeSet::new();
    let mut stack = vec![(root.clone(), 0usize)];
    let mut out = Vec::new();
    while let Some((node_obj, depth)) = stack.pop() {
        let skip = || {
            doc.warn(Warning::NameTreeNodeSkipped {
                object: object_number(&node_obj),
            })
        };
        if depth > max_depth {
            skip()?;
            continue;
        }
        if let Some(r) = node_obj.as_ref()
            && !visited.insert(r.num)
        {
            skip()?;
            continue;
        }
        let Some(node) = doc.read(&node_obj)? else {
            skip()?;
            continue;
        };
        let Some(node) = node.as_dict() else {
            skip()?;
            continue;
        };
        if let Some(entries) = node.get(K::ENTRIES) {
            let entries = doc.read(entries)?;
            match entries.as_deref().and_then(Object::as_array) {
                Some(items) => {
                    let (pairs, rest) = items.as_chunks::<2>();
                    for [key, value] in pairs {
                        let key = doc.read(key)?.and_then(|k| K::key(&k));
                        match key {
                            Some(key) if target.is_none_or(|t| *t == key) => out.push((key, value.clone())),
                            Some(_) => {}
                            None => skip()?,
                        }
                    }
                    if !rest.is_empty() {
                        skip()?;
                    }
                }
                None => skip()?,
            }
        }
        if let Some(kids) = node.get("Kids") {
            let kids = doc.read(kids)?;
            let Some(kids) = kids.as_deref().and_then(Object::as_array) else {
                skip()?;
                continue;
            };
            for kid in kids.iter().rev() {
                if let Some(target) = target
                    && !may_contain::<K>(doc, kid, target)?
                {
                    continue;
                }
                stack.push((kid.clone(), depth + 1));
            }
        }
    }
    Ok(out)
}

/// False only when `kid` has a well-formed `/Limits` that excludes `target`.
fn may_contain<K: TreeKind>(doc: &Document, kid: &Object, target: &K::Key) -> Result<bool> {
    let Some(kid) = doc.read(kid)? else {
        return Ok(true);
    };
    let Some(limits) = kid.as_dict().and_then(|d| d.get("Limits")) else {
        return Ok(true);
    };
    let Some(limits) = doc.read(limits)? else {
        return Ok(true);
    };
    let bounds = match limits.as_array() {
        Some([lo, hi]) => (doc.read(lo)?, doc.read(hi)?),
        _ => return Ok(true),
    };
    match bounds {
        (Some(lo), Some(hi)) => match (K::key(&lo), K::key(&hi)) {
            (Some(lo), Some(hi)) if lo <= hi => Ok(lo <= *target && *target <= hi),
            _ => Ok(true),
        },
        _ => Ok(true),
    }
}

/// A name tree (§7.9.6): string keys, in tree order, mapped to values.
#[derive(Debug, Clone)]
pub struct NameTree<'d> {
    doc: &'d Document,
    root: Object,
}

impl<'d> NameTree<'d> {
    /// The name tree rooted at `root` (a dictionary or a reference to one).
    #[must_use]
    pub const fn new(doc: &'d Document, root: Object) -> Self {
        Self { doc, root }
    }

    /// The value stored under `key` (as stored: often a reference), if any. When the tree
    /// holds the key twice, the first in tree order wins.
    ///
    /// # Errors
    /// [`lamina_cos::Error::Strict`] in strict mode when a node is skipped;
    /// [`lamina_cos::Error::DataPending`] / [`lamina_cos::Error::Source`].
    pub fn get(&self, key: &[u8]) -> Result<Option<Object>> {
        let key = key.to_vec();
        Ok(walk::<Names>(self.doc, &self.root, Some(&key))?
            .into_iter()
            .next()
            .map(|(_, v)| v))
    }

    /// Every entry in tree order (which ISO 32000 requires to be sorted by key).
    ///
    /// # Errors
    /// As [`NameTree::get`].
    pub fn entries(&self) -> Result<Vec<(Vec<u8>, Object)>> {
        walk::<Names>(self.doc, &self.root, None)
    }
}

/// A number tree (§7.9.7): integer keys, in tree order, mapped to values.
#[derive(Debug, Clone)]
pub struct NumberTree<'d> {
    doc: &'d Document,
    root: Object,
}

impl<'d> NumberTree<'d> {
    /// The number tree rooted at `root` (a dictionary or a reference to one).
    #[must_use]
    pub const fn new(doc: &'d Document, root: Object) -> Self {
        Self { doc, root }
    }

    /// The value stored under `key` (as stored), if any; the first in tree order wins.
    ///
    /// # Errors
    /// As [`NameTree::get`].
    pub fn get(&self, key: i64) -> Result<Option<Object>> {
        Ok(walk::<Nums>(self.doc, &self.root, Some(&key))?
            .into_iter()
            .next()
            .map(|(_, v)| v))
    }

    /// Every entry in tree order (which ISO 32000 requires to be sorted by key).
    ///
    /// # Errors
    /// As [`NameTree::get`].
    pub fn entries(&self) -> Result<Vec<(i64, Object)>> {
        walk::<Nums>(self.doc, &self.root, None)
    }
}
