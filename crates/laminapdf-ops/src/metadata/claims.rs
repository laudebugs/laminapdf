//! [`RemoveConformanceClaims`]: the one op that may take the PDF/A or PDF/UA identification out
//! of the XMP packet (E7 plan §2.1, §10 question 2), for a file whose edits broke a claim it
//! should no longer make. Every other op refuses to touch it (`xmp-claim-read-only`).

use laminapdf_model::xmp::{PDFA_ID, PDFUA_ID};
use serde::{Deserialize, Serialize};

use super::write::{Policy, edit_packet};
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;

/// Removes the file's PDF/A claim (every `pdfaid:` property: part, conformance, amendment,
/// revision), its PDF/UA claim (every `pdfuaid:` property), or both, from the XMP packet, so the
/// file no longer promises what its edits broke. A claim the packet doesn't make is left alone;
/// a file without a packet is unchanged. The removed claim stays in the file's earlier revisions
/// until a full rewrite
/// ([`Applied::still_in_earlier_revisions`](crate::Applied::still_in_earlier_revisions)). It
/// never adds a claim back: a file that passes its checks again is claimed by conversion.
///
/// ```
/// use laminapdf_ops::{Op, RemoveConformanceClaims};
///
/// let op = Op::from(RemoveConformanceClaims::new(false, true));
/// assert_eq!(
///     serde_json::to_string(&op)?,
///     r#"{"op":"removeConformanceClaims","args":{"pdfua":true}}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct RemoveConformanceClaims {
    /// Remove the PDF/A claim (`pdfaid:`).
    #[serde(default, skip_serializing_if = "wire::is_false")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub pdfa: bool,
    /// Remove the PDF/UA claim (`pdfuaid:`).
    #[serde(default, skip_serializing_if = "wire::is_false")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub pdfua: bool,
}

impl RemoveConformanceClaims {
    /// The op that removes the PDF/A claim when `archival` is set, and the PDF/UA claim when
    /// `accessible` is.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(archival: bool, accessible: bool) -> Self {
        Self {
            pdfa: archival,
            pdfua: accessible,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        if !self.pdfa && !self.pdfua {
            return Err(Error::InvalidOp("removeConformanceClaims needs pdfa or pdfua"));
        }
        let namespaces: Vec<&str> = [(self.pdfa, PDFA_ID), (self.pdfua, PDFUA_ID)]
            .into_iter()
            .filter_map(|(on, ns)| on.then_some(ns))
            .collect();
        edit_packet(doc, false, Policy::remove(false), &mut |packet| {
            let held: Vec<(String, String)> = packet
                .properties()
                .filter(|p| namespaces.contains(&p.namespace.as_str()))
                .map(|p| (p.namespace.clone(), p.name.clone()))
                .collect();
            for (namespace, name) in &held {
                packet.remove_property(namespace, name)?;
            }
            Ok(!held.is_empty())
        })
    }
}
