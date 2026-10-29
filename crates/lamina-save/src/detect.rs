//! Finding signatures and their restrictions in a document ([ADR 0007] §2; ISO 32000-2
//! §12.7.5.5, §12.8; research ch. 04 §8), and the changes the explicit override makes to
//! remove the ones a save invalidates.
//!
//! * Signature fields: the `/AcroForm /Fields` tree (with `/Kids`, `/FT` inherited) for
//!   fields of type `/Sig` whose `/V` is a dictionary; their `/Lock` (`FieldMDP`, and its
//!   PDF 2.0 `/P`, Table 236).
//! * `/Perms`: `/DocMDP` (the certification signature) and `/UR3` (or the older `/UR`).
//! * Each signature dictionary's `/Type` (`/DocTimeStamp`) and `/Reference` transforms
//!   (`/DocMDP` with `/P`, `/FieldMDP`, `/UR3` with the rights it grants).
//! * A repaired base: every object is checked for a signature dictionary (`/Type /Sig` or
//!   `/DocTimeStamp`, or, since `/Type` is optional, `/ByteRange` with a string `/Contents`),
//!   because the catalog repair picks may not reach a signature the damaged revision added.
//!
//! Nothing is assumed absent: a branch of the field tree that can't be followed (too deep, or
//! not an array or dictionary where one belongs) makes the signature state unknown, and a
//! certification whose `DocMDP` permission can't be read is taken as the strictest (`/P 1`).
//!
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use std::collections::{BTreeMap, BTreeSet};

use lamina_cos::{Dict, DirectPath, Document, ObjRef, Object, PathSegment};
use lamina_model::FieldId;
use lamina_ops::ModificationClass;

use crate::constraint::{DocMdp, DocMdpPermission, Restriction, SignatureInfo, SignatureKind, Violation};
use crate::error::Result;

/// What a signature forbids.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Rule {
    /// A `DocMDP` permission (a certification, or a PDF 2.0 field lock's `/P`); `assumed` when
    /// it couldn't be read and the strictest was taken.
    DocMdp {
        permission: DocMdpPermission,
        assumed: bool,
    },
    /// A `FieldMDP` lock that may cover fields.
    FieldMdp,
    /// A usage-rights signature, with the op classes its rights cover.
    UsageRights(BTreeSet<ModificationClass>),
}

impl Rule {
    fn allows(&self, class: ModificationClass) -> bool {
        match self {
            Self::DocMdp { permission, .. } => permission.allows(class),
            // A lock covers named fields, and op classes don't name fields: any form change
            // may touch a locked field (ADR 0007 §9).
            Self::FieldMdp => class != ModificationClass::FormFill,
            Self::UsageRights(granted) => granted.contains(&class),
        }
    }

    const fn restriction(&self) -> Restriction {
        match self {
            Self::DocMdp { permission, .. } => Restriction::DocMdp(*permission),
            Self::FieldMdp => Restriction::FieldMdp,
            Self::UsageRights(_) => Restriction::UsageRights,
        }
    }
}

/// One signature found, with what the override needs to remove it.
#[derive(Debug, Clone)]
struct Found {
    info: SignatureInfo,
    rules: Vec<Rule>,
    /// The value as stored (a reference, or a direct signature dictionary): how `/Perms`
    /// entries are matched.
    value: Object,
    /// Where the field dictionary holding it is.
    field: Option<DirectPath>,
}

/// The signatures of a document and the restrictions they carry.
#[derive(Debug, Clone, Default)]
pub(crate) struct Signatures {
    found: Vec<Found>,
    /// A branch of the field tree couldn't be followed: there may be signatures nobody saw.
    pub(crate) unknown: bool,
    /// Where the `/Perms` dictionary is, and where the catalog is (to drop an emptied
    /// `/Perms`).
    perms: Option<(DirectPath, DirectPath)>,
}

/// Objects (by number) and the trailer as the override leaves them: what the writers write
/// instead of the document's own values.
#[derive(Debug, Clone, Default)]
pub(crate) struct Patches {
    pub(crate) objects: BTreeMap<u32, Object>,
    pub(crate) trailer: Option<Dict>,
}

impl Patches {
    pub(crate) fn is_empty(&self) -> bool {
        self.objects.is_empty() && self.trailer.is_none()
    }
}

impl Signatures {
    /// Every signature, in the order found.
    pub(crate) fn infos(&self) -> Vec<SignatureInfo> {
        self.found.iter().map(|f| f.info.clone()).collect()
    }

    /// The signatures that restrict later changes (certification, usage rights, locks): an
    /// incremental update that removes any signature's value breaks them too.
    pub(crate) fn restricting(&self) -> impl Iterator<Item = &SignatureInfo> + '_ {
        self.found.iter().filter(|f| !f.rules.is_empty()).map(|f| &f.info)
    }

    /// The certification signature's restriction, if there is one.
    pub(crate) fn doc_mdp(&self) -> Option<DocMdp> {
        self.found.iter().find_map(|f| {
            f.rules.iter().find_map(|rule| match rule {
                Rule::DocMdp { permission, assumed } if f.info.kind == SignatureKind::Certification => Some(DocMdp {
                    permission: *permission,
                    assumed: *assumed,
                    signature: f.info.clone(),
                }),
                _ => None,
            })
        })
    }

    /// The restrictions a change of `class` breaks.
    pub(crate) fn violations(&self, class: ModificationClass) -> impl Iterator<Item = Violation> + '_ {
        self.found.iter().flat_map(move |f| {
            f.rules
                .iter()
                .filter(move |rule| !rule.allows(class))
                .map(move |rule| Violation::new(class, rule.restriction(), f.info.clone()))
        })
    }

    /// The changes that remove `invalidated` ([ADR 0007] §3): `/V` from each one's field, and
    /// the `/Perms` entries pointing at one (`/Perms` itself when nothing is left), on the
    /// document as it is now. Direct fields and a direct `/Perms` are edited in the object
    /// that contains them.
    ///
    /// # Errors
    /// Errors loading an object to edit.
    ///
    /// [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
    pub(crate) fn patches(&self, doc: &Document, invalidated: &[SignatureInfo]) -> Result<Patches> {
        let mut edit = Editor::new(doc);
        let targets: Vec<&Found> = self.found.iter().filter(|f| invalidated.contains(&f.info)).collect();
        for found in &targets {
            if let Some(path) = &found.field {
                edit.with(path, |field| {
                    if let Object::Dict(d) = field {
                        d.remove("V").is_some()
                    } else {
                        false
                    }
                })?;
            }
        }
        if let Some((perms, catalog)) = &self.perms {
            let mut emptied = false;
            edit.with(perms, |p| {
                let Object::Dict(d) = p else { return false };
                let before = d.len();
                for key in ["DocMDP", "UR3", "UR"] {
                    if d.get(key).is_some_and(|v| targets.iter().any(|f| f.value == *v)) {
                        d.remove(key);
                    }
                }
                emptied = d.is_empty() && d.len() < before;
                d.len() < before
            })?;
            if emptied {
                edit.with(catalog, |c| match c {
                    Object::Dict(d) => d.remove("Perms").is_some(),
                    _ => false,
                })?;
            }
        }
        Ok(edit.finish())
    }

    fn add(&mut self, found: Found) {
        self.found.push(found);
    }
}

/// Edits objects (and the trailer) by [`DirectPath`], keeping only those that changed.
struct Editor<'d> {
    doc: &'d Document,
    objects: BTreeMap<u32, (Object, bool)>,
    trailer: Option<(Object, bool)>,
}

impl<'d> Editor<'d> {
    const fn new(doc: &'d Document) -> Self {
        Self {
            doc,
            objects: BTreeMap::new(),
            trailer: None,
        }
    }

    /// Runs `f` on the value at `path`; `f` says whether it changed it.
    fn with(&mut self, path: &DirectPath, f: impl FnOnce(&mut Object) -> bool) -> Result<()> {
        let (root, changed) = if let Some(r) = path.anchor {
            if !self.objects.contains_key(&r.num) {
                let value = (*self.doc.object(r.num)?).clone();
                self.objects.insert(r.num, (value, false));
            }
            let Some((value, changed)) = self.objects.get_mut(&r.num) else {
                return Ok(());
            };
            (value, changed)
        } else {
            let (value, changed) = self
                .trailer
                .get_or_insert_with(|| (Object::Dict(self.doc.trailer().clone()), false));
            (value, changed)
        };
        if let Some(target) = at_mut(root, &path.segments)
            && f(target)
        {
            *changed = true;
        }
        Ok(())
    }

    fn finish(self) -> Patches {
        Patches {
            objects: self
                .objects
                .into_iter()
                .filter(|(_, (_, changed))| *changed)
                .map(|(num, (value, _))| (num, value))
                .collect(),
            trailer: self.trailer.and_then(|(value, changed)| match value {
                Object::Dict(d) if changed => Some(d),
                _ => None,
            }),
        }
    }
}

/// The value at `segments` inside `object`.
fn at_mut<'o>(object: &'o mut Object, segments: &[PathSegment]) -> Option<&'o mut Object> {
    let mut current = object;
    for step in segments {
        current = match (step, current) {
            (PathSegment::Key(k), Object::Dict(d)) => d.get_mut(k)?,
            (PathSegment::Index(i), Object::Array(items)) => items.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(current)
}

/// Where `value`, found at `step` below `parent`, lives: its own object when it is a
/// reference, else inside `parent`.
fn locate(parent: &DirectPath, step: PathSegment, value: &Object) -> DirectPath {
    if let Object::Ref(r) = value {
        DirectPath::new(Some(*r), Vec::new())
    } else {
        let mut segments = parent.segments.clone();
        segments.push(step);
        DirectPath::new(parent.anchor, segments)
    }
}

/// The `/Perms` entries (§12.8.4): which signature values certify or grant rights.
#[derive(Default)]
struct Perms {
    doc_mdp: Option<Object>,
    usage_rights: Option<Object>,
}

/// Finds every signature of `doc` as it is now (base plus overlay).
///
/// # Errors
/// Errors reading the catalog, the form or a signature dictionary (unavailable input, strict
/// mode, limits, corrupt objects): a signature that can't be read is not assumed absent. In a
/// repaired file's scan, an object that doesn't parse is skipped (it is no signature
/// dictionary a viewer could show), but unavailable input still fails.
pub(crate) fn signatures(doc: &Document) -> Result<Signatures> {
    let catalog = doc.catalog()?;
    let catalog_path = locate(
        &DirectPath::new(None, Vec::new()),
        PathSegment::Key("Root"),
        doc.trailer().get("Root").unwrap_or(&Object::Null),
    );
    let mut found = Signatures::default();
    let perms = perms(doc, &catalog, &catalog_path, &mut found)?;
    let mut seen: Vec<Object> = Vec::new();
    fields(doc, &catalog, &catalog_path, &perms, &mut found, &mut seen)?;
    for (kind, value) in [
        (SignatureKind::Certification, &perms.doc_mdp),
        (SignatureKind::UsageRights, &perms.usage_rights),
    ] {
        let Some(value) = value else { continue };
        if seen.contains(value) {
            continue;
        }
        let resolved = doc.deref(value)?;
        match &*resolved {
            Object::Dict(dict) => {
                let (mut info, rules) = classify(doc, dict, value, &perms)?;
                info.kind = kind;
                found.add(Found {
                    info,
                    rules,
                    value: value.clone(),
                    field: None,
                });
            }
            Object::Null => {}
            _ => found.unknown = true,
        }
        seen.push(value.clone());
    }
    if doc.was_repaired() {
        scan(doc, &perms, &mut found, &seen)?;
    }
    Ok(found)
}

fn perms(doc: &Document, catalog: &Dict, catalog_path: &DirectPath, found: &mut Signatures) -> Result<Perms> {
    let mut out = Perms::default();
    let Some(raw) = catalog.get("Perms") else {
        return Ok(out);
    };
    let resolved = doc.deref(raw)?;
    let perms = match &*resolved {
        Object::Dict(d) => d,
        Object::Null => return Ok(out),
        _ => {
            found.unknown = true;
            return Ok(out);
        }
    };
    found.perms = Some((
        locate(catalog_path, PathSegment::Key("Perms"), raw),
        catalog_path.clone(),
    ));
    out.doc_mdp = perms.get("DocMDP").cloned();
    out.usage_rights = perms.get("UR3").or_else(|| perms.get("UR")).cloned();
    Ok(out)
}

/// The value at `key` of `dict` as an array, resolved: `Ok(None)` when absent or `null`,
/// `Err(())` when it is something else (the walk can't follow it).
fn array_at(doc: &Document, value: Option<&Object>) -> Result<core::result::Result<Option<Vec<Object>>, ()>> {
    let Some(value) = value else { return Ok(Ok(None)) };
    let resolved = doc.deref(value)?;
    Ok(match &*resolved {
        Object::Array(items) => Ok(Some(items.clone())),
        Object::Null => Ok(None),
        _ => Err(()),
    })
}

/// One node to visit: its value as stored, where it lives, the inherited `/FT /Sig`, depth.
struct Node {
    value: Object,
    path: DirectPath,
    inherited_sig: bool,
    depth: usize,
}

/// Walks the field tree for signed signature fields.
fn fields(
    doc: &Document,
    catalog: &Dict,
    catalog_path: &DirectPath,
    perms: &Perms,
    found: &mut Signatures,
    seen: &mut Vec<Object>,
) -> Result<()> {
    let Some(form_raw) = catalog.get("AcroForm") else {
        return Ok(());
    };
    let form = doc.deref(form_raw)?;
    let form_dict = match &*form {
        Object::Dict(d) => d,
        Object::Null => return Ok(()),
        _ => {
            found.unknown = true;
            return Ok(());
        }
    };
    let form_path = locate(catalog_path, PathSegment::Key("AcroForm"), form_raw);
    let fields_raw = form_dict.get("Fields");
    let Ok(roots) = array_at(doc, fields_raw)? else {
        found.unknown = true;
        return Ok(());
    };
    let roots_path = locate(
        &form_path,
        PathSegment::Key("Fields"),
        fields_raw.unwrap_or(&Object::Null),
    );
    let max_depth = doc.limits().max_tree_depth();
    let mut stack: Vec<Node> = roots
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .rev()
        .map(|(i, value)| Node {
            path: locate(&roots_path, PathSegment::Index(i), &value),
            value,
            inherited_sig: false,
            depth: 1,
        })
        .collect();
    let mut visited = BTreeSet::new();
    while let Some(node) = stack.pop() {
        let reference = node.value.as_ref();
        if let Some(r) = reference
            && !visited.insert(r.num)
        {
            continue;
        }
        let resolved = doc.deref(&node.value)?;
        let field = match &*resolved {
            Object::Dict(d) => d,
            Object::Null => continue,
            _ => {
                found.unknown = true;
                continue;
            }
        };
        let is_sig = field.get_name("FT").map_or(node.inherited_sig, |ft| ft == b"Sig");
        if is_sig && let Some(value) = field.get("V") {
            let resolved_value = doc.deref(value)?;
            match &*resolved_value {
                Object::Dict(dict) => {
                    let (mut info, mut rules) = classify(doc, dict, value, perms)?;
                    info.field = reference.map(FieldId::object);
                    for rule in lock_rules(doc, field)? {
                        if !rules.contains(&rule) {
                            rules.push(rule);
                        }
                    }
                    if !seen.contains(value) {
                        seen.push(value.clone());
                        found.add(Found {
                            info,
                            rules,
                            value: value.clone(),
                            field: Some(node.path.clone()),
                        });
                    }
                }
                Object::Null => {}
                _ => found.unknown = true,
            }
        }
        let kids_raw = field.get("Kids");
        let Ok(kids) = array_at(doc, kids_raw)? else {
            found.unknown = true;
            continue;
        };
        let Some(kids) = kids.filter(|k| !k.is_empty()) else {
            continue;
        };
        if node.depth >= max_depth {
            // Kids nobody looked at may be signed.
            found.unknown = true;
            continue;
        }
        let kids_path = locate(&node.path, PathSegment::Key("Kids"), kids_raw.unwrap_or(&Object::Null));
        stack.extend(kids.into_iter().enumerate().rev().map(|(i, value)| Node {
            path: locate(&kids_path, PathSegment::Index(i), &value),
            value,
            inherited_sig: is_sig,
            depth: node.depth + 1,
        }));
    }
    Ok(())
}

/// The restrictions of a signed field's `/Lock` (§12.7.5.5, Table 236): a `FieldMDP` lock when
/// it may cover fields, and its PDF 2.0 `/P` as a `DocMDP` permission.
fn lock_rules(doc: &Document, field: &Dict) -> Result<Vec<Rule>> {
    let Some(lock) = field.get("Lock") else {
        return Ok(Vec::new());
    };
    let lock = doc.deref(lock)?;
    let Some(lock) = lock.as_dict() else {
        return Ok(Vec::new());
    };
    let mut rules = Vec::new();
    if locks_fields(lock) {
        rules.push(Rule::FieldMdp);
    }
    if let Some(p) = lock.get("P") {
        rules.push(docmdp_rule(p.as_int()));
    }
    Ok(rules)
}

/// Whether a lock (or `FieldMDP` transform parameters) may cover any field: all but
/// `/Action /Include` with no `/Fields` (§12.8.2.4, Table 258).
fn locks_fields(params: &Dict) -> bool {
    let empty_include = params.get_name("Action") == Some(b"Include")
        && params
            .get("Fields")
            .and_then(Object::as_array)
            .is_none_or(<[Object]>::is_empty);
    !empty_include
}

/// The `DocMDP` rule for a `/P` value: absent means 2 (Table 257); a value that isn't 1, 2 or 3
/// is taken as the strictest, 1.
fn docmdp_rule(p: Option<i64>) -> Rule {
    match p {
        None => Rule::DocMdp {
            permission: DocMdpPermission::FormFill,
            assumed: false,
        },
        Some(p) => DocMdpPermission::from_p(p).map_or(
            Rule::DocMdp {
                permission: DocMdpPermission::NoChanges,
                assumed: true,
            },
            |permission| Rule::DocMdp {
                permission,
                assumed: false,
            },
        ),
    }
}

/// Whether `dict` is a signature dictionary (§12.8.1, Table 255): `/Type /Sig` or
/// `/DocTimeStamp`, or, since `/Type` is optional, a `/ByteRange` array with a string
/// `/Contents`.
fn is_signature(dict: &Dict) -> bool {
    matches!(dict.get_name("Type"), Some(b"Sig" | b"DocTimeStamp"))
        || (dict.get("ByteRange").and_then(Object::as_array).is_some()
            && matches!(dict.get("Contents"), Some(Object::String(_))))
}

/// Checks every object of a repaired file for a signature dictionary nothing else reached.
fn scan(doc: &Document, perms: &Perms, found: &mut Signatures, seen: &[Object]) -> Result<()> {
    let numbers: Vec<u32> = doc.object_numbers().collect();
    for num in numbers {
        // Objects created in this session have generation 0.
        let reference = ObjRef::new(num, doc.base_generation(num).unwrap_or(0));
        if seen.iter().any(|v| v.as_ref().is_some_and(|r| r.num == num)) {
            continue;
        }
        let object = match doc.object(num) {
            Ok(object) => object,
            Err(e @ (lamina_cos::Error::DataPending { .. } | lamina_cos::Error::Source(_))) => return Err(e.into()),
            Err(_) => continue,
        };
        let Some(dict) = object.as_dict() else {
            continue;
        };
        if is_signature(dict) {
            let value = Object::Ref(reference);
            let (info, rules) = classify(doc, dict, &value, perms)?;
            found.add(Found {
                info,
                rules,
                value,
                field: None,
            });
        }
    }
    Ok(())
}

/// What the signature dictionary `dict` (stored as `value`) is, and the restrictions its
/// transforms carry.
fn classify(doc: &Document, dict: &Dict, value: &Object, perms: &Perms) -> Result<(SignatureInfo, Vec<Rule>)> {
    let mut kind = if dict.get_name("Type") == Some(b"DocTimeStamp") {
        SignatureKind::DocTimeStamp
    } else {
        SignatureKind::Approval
    };
    let mut rules = Vec::new();
    let certifies = perms.doc_mdp.as_ref() == Some(value);
    let grants = perms.usage_rights.as_ref() == Some(value);
    if let Some(references) = dict.get("Reference") {
        let references = doc.deref(references)?;
        for item in references.as_array().unwrap_or_default() {
            let item = doc.deref(item)?;
            let Some(sig_ref) = item.as_dict() else {
                continue;
            };
            let params = match sig_ref.get("TransformParams") {
                Some(p) => doc.deref(p)?.as_dict().cloned(),
                None => Some(Dict::new()),
            };
            match sig_ref.get_name("TransformMethod") {
                Some(b"DocMDP") => {
                    kind = SignatureKind::Certification;
                    rules.push(params.map_or(
                        Rule::DocMdp {
                            permission: DocMdpPermission::NoChanges,
                            assumed: true,
                        },
                        |p| docmdp_rule(p.get_int("P")),
                    ));
                }
                Some(b"UR3" | b"UR") => {
                    kind = SignatureKind::UsageRights;
                    rules.push(Rule::UsageRights(params.as_ref().map(granted).unwrap_or_default()));
                }
                Some(b"FieldMDP") if params.as_ref().is_none_or(locks_fields) => rules.push(Rule::FieldMdp),
                _ => {}
            }
        }
    }
    if certifies && !rules.iter().any(|r| matches!(r, Rule::DocMdp { .. })) {
        // Certified in `/Perms` but no readable DocMDP transform: the strictest permission.
        kind = SignatureKind::Certification;
        rules.push(Rule::DocMdp {
            permission: DocMdpPermission::NoChanges,
            assumed: true,
        });
    }
    if grants && !rules.iter().any(|r| matches!(r, Rule::UsageRights(_))) {
        kind = SignatureKind::UsageRights;
        rules.push(Rule::UsageRights(BTreeSet::new()));
    }
    let covers = byte_range_end(dict);
    let revision = covers.and_then(|end| revision(doc, end));
    Ok((
        SignatureInfo {
            field: None,
            kind,
            value: value.as_ref(),
            revision,
            covers,
        },
        rules,
    ))
}

/// The op classes a UR3 transform's rights cover (§12.8.2.3, Table 260): `/Form [/FillIn]`
/// form filling, `/Annots` with `/Create`, `/Delete` or `/Modify` annotations, and
/// `/Signature [/Modify]` signing. Everything else is outside the rights.
fn granted(params: &Dict) -> BTreeSet<ModificationClass> {
    let has = |key: &str, rights: &[&[u8]]| -> bool {
        params
            .get(key)
            .and_then(Object::as_array)
            .unwrap_or_default()
            .iter()
            .any(|o| matches!(o, Object::Name(n) if rights.contains(&n.as_bytes())))
    };
    let mut out = BTreeSet::new();
    if has("Form", &[b"FillIn"]) {
        out.insert(ModificationClass::FormFill);
    }
    if has("Annots", &[b"Create", b"Delete", b"Modify"]) {
        out.insert(ModificationClass::Annotation);
    }
    if has("Signature", &[b"Modify"]) {
        out.insert(ModificationClass::Signature);
    }
    out
}

/// The end of the bytes a `/ByteRange [a b c d]` covers: `c + d`.
fn byte_range_end(dict: &Dict) -> Option<u64> {
    let range = dict.get("ByteRange")?.as_array()?;
    let [_, _, c, d] = range else { return None };
    let (c, d) = (u64::try_from(c.as_int()?).ok()?, u64::try_from(d.as_int()?).ok()?);
    c.checked_add(d)
}

/// The 1-based revision a signature covering the first `end` bytes closes: the xref sections
/// of the `/Prev` chain that start before `end`, oldest first, where a section that starts
/// before the older one it points to (a linearized file's first-page section) belongs to that
/// one's revision. `None` for a repaired base (no chain) or when no section starts before
/// `end`.
fn revision(doc: &Document, end: u64) -> Option<usize> {
    revision_of(doc.xref_section_offsets(), end)
}

/// [`revision`] over the section offsets, newest first.
fn revision_of(offsets: &[usize], end: u64) -> Option<usize> {
    let mut count = 0usize;
    let mut older: Option<u64> = None;
    for &offset in offsets.iter().rev() {
        let offset = u64::try_from(offset).ok()?;
        if offset >= end {
            break;
        }
        if older.is_none_or(|o| offset > o) {
            count += 1;
        }
        older = Some(offset);
    }
    (count > 0).then_some(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_section_before_the_end_is_a_revision() {
        // Newest first: an update at 900 after an original at 500.
        assert_eq!(
            (revision_of(&[900, 500], 1000), revision_of(&[900, 500], 800)),
            (Some(2), Some(1))
        );
    }

    #[test]
    fn a_linearized_files_first_page_section_joins_its_revision() {
        // Newest first: an update at 2000, the first-page section at 100 pointing to the main
        // section at 1500, both part of the original revision.
        assert_eq!(revision_of(&[2000, 100, 1500], 1800), Some(1));
        assert_eq!(revision_of(&[2000, 100, 1500], 2100), Some(2));
    }

    #[test]
    fn no_section_before_the_end_is_no_revision() {
        assert_eq!((revision_of(&[500], 10), revision_of(&[], 10)), (None, None));
    }

    #[test]
    fn ur3_rights_need_the_rights_that_cover_each_class() {
        let params: Dict = [
            ("Annots", Object::Array(vec![Object::name("Online")])),
            ("Form", Object::Array(vec![Object::name("Add"), Object::name("Delete")])),
        ]
        .into_iter()
        .collect();
        assert!(granted(&params).is_empty());
        let params: Dict = [
            ("Annots", Object::Array(vec![Object::name("Modify")])),
            ("Form", Object::Array(vec![Object::name("FillIn")])),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            granted(&params).into_iter().collect::<Vec<_>>(),
            [ModificationClass::FormFill, ModificationClass::Annotation]
        );
    }

    #[test]
    fn a_lock_including_no_field_locks_nothing() {
        let include_none: Dict = [("Action", Object::name("Include"))].into_iter().collect();
        let all: Dict = [("Action", Object::name("All"))].into_iter().collect();
        assert_eq!((locks_fields(&include_none), locks_fields(&all)), (false, true));
    }

    #[test]
    fn a_docmdp_p_outside_1_to_3_is_the_strictest() {
        assert_eq!(
            (docmdp_rule(Some(7)), docmdp_rule(None)),
            (
                Rule::DocMdp {
                    permission: DocMdpPermission::NoChanges,
                    assumed: true
                },
                Rule::DocMdp {
                    permission: DocMdpPermission::FormFill,
                    assumed: false
                }
            )
        );
    }

    #[test]
    fn a_signature_dictionary_needs_no_type() {
        let untyped: Dict = [
            ("ByteRange", Object::Array(vec![0.into(), 1.into(), 2.into(), 3.into()])),
            ("Contents", Object::String(lamina_cos::PdfString::hex(vec![0]))),
        ]
        .into_iter()
        .collect();
        let field: Dict = [("FT", Object::name("Sig"))].into_iter().collect();
        assert_eq!((is_signature(&untyped), is_signature(&field)), (true, false));
    }
}
