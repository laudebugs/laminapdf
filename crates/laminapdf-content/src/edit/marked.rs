//! What tagged content an IR still draws ([ADR 0022] §4): the MCIDs of the marked-content
//! sequences that hold something painted, so an edit can tell which ones it emptied and the
//! structure tree can let them go.
//!
//! [ADR 0022]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0022-content-stream-editing.md

use std::collections::BTreeSet;

use laminapdf_syntax::Object;

use super::{ContentObjectId, ContentOp, ContentOps};

impl ContentOps {
    /// The MCIDs of the marked-content sequences that still draw something: a path painted, an
    /// image, a shading, a form or a glyph shown. A sequence an edit emptied isn't listed, so the
    /// MCIDs listed before an edit and not after it are the ones the edit took away.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn drawn_mcids(&self) -> BTreeSet<i64> {
        // Inserted one by one: collecting would sort, and bring a sort's code into the module.
        let mut out = BTreeSet::new();
        for seq in &self.marked {
            let Some(mcid) = seq.mcid else { continue };
            let end = seq.end.unwrap_or(self.ops.len());
            let inside = self.ops.get(seq.begin.saturating_add(1)..end).unwrap_or_default();
            if inside.iter().any(draws) {
                out.insert(mcid);
            }
        }
        out
    }

    /// The MCIDs of the marked-content sequences around the object or inside it: the structure
    /// elements it belongs to.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn object_mcids(&self, id: ContentObjectId) -> BTreeSet<i64> {
        let Some(object) = self.object(id) else {
            return BTreeSet::new();
        };
        let range = object.operators();
        let around = object.marked().iter().filter_map(|&m| self.marked.get(m));
        let inside = self.marked.iter().filter(|seq| range.contains(&seq.begin));
        let mut out = BTreeSet::new();
        for mcid in around.chain(inside).filter_map(|seq| seq.mcid) {
            out.insert(mcid);
        }
        out
    }

    /// Whether the content draws something outside every marked-content sequence: real content
    /// that isn't tagged (ISO 14289-1 §7.1), once a page is tagged. A form or an image drawn by
    /// `Do` isn't counted, since a form may be tagged inside.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn draws_untagged(&self) -> bool {
        let mut depth = 0_usize;
        for op in &self.ops {
            match &op.keyword[..] {
                b"BMC" | b"BDC" => depth += 1,
                b"EMC" => depth = depth.saturating_sub(1),
                b"Do" => {}
                _ if depth == 0 && draws(op) => return true,
                _ => {}
            }
        }
        false
    }

    /// Whether the object still draws anything: text an edit deleted can leave a text object
    /// that shows no glyph.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn object_draws(&self, id: ContentObjectId) -> bool {
        let range = self.object(id).map(super::ContentObject::operators).unwrap_or_default();
        self.ops.get(range).unwrap_or_default().iter().any(draws)
    }
}

/// Whether the operator puts marks on the page: paints a path, draws an image, a shading or a
/// form, or shows at least one glyph.
fn draws(op: &ContentOp) -> bool {
    if op.inline_image.is_some() {
        return true;
    }
    let shown = |o: &Object| match o {
        Object::String(s) => !s.bytes.is_empty(),
        Object::Array(items) => items
            .iter()
            .any(|i| matches!(i, Object::String(s) if !s.bytes.is_empty())),
        _ => false,
    };
    match &op.keyword[..] {
        b"S" | b"s" | b"f" | b"F" | b"f*" | b"B" | b"B*" | b"b" | b"b*" | b"sh" | b"Do" => true,
        b"Tj" | b"TJ" | b"'" | b"\"" => op.operands.last().is_some_and(shown),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::Limits;

    use super::super::{ContentOps, ContentOwner};

    fn ir(content: &[u8]) -> ContentOps {
        let owner = ContentOwner::Page("p3.0".parse().unwrap());
        ContentOps::from_parts(owner, vec![(None, content.to_vec())], &Limits::default()).unwrap()
    }

    #[test]
    fn drawn_mcids_skip_a_sequence_that_shows_nothing() {
        let ir = ir(b"/P <</MCID 0>> BDC BT (a) Tj ET EMC /P <</MCID 1>> BDC BT () Tj ET EMC");
        assert_eq!(ir.drawn_mcids().into_iter().collect::<Vec<_>>(), vec![0]);
    }

    #[test]
    fn content_outside_marked_content_is_untagged() {
        assert!(!ir(b"/P <</MCID 0>> BDC 0 0 1 1 re f EMC /Artifact BMC 0 0 1 1 re f EMC").draws_untagged());
        assert!(ir(b"/P <</MCID 0>> BDC EMC 0 0 1 1 re f").draws_untagged());
    }

    #[test]
    fn deleting_an_object_takes_the_marked_content_that_holds_only_it() {
        let mut ir = ir(b"/Figure <</MCID 0>> BDC 0 0 10 10 re f EMC /P <</MCID 1>> BDC BT (a) Tj ET EMC");
        let first = ir.objects().first().unwrap().id();
        ir.delete(first).unwrap();
        let encoded = String::from_utf8(ir.encode_every_part().parts[0].bytes().unwrap().to_vec()).unwrap();
        assert!(!encoded.contains("MCID 0"), "{encoded}");
        assert!(encoded.contains("MCID 1"), "{encoded}");
    }
}
