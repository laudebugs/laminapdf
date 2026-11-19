//! The reads the Organise mode asks of a document as its edits leave it (`ToEngine::Query`): what
//! an op log would break before it applies, a page's boxes for Page boxes…, and the label ranges
//! for Page labels…. Each answer is JSON in `@laminapdf/sdk`'s shape (`Violation[]`,
//! `PageBoxesRead`, `PageLabelRange[]`), which the webview reads as it is, so the web reader's
//! dialogs run unchanged over it.

use laminapdf::{
    Document, ModificationClass, OpLog, PageGeometry, Restriction, SignatureInfo, UserSpaceRect, Violation,
};
use laminapdf_desktop_protocol::Query;
use serde::Serialize;

/// The answer to `query` on `doc`, as JSON.
///
/// # Errors
/// The read's error code: `invalid-op-log` for a log that isn't one, `page-index-out-of-range`,
/// the reading errors.
pub(crate) fn answer(doc: &Document, query: &Query) -> Result<String, String> {
    let code = |error: laminapdf::Error| error.code().to_owned();
    let json = match query {
        Query::WouldViolate { log } => {
            let ops = OpLog::from_json(log).map_err(|e| code(e.into()))?.into_ops();
            // Each op is checked against the document as it is, as the web reader's worker does.
            let mut found = Vec::new();
            for op in &ops {
                found.extend(doc.would_violate(op).map_err(code)?.iter().map(ViolationData::from));
            }
            serde_json::to_string(&found)
        }
        Query::PageBoxes { index } => serde_json::to_string(&BoxesData::from(doc.page_boxes(*index).map_err(code)?)),
        Query::LabelRanges => serde_json::to_string(&doc.label_ranges().map_err(code)?),
    };
    json.map_err(|_| "internal".to_owned())
}

/// `@laminapdf/sdk`'s `Violation`: a change a signature's restriction forbids.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ViolationData {
    class: ModificationClass,
    /// `docMdp`, `fieldMdp`, `usageRights`, `signedBytes` or `signedWidget`.
    restriction: &'static str,
    /// The certification's `/P`, for a `docMdp` restriction.
    #[serde(skip_serializing_if = "Option::is_none")]
    permission: Option<u8>,
    signature: SignatureData,
    /// The form field the change touches: whose widget a page edit deletes or reshows, or
    /// which a form or authoring edit changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
}

impl From<&Violation> for ViolationData {
    fn from(v: &Violation) -> Self {
        let permission = match v.restriction {
            Restriction::DocMdp(p) => Some(p.p()),
            _ => None,
        };
        Self {
            class: v.class,
            restriction: v.restriction.name(),
            permission,
            signature: SignatureData::from(&v.signature),
            field: v.field.as_ref().map(ToString::to_string),
        }
    }
}

/// `@laminapdf/sdk`'s `SignatureSummary`: the signature that carries a restriction.
#[derive(Debug, Serialize)]
struct SignatureData {
    /// `approval`, `certification`, `docTimeStamp` or `usageRights`.
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    /// The 1-based revision the signature closes.
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<u32>,
}

impl From<&SignatureInfo> for SignatureData {
    fn from(s: &SignatureInfo) -> Self {
        Self {
            kind: s.kind.name(),
            field: s.field.as_ref().map(ToString::to_string),
            revision: s.revision.and_then(|r| u32::try_from(r).ok()),
        }
    }
}

/// `@laminapdf/sdk`'s `PageBoxesRead`: a page's boxes in user space, `[x0, y0, x1, y1]`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BoxesData {
    media: [f64; 4],
    crop: [f64; 4],
    bleed: [f64; 4],
    trim: [f64; 4],
    art: [f64; 4],
    /// Which of the bleed, trim and art boxes the page sets itself.
    own: OwnData,
    user_unit: f64,
}

#[derive(Debug, Serialize)]
struct OwnData {
    bleed: bool,
    trim: bool,
    art: bool,
}

const fn corners(r: UserSpaceRect) -> [f64; 4] {
    [r.x0, r.y0, r.x1, r.y1]
}

impl From<PageGeometry> for BoxesData {
    fn from(g: PageGeometry) -> Self {
        Self {
            media: corners(g.media),
            crop: corners(g.crop),
            bleed: corners(g.bleed),
            trim: corners(g.trim),
            art: corners(g.art),
            own: OwnData {
                bleed: g.own.bleed,
                trim: g.own.trim,
                art: g.own.art,
            },
            user_unit: g.user_unit,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn open(name: &str) -> Document {
        let bytes = laminapdf_test_fixtures::fixture(name);
        Document::open_bytes(bytes, laminapdf::OpenOptions::default()).expect("the fixture opens")
    }

    fn json(doc: &Document, query: &Query) -> Value {
        serde_json::from_str(&answer(doc, query).expect("answers")).expect("is JSON")
    }

    /// An op log that deletes the first page of `doc`.
    fn delete_first(doc: &Document) -> Query {
        let pages = doc.page_infos().expect("reads its pages");
        let id = pages.first().map(|p| p.id.to_string()).expect("has a page");
        Query::WouldViolate {
            log: format!(
                r#"{{"v":1,"ops":[{{"op":"deletePages","args":{{"pages":["{id}"],"references":"retarget"}}}}]}}"#
            ),
        }
    }

    #[test]
    fn page_boxes_should_answer_in_the_sdk_shape() {
        let boxes = json(&open("golden/metadata.pdf"), &Query::PageBoxes { index: 0 });
        assert_eq!(
            (
                boxes["media"].as_array().map(Vec::len),
                boxes["own"]["trim"].as_bool(),
                boxes["userUnit"].as_f64()
            ),
            (Some(4), Some(false), Some(1.0))
        );
    }

    #[test]
    fn a_page_past_the_end_should_answer_its_code() {
        assert_eq!(
            answer(&open("golden/metadata.pdf"), &Query::PageBoxes { index: 99 }),
            Err("page-index-out-of-range".to_owned())
        );
    }

    #[test]
    fn an_unlabelled_document_should_have_no_label_ranges() {
        assert_eq!(
            answer(&open("golden/metadata.pdf"), &Query::LabelRanges),
            Ok("[]".to_owned())
        );
    }

    #[test]
    fn deleting_a_page_of_a_certified_file_should_break_its_certification() {
        let doc = open("signed/certified-p1.pdf");
        let found = json(&doc, &delete_first(&doc));
        assert_eq!(
            (
                found[0]["restriction"].as_str(),
                found[0]["permission"].as_u64(),
                found[0]["signature"]["kind"].as_str()
            ),
            (Some("docMdp"), Some(1), Some("certification"))
        );
    }

    #[test]
    fn an_unsigned_document_should_break_nothing() {
        let doc = open("golden/metadata.pdf");
        assert_eq!(answer(&doc, &delete_first(&doc)), Ok("[]".to_owned()));
    }

    #[test]
    fn a_log_that_is_not_one_should_be_an_invalid_op_log() {
        let query = Query::WouldViolate { log: "{}".into() };
        assert_eq!(
            answer(&open("golden/metadata.pdf"), &query),
            Err("invalid-op-log".to_owned())
        );
    }
}
