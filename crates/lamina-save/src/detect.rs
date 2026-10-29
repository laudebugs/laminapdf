//! Finding signatures and their restrictions in a document ([ADR 0007] §2; ISO 32000-2
//! §12.7.5.5, §12.8; research ch. 04 §8).
//!
//! * Signature fields: the `/AcroForm /Fields` tree (with `/Kids`, `/FT` inherited) for
//!   fields of type `/Sig` whose `/V` is a dictionary; their `/Lock` (`FieldMDP`).
//! * `/Perms`: `/DocMDP` (the certification signature) and `/UR3` (or the older `/UR`).
//! * Each signature dictionary's `/Type` (`/DocTimeStamp`) and `/Reference` transforms
//!   (`/DocMDP` with `/P`, `/FieldMDP`, `/UR3` with the rights it grants).
//! * A repaired base: every object is checked for `/Type /Sig` or `/DocTimeStamp`, because
//!   the catalog repair picks may not reach a signature the damaged revision added.
//!
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use std::collections::BTreeSet;

use lamina_cos::{Dict, Document, ObjRef, Object};
use lamina_model::FieldId;
use lamina_ops::ModificationClass;

use crate::constraint::{DocMdp, DocMdpPermission, Restriction, SignatureInfo, SignatureKind, Violation};
use crate::error::Result;

/// What a signature forbids.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Rule {
    DocMdp(DocMdpPermission),
    FieldMdp,
    /// A usage-rights signature, with the op classes its rights cover.
    UsageRights(BTreeSet<ModificationClass>),
}

impl Rule {
    fn allows(&self, class: ModificationClass) -> bool {
        match self {
            Self::DocMdp(permission) => permission.allows(class),
            // A lock covers named fields, and op classes don't name fields: only form changes
            // can touch them.
            Self::FieldMdp => class != ModificationClass::FormFill,
            Self::UsageRights(granted) => granted.contains(&class),
        }
    }

    const fn restriction(&self) -> Restriction {
        match self {
            Self::DocMdp(permission) => Restriction::DocMdp(*permission),
            Self::FieldMdp => Restriction::FieldMdp,
            Self::UsageRights(_) => Restriction::UsageRights,
        }
    }
}

/// The signatures of a document and the restrictions they carry.
#[derive(Debug, Clone, Default)]
pub(crate) struct Signatures {
    pub(crate) signatures: Vec<SignatureInfo>,
    rules: Vec<(Rule, SignatureInfo)>,
}

impl Signatures {
    /// The certification signature's restriction, if there is one.
    pub(crate) fn doc_mdp(&self) -> Option<DocMdp> {
        self.rules.iter().find_map(|(rule, signature)| match rule {
            Rule::DocMdp(permission) => Some(DocMdp {
                permission: *permission,
                signature: signature.clone(),
            }),
            _ => None,
        })
    }

    /// The restrictions a change of `class` breaks.
    pub(crate) fn violations(&self, class: ModificationClass) -> impl Iterator<Item = Violation> + '_ {
        self.rules
            .iter()
            .filter(move |(rule, _)| !rule.allows(class))
            .map(move |(rule, signature)| Violation {
                class,
                restriction: rule.restriction(),
                signature: signature.clone(),
            })
    }

    fn add(&mut self, signature: SignatureInfo, rules: Vec<Rule>) {
        self.rules
            .extend(rules.into_iter().map(|rule| (rule, signature.clone())));
        self.signatures.push(signature);
    }
}

/// The `/Perms` entries (§12.8.4): which signature dictionaries certify or grant rights.
struct Perms {
    doc_mdp: Option<ObjRef>,
    usage_rights: Option<ObjRef>,
    /// The values themselves, for one that is direct or found nowhere else.
    entries: Vec<(SignatureKind, Object)>,
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
    let perms = perms(doc, &catalog)?;
    let mut found = Signatures::default();
    let mut seen: BTreeSet<ObjRef> = BTreeSet::new();
    fields(doc, &catalog, &perms, &mut found, &mut seen)?;
    for (kind, value) in &perms.entries {
        let reference = value.as_ref();
        if reference.is_some_and(|r| seen.contains(&r)) {
            continue;
        }
        if let Some(dict) = doc.deref(value)?.as_dict() {
            let (mut signature, rules) = classify(doc, dict, reference, &perms)?;
            if signature.kind == SignatureKind::Approval {
                signature.kind = *kind;
            }
            found.add(signature, rules);
        }
        seen.extend(reference);
    }
    if doc.was_repaired() {
        scan(doc, &perms, &mut found, &mut seen)?;
    }
    Ok(found)
}

fn perms(doc: &Document, catalog: &Dict) -> Result<Perms> {
    let mut out = Perms {
        doc_mdp: None,
        usage_rights: None,
        entries: Vec::new(),
    };
    let Some(perms) = catalog.get("Perms") else {
        return Ok(out);
    };
    let perms = doc.deref(perms)?;
    let Some(perms) = perms.as_dict() else {
        return Ok(out);
    };
    if let Some(v) = perms.get("DocMDP") {
        out.doc_mdp = v.as_ref();
        out.entries.push((SignatureKind::Certification, v.clone()));
    }
    if let Some(v) = perms.get("UR3").or_else(|| perms.get("UR")) {
        out.usage_rights = v.as_ref();
        out.entries.push((SignatureKind::UsageRights, v.clone()));
    }
    Ok(out)
}

/// Walks the field tree for signed signature fields.
fn fields(
    doc: &Document,
    catalog: &Dict,
    perms: &Perms,
    found: &mut Signatures,
    seen: &mut BTreeSet<ObjRef>,
) -> Result<()> {
    let Some(form) = catalog.get("AcroForm") else {
        return Ok(());
    };
    let form = doc.deref(form)?;
    let Some(roots) = form.as_dict().and_then(|f| f.get("Fields")) else {
        return Ok(());
    };
    let roots = doc.deref(roots)?;
    let max_depth = doc.limits().max_tree_depth();
    // (node, inherited `/FT /Sig`, depth)
    let mut stack: Vec<(Object, bool, usize)> = roots
        .as_array()
        .unwrap_or_default()
        .iter()
        .rev()
        .map(|o| (o.clone(), false, 1))
        .collect();
    let mut visited = BTreeSet::new();
    while let Some((node, inherited_sig, depth)) = stack.pop() {
        let reference = node.as_ref();
        if let Some(r) = reference
            && !visited.insert(r.num)
        {
            continue;
        }
        let resolved = doc.deref(&node)?;
        let Some(field) = resolved.as_dict() else {
            continue;
        };
        let is_sig = field.get_name("FT").map_or(inherited_sig, |ft| ft == b"Sig");
        if is_sig && let Some(value) = field.get("V") {
            let value_ref = value.as_ref();
            let resolved_value = doc.deref(value)?;
            if let Some(dict) = resolved_value.as_dict() {
                let (mut signature, mut rules) = classify(doc, dict, value_ref, perms)?;
                signature.field = reference.map(FieldId::object);
                if let Some(lock) = field.get("Lock")
                    && doc.deref(lock)?.as_dict().is_some()
                    && !rules.contains(&Rule::FieldMdp)
                {
                    rules.push(Rule::FieldMdp);
                }
                if value_ref.is_none_or(|r| seen.insert(r)) {
                    found.add(signature, rules);
                }
            }
        }
        if depth < max_depth
            && let Some(kids) = field.get("Kids")
        {
            let kids = doc.deref(kids)?;
            stack.extend(
                kids.as_array()
                    .unwrap_or_default()
                    .iter()
                    .rev()
                    .map(|k| (k.clone(), is_sig, depth + 1)),
            );
        }
    }
    Ok(())
}

/// Checks every object of a repaired file for a signature dictionary nothing else reached.
fn scan(doc: &Document, perms: &Perms, found: &mut Signatures, seen: &mut BTreeSet<ObjRef>) -> Result<()> {
    let numbers: Vec<u32> = doc.object_numbers().collect();
    for num in numbers {
        // Objects created in this session have generation 0.
        let reference = ObjRef::new(num, doc.base_generation(num).unwrap_or(0));
        if seen.iter().any(|r| r.num == num) {
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
        if matches!(dict.get_name("Type"), Some(b"Sig" | b"DocTimeStamp")) {
            let (signature, rules) = classify(doc, dict, Some(reference), perms)?;
            found.add(signature, rules);
            seen.insert(reference);
        }
    }
    Ok(())
}

/// What the signature dictionary `dict` (the indirect object `reference`, if it is one) is,
/// and the restrictions its transforms carry.
fn classify(
    doc: &Document,
    dict: &Dict,
    reference: Option<ObjRef>,
    perms: &Perms,
) -> Result<(SignatureInfo, Vec<Rule>)> {
    let mut kind = if dict.get_name("Type") == Some(b"DocTimeStamp") {
        SignatureKind::DocTimeStamp
    } else {
        SignatureKind::Approval
    };
    let mut rules = Vec::new();
    let certifies = reference.is_some() && reference == perms.doc_mdp;
    let grants = reference.is_some() && reference == perms.usage_rights;
    if let Some(references) = dict.get("Reference") {
        let references = doc.deref(references)?;
        for item in references.as_array().unwrap_or_default() {
            let item = doc.deref(item)?;
            let Some(sig_ref) = item.as_dict() else {
                continue;
            };
            let params = match sig_ref.get("TransformParams") {
                Some(p) => doc.deref(p)?.as_dict().cloned().unwrap_or_default(),
                None => Dict::new(),
            };
            match sig_ref.get_name("TransformMethod") {
                Some(b"DocMDP") => {
                    kind = SignatureKind::Certification;
                    rules.push(Rule::DocMdp(DocMdpPermission::from_p(params.get_int("P"))));
                }
                Some(b"UR3" | b"UR") => {
                    kind = SignatureKind::UsageRights;
                    rules.push(Rule::UsageRights(granted(&params)));
                }
                Some(b"FieldMDP") => rules.push(Rule::FieldMdp),
                _ => {}
            }
        }
    }
    if certifies && !rules.iter().any(|r| matches!(r, Rule::DocMdp(_))) {
        kind = SignatureKind::Certification;
        rules.push(Rule::DocMdp(DocMdpPermission::from_p(None)));
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
            value: reference,
            revision,
            covers,
        },
        rules,
    ))
}

/// The op classes a UR3 transform's rights cover (§12.8.2.3, Table 258): form rights cover
/// form filling, annotation rights annotations, and `/Signature [/Modify]` signing.
fn granted(params: &Dict) -> BTreeSet<ModificationClass> {
    let names = |key: &str| -> Vec<Vec<u8>> {
        params
            .get(key)
            .and_then(Object::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(|o| match o {
                Object::Name(n) => Some(n.as_bytes().to_vec()),
                _ => None,
            })
            .collect()
    };
    let mut out = BTreeSet::new();
    if names("Form")
        .iter()
        .any(|n| matches!(n.as_slice(), b"FillIn" | b"Add" | b"Delete"))
    {
        out.insert(ModificationClass::FormFill);
    }
    if !names("Annots").is_empty() {
        out.insert(ModificationClass::Annotation);
    }
    if names("Signature").iter().any(|n| n.as_slice() == b"Modify") {
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

/// The number of `%%EOF` markers in the first `end` bytes of an in-memory base.
fn revision(doc: &Document, end: u64) -> Option<usize> {
    let data = doc.data()?;
    let covered = data.get(..usize::try_from(end).ok()?)?;
    Some(covered.windows(5).filter(|w| *w == b"%%EOF").count())
}
