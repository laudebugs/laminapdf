//! Copying objects from one PDF into another lamina graph with renumbering: the glue that
//! "krilla → bytes → lamina" needs (experiment B, route 1).

use std::collections::{BTreeMap, VecDeque};

use anyhow::{Context, Result, anyhow, bail};
use lamina_cos::{Dict, Document, ObjRef, Object, XrefEol, incremental_update};

/// What an import did, for the README.
#[derive(Debug, Default)]
pub struct ImportReport {
    pub objects_copied: usize,
    pub pages: usize,
    /// Catalog-level entries of the source that were carried over (base had none).
    pub catalog_keys_imported: Vec<String>,
    /// Catalog-level entries of the source that were dropped because the base has its own and
    /// merging them is not implemented (outline/structure-tree merge).
    pub catalog_keys_conflicting: Vec<String>,
    /// Page keys removed because they only make sense with the source's catalog.
    pub page_keys_dropped: Vec<String>,
}

/// The next free object number of `doc` (max of `/Size` and the highest number + 1).
pub fn next_free(doc: &Document<'_>) -> u32 {
    let size = doc.trailer().get_int("Size").unwrap_or(0);
    let max = doc.object_numbers().max().map_or(0, |n| i64::from(n) + 1);
    u32::try_from(size.max(max)).unwrap_or(u32::MAX)
}

/// Appends every page of `src` to the end of `base`'s page tree, as an incremental update of
/// `base`. Catalog-level features of `src` (outline, structure tree, …) are imported when the
/// base lacks them and reported as conflicts otherwise.
pub fn append_pages(base: &[u8], src: &[u8]) -> Result<(Vec<u8>, ImportReport)> {
    let base_doc = lamina_cos::open(base)?;
    let src_doc = lamina_cos::open(src)?;
    let mut report = ImportReport::default();

    let root_ref = base_doc
        .trailer()
        .get("Root")
        .and_then(Object::as_ref)
        .ok_or_else(|| anyhow!("base /Root"))?;
    let mut catalog = base_doc.catalog()?;
    let pages_ref = catalog
        .get("Pages")
        .and_then(Object::as_ref)
        .ok_or_else(|| anyhow!("base /Pages"))?;
    let mut pages_root = base_doc.resolve(pages_ref)?.as_dict().cloned().context("pages root")?;

    let mut cx = Copier {
        src: &src_doc,
        map: BTreeMap::new(),
        next: next_free(&base_doc),
        queue: VecDeque::new(),
        out: Vec::new(),
    };

    // Pre-assign page numbers so destinations and /P entries pointing at pages map onto the
    // copies (and never drag the source page tree along through /Parent).
    let src_pages = src_doc.pages()?;
    let mut new_kids = Vec::new();
    for page in &src_pages {
        let r = page.obj_ref.ok_or_else(|| anyhow!("direct page dictionary"))?;
        new_kids.push(cx.map_ref(r, false));
    }
    let src_catalog = src_doc.catalog()?;
    let tagged = src_catalog.contains_key("StructTreeRoot") && !catalog.contains_key("StructTreeRoot");
    for (page, &new_ref) in src_pages.iter().zip(&new_kids) {
        let mut dict = page.dict.clone();
        // Materialise inherited attributes: the base tree's inheritance must not leak in.
        if let Some(mb) = page.media_box {
            dict.insert("MediaBox", Object::Array(mb.iter().map(|&v| Object::Real(v)).collect()));
        }
        if !dict.contains_key("Resources")
            && let Some(res) = &page.resources
        {
            dict.insert("Resources", (**res).clone());
        }
        dict.insert("Parent", pages_ref);
        if !tagged && dict.remove("StructParents").is_some() {
            report.page_keys_dropped.push("StructParents".into());
        }
        let copied = cx.rewrite(&Object::Dict(dict));
        cx.out.push((new_ref, copied));
    }

    // Catalog-level features.
    for key in [
        "Outlines",
        "StructTreeRoot",
        "MarkInfo",
        "Lang",
        "PageLabels",
        "Names",
        "ViewerPreferences",
    ] {
        let Some(value) = src_catalog.get(key) else { continue };
        if catalog.contains_key(key) {
            report.catalog_keys_conflicting.push(key.into());
            continue;
        }
        if key == "PageLabels" {
            // Page labels are indexed by page number: they would need offsetting by the base's
            // page count. Not done in the spike.
            report.catalog_keys_conflicting.push(key.into());
            continue;
        }
        let copied = cx.rewrite(value);
        catalog.insert(key, copied);
        report.catalog_keys_imported.push(key.into());
    }
    cx.drain()?;

    // Graft into the base page tree.
    let mut kids = pages_root
        .get("Kids")
        .and_then(Object::as_array)
        .map(<[Object]>::to_vec)
        .unwrap_or_default();
    kids.extend(new_kids.iter().map(|&r| Object::Ref(r)));
    let count = pages_root.get_int("Count").unwrap_or(0) + i64::try_from(new_kids.len())?;
    pages_root.insert("Kids", kids);
    pages_root.insert("Count", count);

    report.pages = new_kids.len();
    report.objects_copied = cx.out.len();
    let mut changes = cx.out;
    changes.push((pages_ref, Object::Dict(pages_root)));
    changes.push((root_ref, Object::Dict(catalog)));
    let bytes = incremental_update(base, &changes, XrefEol::default())?;
    Ok((bytes, report))
}

struct Copier<'d, 'a> {
    src: &'d Document<'a>,
    /// Source object number → new reference in the base's numbering space.
    map: BTreeMap<u32, ObjRef>,
    next: u32,
    /// Source objects discovered but not yet copied.
    queue: VecDeque<(ObjRef, ObjRef)>,
    out: Vec<(ObjRef, Object)>,
}

impl Copier<'_, '_> {
    fn map_ref(&mut self, r: ObjRef, enqueue: bool) -> ObjRef {
        if let Some(&n) = self.map.get(&r.num) {
            return n;
        }
        let new = ObjRef::new(self.next, 0);
        self.next += 1;
        self.map.insert(r.num, new);
        if enqueue {
            self.queue.push_back((r, new));
        }
        new
    }

    fn rewrite(&mut self, obj: &Object) -> Object {
        match obj {
            Object::Ref(r) => Object::Ref(self.map_ref(*r, true)),
            Object::Array(items) => Object::Array(items.iter().map(|o| self.rewrite(o)).collect()),
            Object::Dict(d) => Object::Dict(self.rewrite_dict(d)),
            Object::Stream(s) => {
                let mut s = s.clone();
                s.dict = self.rewrite_dict(&s.dict);
                Object::Stream(s)
            }
            other => other.clone(),
        }
    }

    fn rewrite_dict(&mut self, d: &Dict) -> Dict {
        let mut out = Dict::new();
        for (k, v) in d.iter() {
            out.insert(k.clone(), self.rewrite(v));
        }
        out
    }

    fn drain(&mut self) -> Result<()> {
        while let Some((old, new)) = self.queue.pop_front() {
            let obj = self.src.resolve(old)?;
            if matches!(*obj, Object::Null) {
                bail!("dangling reference {old}");
            }
            let copied = self.rewrite(&obj);
            self.out.push((new, copied));
        }
        Ok(())
    }
}
