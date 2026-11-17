//! A copied page's content without its marked content (ISO 32000-2 §14.6): every `BMC`, `BDC`,
//! `EMC`, `MP` and `DP` goes with its operands, and so does everything inside an optional
//! content sequence (`/OC … BDC`, §8.11.3.2) whose group or membership is off as the document
//! opens, or inside one that is. The content of other sequences stays, `/Span` with
//! `/ActualText` included, which loses the replacement text: the copy is drawn as a figure,
//! whose alternate text is what a reader gets.

use laminapdf_content::{ContentIter, Lexed, Operator};
use laminapdf_cos::{Dict, Object};

use crate::error::Result;

/// `content` of a page whose resources are `resources`, without its marked content and what
/// hidden optional content holds; `None` when the tokenizer stops at a hard limit before the
/// end, since what follows could still hold marked content.
pub(super) fn unmarked(
    doc: &laminapdf_model::Document,
    resources: Option<&Dict>,
    content: &[u8],
) -> Result<Option<Vec<u8>>> {
    let mut out = Vec::with_capacity(content.len());
    // Where the operator before the current one ended: an operator's span starts there, so its
    // operands go with it.
    let mut previous = 0;
    // Each open sequence: whether it, or one around it, hides its content.
    let mut hidden: Vec<bool> = Vec::new();
    // A span was left out since the last one kept: the next starts on a new line, so two
    // tokens never run together.
    let mut gap = false;
    for item in ContentIter::new(content) {
        let op = match item {
            Ok(Lexed::Op(op)) => op,
            Ok(Lexed::Issue(_)) => continue,
            Err(_) => return Ok(None),
        };
        let end = op
            .inline_image
            .as_ref()
            .map_or(op.offset + op.operator.keyword().len(), |image| image.end);
        let span = content.get(previous..end).unwrap_or_default();
        previous = end;
        let off = hidden.last().copied().unwrap_or(false);
        match op.operator {
            Operator::BeginMarked => hidden.push(off),
            Operator::BeginMarkedProperties => {
                let layer_off = layer_off(doc, resources, &op.operands)?;
                hidden.push(off || layer_off);
            }
            Operator::EndMarked => {
                hidden.pop();
            }
            Operator::MarkPoint | Operator::MarkPointProperties => {}
            _ if off => {}
            _ => {
                if gap {
                    out.push(b'\n');
                    gap = false;
                }
                out.extend_from_slice(span);
                continue;
            }
        }
        gap = true;
    }
    if !hidden.last().copied().unwrap_or(false) {
        out.extend_from_slice(content.get(previous..).unwrap_or_default());
    }
    Ok(Some(out))
}

/// Whether `BDC`'s operands open an optional content sequence (`/OC /Name BDC`) whose group or
/// membership dictionary, named in the page's `/Properties`, is off.
fn layer_off(doc: &laminapdf_model::Document, resources: Option<&Dict>, operands: &[Object]) -> Result<bool> {
    let (Some(Object::Name(tag)), Some(Object::Name(name))) = (operands.first(), operands.get(1)) else {
        return Ok(false);
    };
    if tag.as_bytes() != b"OC" {
        return Ok(false);
    }
    let Some(properties) = resources.and_then(|r| r.get("Properties")) else {
        return Ok(false);
    };
    let properties = doc.cos().deref(properties)?;
    let entry = properties
        .as_dict()
        .and_then(|p| p.iter().find(|(key, _)| key.as_bytes() == name.as_bytes()));
    let Some((_, value)) = entry else {
        return Ok(false);
    };
    Ok(doc.optional_content_off(value)?)
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::{Dict, Object, OpenOptions};
    use laminapdf_test_fixtures::classic_with_root;

    use super::unmarked;

    /// A document with two layers, `On` (object 4) and `Off` (object 5, off by default), named
    /// in the returned resources.
    fn layered() -> (laminapdf_model::Document, Dict) {
        let bytes = classic_with_root(
            &[
                "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R 5 0 R] /D << /OFF [5 0 R] >> >> >>",
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
                "<< /Type /OCG /Name (On) >>",
                "<< /Type /OCG /Name (Off) >>",
                "<< /Type /OCMD /OCGs [4 0 R 5 0 R] /P /AllOn >>",
            ],
            "",
        );
        let doc = laminapdf_model::Document::open(bytes, OpenOptions::default()).unwrap();
        let mut properties = Dict::new();
        properties.insert("On", Object::Ref(laminapdf_cos::ObjRef::new(4, 0)));
        properties.insert("Off", Object::Ref(laminapdf_cos::ObjRef::new(5, 0)));
        properties.insert("Both", Object::Ref(laminapdf_cos::ObjRef::new(6, 0)));
        let mut resources = Dict::new();
        resources.insert("Properties", Object::Dict(properties));
        (doc, resources)
    }

    fn strip(content: &[u8]) -> Vec<u8> {
        let (doc, resources) = layered();
        unmarked(&doc, Some(&resources), content).unwrap().unwrap()
    }

    #[test]
    fn marked_content_operators_go_with_their_operands() {
        let content = b"/P <</MCID 0>> BDC BT (Hi) Tj ET EMC /Artifact BMC 0 0 m 1 1 l S EMC /Tag MP";
        assert_eq!(strip(content), b"\n BT (Hi) Tj ET\n 0 0 m 1 1 l S");
    }

    #[test]
    fn content_without_marked_content_is_kept_byte_for_byte() {
        let content = b"q 1 0 0 1 5 5 cm BI /W 1 /H 1 /BPC 8 /CS /G ID \x00 EI Q";
        assert_eq!(strip(content), content);
    }

    #[test]
    fn content_on_a_layer_that_is_off_is_left_out() {
        let content = b"/OC /On BDC (shown) Tj EMC /OC /Off BDC (hidden) Tj EMC (after) Tj";
        assert_eq!(strip(content), b"\n (shown) Tj\n (after) Tj");
    }

    #[test]
    fn content_nested_in_a_layer_that_is_off_is_left_out() {
        let content = b"/OC /Off BDC /P <</MCID 1>> BDC (deep) Tj EMC /OC /On BDC (on) Tj EMC EMC (out) Tj";
        assert_eq!(strip(content), b"\n (out) Tj");
    }

    #[test]
    fn a_membership_dictionary_that_needs_every_group_on_hides_its_content() {
        assert_eq!(strip(b"/OC /Both BDC (both) Tj EMC (rest) Tj"), b"\n (rest) Tj");
    }

    #[test]
    fn a_span_with_replacement_text_keeps_its_content() {
        let content = b"/Span <</ActualText (fi)>> BDC (\xAE) Tj EMC";
        assert_eq!(strip(content), b"\n (\xAE) Tj");
    }

    #[test]
    fn content_the_tokenizer_cannot_finish_is_refused() {
        let (doc, resources) = layered();
        // Deeper than the syntax layer's nesting limit.
        let content = [b"[".repeat(1000), b"] Tj".to_vec()].concat();
        assert_eq!(unmarked(&doc, Some(&resources), &content).unwrap(), None);
    }
}
