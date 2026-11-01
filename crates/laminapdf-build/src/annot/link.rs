//! Link annotations (§12.5.6.5): `/Subtype /Link` with `/A /URI` or `/Dest`, written when
//! the build finishes so a link can name a page added after its own.

use laminapdf_cos::{Dict, Name, ObjRef, PdfString};
use laminapdf_model::UserSpaceRect;
use laminapdf_syntax::text::encode_text_string;

use crate::nav::LinkTarget;
use crate::page::rect_object;

/// A link annotation as drawn: where it goes and its `/Contents`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LinkSpec {
    pub(crate) target: LinkTarget,
    pub(crate) contents: Option<String>,
}

/// A link annotation whose object number is reserved and listed in its page's `/Annots`,
/// waiting for its target to be resolved.
#[derive(Debug, Clone)]
pub(crate) struct PendingLink {
    pub(crate) object: ObjRef,
    pub(crate) dict: Dict,
    pub(crate) target: LinkTarget,
    /// The index of the page it is on.
    pub(crate) page: u32,
}

/// The annotation dictionary without its target: `/Rect` in user space, no border, printed
/// (`/F 4`), `/Contents` when given.
pub(crate) fn link_dict(rect: &UserSpaceRect, contents: Option<&str>) -> Dict {
    let mut d = Dict::new();
    d.insert("Type", Name::from("Annot"));
    d.insert("Subtype", Name::from("Link"));
    d.insert("Rect", rect_object(rect));
    d.insert("Border", vec![0.into(), 0.into(), 0.into()]);
    d.insert("F", 4);
    if let Some(contents) = contents {
        d.insert("Contents", PdfString::literal(encode_text_string(contents)));
    }
    d
}
