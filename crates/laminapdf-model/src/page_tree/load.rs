//! The editor's in-memory page tree: an arena of `/Pages` nodes and page slots, read once
//! from the document as it is now (overlay over base).

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, Error, ObjRef, Object, Result};

use crate::document::Document;

/// The inheritable page attributes (ISO 32000-2 §7.7.3.4, Table 31), in a fixed order.
pub(super) const INHERITABLE: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];

/// One value per [`INHERITABLE`] key: the value as stored (often a reference), `None` when
/// absent.
pub(super) type Attrs = [Option<Object>; 4];

/// A `/Pages` node.
#[derive(Debug, Clone)]
pub(super) struct Node {
    /// The node object; `None` for a node the editor made, until it is written.
    pub(super) r: Option<ObjRef>,
    /// The node dictionary as read (empty for a new node).
    pub(super) dict: Dict,
    pub(super) parent: Option<usize>,
    pub(super) kids: Vec<Kid>,
    /// The attributes a page under this node inherits (its own and its ancestors').
    pub(super) inherited: Attrs,
    /// Its kids changed, so its `/Kids` and `/Count` are written.
    pub(super) touched: bool,
    /// Still in the tree.
    pub(super) alive: bool,
}

/// A kid of a node.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Kid {
    Node(usize),
    Page(usize),
    /// A kid the editor doesn't edit, kept where it is: a direct dictionary (which §7.7.3.2
    /// doesn't allow), something unreadable or a subtree past the depth limit. `pages` is how
    /// many pages it holds, for `/Count`.
    Other {
        value: Object,
        pages: usize,
    },
}

/// One listing of a page in a `/Kids` array.
#[derive(Debug, Clone)]
pub(super) struct Slot {
    pub(super) r: ObjRef,
    pub(super) dict: Dict,
    /// What the page must keep showing: its effective attributes when read (own, else
    /// inherited), or for an inserted page its own.
    pub(super) desired: Attrs,
    pub(super) parent: Option<usize>,
    /// The page moved (or was inserted, or the tree was rebuilt): its inherited attributes
    /// may differ now.
    pub(super) moved: bool,
}

/// The editor's tree. Node 0 is the root.
#[derive(Debug, Clone)]
pub(super) struct Tree {
    pub(super) nodes: Vec<Node>,
    pub(super) slots: Vec<Slot>,
}

/// An entry still to read: a kid value, the node that lists it and its depth.
struct Pending {
    node: usize,
    depth: usize,
}

impl Tree {
    /// Reads the page tree under the catalog's `/Pages`, which must be an indirect object.
    pub(super) fn load(doc: &Document) -> Result<Self> {
        let catalog = doc.catalog()?;
        let Some(Object::Ref(root)) = catalog.get("Pages") else {
            return Err(Error::Type {
                expected: "indirect reference",
                context: "/Pages",
            });
        };
        let root = *root;
        let dict = node_dict(doc, root)?.ok_or(Error::Type {
            expected: "dictionary",
            context: "/Pages",
        })?;
        let mut tree = Self {
            nodes: Vec::new(),
            slots: Vec::new(),
        };
        let inherited = overlay(doc, &Attrs::default(), &dict)?;
        tree.nodes.push(Node {
            r: Some(root),
            dict,
            parent: None,
            kids: Vec::new(),
            inherited,
            touched: false,
            alive: true,
        });
        let mut visited = BTreeSet::from([root.num]);
        let mut stack = vec![Pending { node: 0, depth: 0 }];
        while let Some(Pending { node, depth }) = stack.pop() {
            for kid in tree.read_kids(doc, node, depth, &mut visited)? {
                if let Kid::Node(child) = kid {
                    stack.push(Pending {
                        node: child,
                        depth: depth + 1,
                    });
                }
            }
        }
        Ok(tree)
    }

    /// Reads node `index`'s `/Kids` into the arena and returns them.
    fn read_kids(
        &mut self,
        doc: &Document,
        index: usize,
        depth: usize,
        visited: &mut BTreeSet<u32>,
    ) -> Result<Vec<Kid>> {
        let Some(node) = self.nodes.get(index) else {
            return Ok(Vec::new());
        };
        let inherited = node.inherited.clone();
        let items = match node.dict.get("Kids") {
            Some(kids) => doc.read(kids)?.and_then(|k| k.as_array().map(<[Object]>::to_vec)),
            None => None,
        }
        .unwrap_or_default();
        let max_depth = doc.limits().max_page_tree_depth();
        let mut kids = Vec::with_capacity(items.len());
        for item in items {
            let kid = match item {
                Object::Ref(r) if depth < max_depth => self.read_kid(doc, index, r, &inherited, visited)?,
                Object::Ref(_) => Kid::Other { value: item, pages: 0 },
                direct => {
                    let pages = direct_pages(doc, &direct, depth + 1, visited)?;
                    Kid::Other { value: direct, pages }
                }
            };
            kids.push(kid);
        }
        if let Some(node) = self.nodes.get_mut(index) {
            node.kids.clone_from(&kids);
        }
        Ok(kids)
    }

    fn read_kid(
        &mut self,
        doc: &Document,
        parent: usize,
        r: ObjRef,
        inherited: &Attrs,
        visited: &mut BTreeSet<u32>,
    ) -> Result<Kid> {
        let Some(dict) = node_dict(doc, r)? else {
            return Ok(Kid::Other {
                value: Object::Ref(r),
                pages: 0,
            });
        };
        if !is_tree_node(&dict) {
            // A page listed twice gets a second slot; `finish` copies it (§7.7.3.2 allows one
            // parent per page).
            let desired = overlay(doc, inherited, &dict)?;
            self.slots.push(Slot {
                r,
                dict,
                desired,
                parent: Some(parent),
                moved: false,
            });
            return Ok(Kid::Page(self.slots.len() - 1));
        }
        if !visited.insert(r.num) {
            return Err(Error::ReferenceCycle(r.num));
        }
        let own = overlay(doc, inherited, &dict)?;
        self.nodes.push(Node {
            r: Some(r),
            dict,
            parent: Some(parent),
            kids: Vec::new(),
            inherited: own,
            touched: false,
            alive: true,
        });
        Ok(Kid::Node(self.nodes.len() - 1))
    }

    /// The page slots in tree order (depth first), those still in the tree only.
    pub(super) fn order(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack: Vec<&Kid> = self
            .nodes
            .first()
            .map(|n| n.kids.iter().rev().collect())
            .unwrap_or_default();
        while let Some(kid) = stack.pop() {
            match kid {
                Kid::Page(slot) => out.push(*slot),
                Kid::Node(node) => {
                    if let Some(node) = self.nodes.get(*node) {
                        stack.extend(node.kids.iter().rev());
                    }
                }
                Kid::Other { .. } => {}
            }
        }
        out
    }

    /// Whether any kid is one the editor keeps as it is ([`Kid::Other`]).
    pub(super) fn has_other_kids(&self) -> bool {
        self.nodes
            .iter()
            .filter(|n| n.alive)
            .any(|n| n.kids.iter().any(|k| matches!(k, Kid::Other { .. })))
    }
}

/// The dictionary of object `r` when it is one.
fn node_dict(doc: &Document, r: ObjRef) -> Result<Option<Dict>> {
    Ok(doc.read(&Object::Ref(r))?.and_then(|o| o.as_dict().cloned()))
}

/// A page-tree node rather than a page, by the reader's rule: `/Type /Pages`, or no
/// `/Type /Page` and a `/Kids`.
pub(super) fn is_tree_node(dict: &Dict) -> bool {
    dict.get_name("Type") == Some(b"Pages") || (dict.get_name("Type") != Some(b"Page") && dict.contains_key("Kids"))
}

/// `inherited` with the attributes `dict` sets itself replacing them. A `null` value, or a
/// reference to `null`, counts as absent (§7.3.7), as the reader takes it.
pub(super) fn overlay(doc: &Document, inherited: &Attrs, dict: &Dict) -> Result<Attrs> {
    let mut out = inherited.clone();
    for (slot, key) in out.iter_mut().zip(INHERITABLE) {
        if let Some(value) = own(doc, dict, key)? {
            *slot = Some(value);
        }
    }
    Ok(out)
}

/// `dict`'s own value for `key`, `None` when absent or null.
pub(super) fn own(doc: &Document, dict: &Dict, key: &str) -> Result<Option<Object>> {
    let Some(value) = dict.get(key) else {
        return Ok(None);
    };
    let is_null = match value {
        Object::Null => true,
        Object::Ref(_) => matches!(doc.read(value)?.as_deref(), Some(Object::Null)),
        _ => false,
    };
    Ok((!is_null).then(|| value.clone()))
}

/// The pages under a direct kid (a page dictionary counts one), for `/Count`. Nodes it
/// reaches by reference join `visited`, so shared or looping subtrees count once.
fn direct_pages(doc: &Document, value: &Object, depth: usize, visited: &mut BTreeSet<u32>) -> Result<usize> {
    let Object::Dict(dict) = value else {
        return Ok(0);
    };
    if !is_tree_node(dict) {
        return Ok(1);
    }
    if depth >= doc.limits().max_page_tree_depth() {
        return Ok(0);
    }
    let Some(kids) = dict.get("Kids") else {
        return Ok(0);
    };
    let mut total = 0usize;
    if let Some(kids) = doc.read(kids)? {
        for kid in kids.as_array().unwrap_or_default() {
            total = total.saturating_add(match kid {
                Object::Dict(_) => direct_pages(doc, kid, depth + 1, visited)?,
                Object::Ref(r) if visited.insert(r.num) => match node_dict(doc, *r)? {
                    Some(d) => direct_pages(doc, &Object::Dict(d), depth + 1, visited)?,
                    None => 0,
                },
                _ => 0,
            });
        }
    }
    Ok(total)
}
