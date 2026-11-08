//! Reads a generated file's structure tree back: how many elements of each kind, and how many
//! paragraphs and tables keep one element across a page break.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_model::{PageId, StructElem, StructKid};

#[derive(Debug, Default)]
pub struct Structure {
    pub kinds: BTreeMap<String, usize>,
    /// `P` and `Table` elements whose content is on more than one page.
    pub across_pages: BTreeMap<String, usize>,
}

fn pages(e: &StructElem, inherited: Option<&PageId>, out: &mut BTreeSet<PageId>) {
    let page = e.page.as_ref().or(inherited);
    for kid in &e.kids {
        match kid {
            StructKid::Elem(child) => pages(child, page, out),
            StructKid::Mcid { page: p, .. } | StructKid::Object { page: p, .. } => {
                if let Some(p) = p.as_ref().or(page) {
                    out.insert(p.clone());
                }
            }
            _ => {}
        }
    }
}

pub fn structure(pdf: &[u8]) -> Structure {
    let doc = laminapdf_model::Document::open(pdf.to_vec(), laminapdf_cos::OpenOptions::default())
        .unwrap_or_else(|e| panic!("open: {e}"));
    let mut out = Structure::default();
    let Ok(Some(tree)) = doc.structure() else { return out };
    for e in tree.elements() {
        let kind = String::from_utf8_lossy(tree.standard_kind(&e.kind)).into_owned();
        *out.kinds.entry(kind.clone()).or_default() += 1;
        if kind == "P" || kind == "Table" {
            let mut set = BTreeSet::new();
            pages(e, None, &mut set);
            if set.len() > 1 {
                *out.across_pages.entry(kind).or_default() += 1;
            }
        }
    }
    out
}
