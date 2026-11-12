//! The canonical rewrite: a packet that can't be edited in place (UTF-16, or read-only by its
//! `end="r"` trailer) written whole, every property kept.

use std::fmt::Write as _;
use std::ops::Range;

use super::extension::{self, ExtensionProperty, Insert};
use super::mapping::{ADOBE_META, RDF};
use super::rdf::{Description, Place, Rdf};
use super::tree::{Binding, Scope};
use super::write::{
    Change, Declare, HEADER, PADDING, TRAILER, element, free_prefix, is, others, padding, prefix_in, qname,
    rdf_attribute_prefix, to_declare, write_declarations,
};

/// The packet rewritten whole: [`HEADER`], `x:xmpmeta`, `rdf:RDF` with each description in
/// turn (its declarations and the properties `changes` leaves alone, as written), the changed
/// properties in the first description, then [`PADDING`] and [`TRAILER`]; and the extension
/// schema entries it declared. Comments and white space between properties are not kept.
pub(super) fn canonical(
    text: &str,
    rdf: &Rdf,
    changes: &[Change<'_>],
    declare: Declare,
) -> (String, Vec<ExtensionProperty>) {
    let dropped: Vec<&Range<usize>> = rdf
        .properties
        .iter()
        .filter(|p| changes.iter().any(|c| is(p, c)))
        .map(|p| match &p.place {
            Place::Attribute(span) | Place::Element(span) => span,
        })
        .collect();
    // Every binding in scope at `rdf:RDF`, so the descriptions copied as written keep their
    // meaning, and a real prefix for the RDF namespace, which `about` needs.
    let mut root: Vec<Binding> = rdf
        .scope
        .effective()
        .into_iter()
        .filter(|b| b.prefix != "xml")
        .collect();
    let rdf_name = if let Some(p) = rdf.scope.prefix_for(RDF) {
        p.to_owned()
    } else {
        let p = free_prefix(&rdf.scope, &root, "rdf");
        root.push(Binding::new(&p, RDF));
        p
    };
    let scope = Scope::default().with(&root);
    let mut out = format!("{HEADER}\n<x:xmpmeta xmlns:x=\"{ADOBE_META}\">\n <{rdf_name}:RDF");
    write_declarations(&mut out, root.iter());
    out.push_str(">\n");
    let fresh = Description {
        qualified: format!("{rdf_name}:Description"),
        start_tag: 0..0,
        empty: false,
        insert_at: 0,
        indent: String::new(),
        own_indent: String::new(),
        scope: scope.clone(),
        declarations: Vec::new(),
        attributes: Vec::new(),
        has_about: false,
        children: Vec::new(),
    };
    let descriptions: Vec<&Description> = if rdf.descriptions.is_empty() {
        vec![&fresh]
    } else {
        rdf.descriptions.iter().collect()
    };
    let first_scope = scope.with(descriptions.first().map_or(&[][..], |d| &d.declarations));
    let mut first_declarations = Vec::new();
    let mut first_properties = String::new();
    let mut written = Vec::new();
    for change in changes {
        let Some(body) = change.value else { continue };
        let prefix = prefix_in(&first_scope, &mut first_declarations, change.namespace, change.prefix);
        let mut own = Vec::new();
        let rdf_here = body.rdf_name(&first_scope.with(&first_declarations), &mut own);
        let _ = writeln!(
            first_properties,
            "   {}",
            element(change.name, body, &prefix, &rdf_here, &others(rdf, body), &own)
        );
        written.push((change, prefix));
    }
    let declared = to_declare(rdf, changes, &written, declare);
    let (inserts, leftover) = extension::plan(rdf.extensions.as_ref(), &declared);
    if !leftover.is_empty() {
        let schemas = extension::schemas_element(&leftover, &first_scope.with(&first_declarations));
        let _ = writeln!(first_properties, "   {schemas}");
    }
    for (i, d) in descriptions.into_iter().enumerate() {
        let inside = scope.with(&d.declarations);
        let mut declarations = if i == 0 {
            std::mem::take(&mut first_declarations)
        } else {
            Vec::new()
        };
        // The kept attributes never include `rdf:about` when the description had none: only
        // changed properties are dropped.
        let about = (!d.has_about).then(|| {
            let name = rdf_attribute_prefix(&inside.with(&declarations), &mut declarations);
            qname(&name, "about")
        });
        let _ = write!(out, "  <{}", d.qualified);
        write_declarations(&mut out, d.declarations.iter().chain(&declarations));
        if let Some(about) = about {
            let _ = write!(out, " {about}=\"\"");
        }
        for span in d.attributes.iter().filter(|s| !dropped.contains(s)) {
            out.push(' ');
            out.push_str(text.get(span.clone()).unwrap_or_default());
        }
        out.push_str(">\n");
        for span in d.children.iter().filter(|s| !dropped.contains(s)) {
            let _ = writeln!(out, "   {}", copy(text, span, &inserts));
        }
        if i == 0 {
            out.push_str(&first_properties);
        }
        let _ = writeln!(out, "  </{}>", d.qualified);
    }
    for span in &rdf.others {
        let _ = writeln!(out, "  {}", text.get(span.clone()).unwrap_or_default());
    }
    let _ = write!(out, " </{rdf_name}:RDF>\n</x:xmpmeta>\n{}{TRAILER}", padding(PADDING));
    (out, declared)
}

/// The bytes of `span` with the `inserts` that fall inside it made.
fn copy(text: &str, span: &Range<usize>, inserts: &[Insert]) -> String {
    let mut inside: Vec<&Insert> = inserts
        .iter()
        .filter(|i| span.start < i.at && i.at < span.end)
        .collect();
    inside.sort_by_key(|i| i.at);
    let mut out = String::new();
    let mut at = span.start;
    for i in inside {
        out.push_str(text.get(at..i.at).unwrap_or_default());
        out.push_str(&i.text);
        at = i.at;
    }
    out.push_str(text.get(at..span.end).unwrap_or_default());
    out
}
