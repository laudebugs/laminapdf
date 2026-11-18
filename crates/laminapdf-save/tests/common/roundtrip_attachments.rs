//! The attachment cases of the round-trip suite (todo E3f, J.05, M.09, H.09): the files the
//! `attachFile` cases attach to every fixture, and the edits of the attachment fixtures (a file
//! attached to a page, one renamed, one deleted that an annotation still holds), saved
//! incrementally and as a full rewrite. Lamina reopens each with the same files, byte for byte;
//! the tools list the `/EmbeddedFiles` tree and compare each file's SHA-256 (`attachments`).
#![allow(
    unreachable_pub,
    reason = "shared by the tests and the `roundtrip` example, as `tests/common` helpers are"
)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test support: a panic is a test failure"
)]

use std::fmt::Write as _;
use std::path::Path;

use laminapdf_cos::OpenOptions;
use laminapdf_model::{AfRelationship, AttachmentHolder};
use laminapdf_ops::{AttachFile, AttachTarget, BlobId, DeleteAttachment, Document, Op, Origin, UpdateAttachment};
use laminapdf_save::{Save as _, SaveMode, SaveOptions};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::roundtrip::{Suite, facts};

/// What the `attachFile` cases attach: an invoice as XML, associated with the document.
pub const ATTACHED: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<invoice number=\"42\" total=\"98.40\"/>\n";

/// The `attachFile` cases' op: [`ATTACHED`] as `invoice.xml`, the document's XML alternative,
/// named by its blob ID (the case adds the bytes to the session first).
pub fn attach_op() -> Op {
    let mut op = AttachFile::new("invoice.xml", BlobId::of(ATTACHED), "text/xml")
        .with_target(AttachTarget::Document, AfRelationship::Alternative);
    op.description = Some("The invoice as XML".into());
    op.modified = Some("D:20261117090000Z".parse().unwrap());
    op.into()
}

/// The files of the `/EmbeddedFiles` tree, sorted by key: `{ key, name, sha256 }`, the key as
/// pdf.js lists it, the name (`/UF`, else `/F`) as PDFium does.
pub fn attachment_facts(doc: &laminapdf_model::Document) -> Vec<Value> {
    let mut out: Vec<Value> = doc
        .attachments()
        .unwrap()
        .into_iter()
        .filter(|a| a.holders.contains(&AttachmentHolder::EmbeddedFiles))
        .map(|a| {
            let bytes = doc
                .attachment_bytes(&a.id)
                .unwrap()
                .map(|b| b.bytes)
                .unwrap_or_default();
            json!({ "key": a.key, "name": a.name, "sha256": sha256(&bytes) })
        })
        .collect();
    out.sort_by_key(|v| v["key"].to_string());
    out
}

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// What a reopened `attachFile` case got wrong: the file missing from the tree, its bytes
/// changed, or the document's association lost.
pub fn attached_problem(id: &str, reopened: &laminapdf_model::Document) -> Option<String> {
    let Some(found) = reopened
        .attachments()
        .unwrap()
        .into_iter()
        .find(|a| a.name.as_deref() == Some("invoice.xml"))
    else {
        return Some(format!("{id}: lamina reopened no invoice.xml"));
    };
    let bytes = reopened.attachment_bytes(&found.id).unwrap().map(|b| b.bytes);
    let associated = found.holders.contains(&AttachmentHolder::Document);
    (bytes.as_deref() != Some(ATTACHED) || !associated).then(|| {
        format!(
            "{id}: lamina reopened invoice.xml with {} bytes, associated {associated}",
            bytes.map_or(0, |b| b.len())
        )
    })
}

/// The attachment edits on the attachment fixtures: a file attached to the first page, the
/// fixture's last file renamed and described, and its first deleted (in `associated.pdf` an
/// annotation still holds it, so it leaves the tree but stays in the file).
pub fn attachment_edits(root: &Path, suite: &mut Suite) {
    for name in ["attachments/associated.pdf", "attachments/portfolio.pdf"] {
        let bytes = std::fs::read(root.join("fixtures").join(name)).unwrap();
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let id = format!(
                "attachments-{}-{mode:?}",
                name.replace('/', "_").trim_end_matches(".pdf")
            )
            .to_lowercase();
            let mut doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            doc.add_blob(ATTACHED.to_vec());
            let (pages, rotations, info, labels) = facts(&doc);
            let first_page = doc.pages().unwrap().get(0).unwrap().id().clone();
            let list = doc.attachments().unwrap();
            let mut ops = vec![{
                let mut op = AttachFile::new("invoice.xml", BlobId::of(ATTACHED), "text/xml")
                    .with_target(AttachTarget::Page(first_page), AfRelationship::Source);
                op.modified = Some("D:20261117090000Z".parse().unwrap());
                Op::from(op)
            }];
            if let Some(last) = list.last() {
                let mut rename = UpdateAttachment::new(last.id.clone());
                rename.name = Some(format!("renamed-{}", last.name.clone().unwrap_or_default()));
                rename.description = Some(Some("Renamed by the round trip".into()));
                ops.push(rename.into());
            }
            if let Some(first) = list.first() {
                ops.push(DeleteAttachment::new(first.id.clone()).into());
            }
            doc.apply_all(Origin::Local, ops).unwrap();
            let attachments = attachment_facts(&doc);
            let mut output = Vec::new();
            let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
            suite.saved += 1;
            let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
            if attachment_facts(&reopened) != attachments
                || facts(&reopened) != (pages, rotations.clone(), info.clone(), labels.clone())
                || reopened.was_repaired()
            {
                suite.failures.push(format!(
                    "{id}: lamina reopened attachments {:?} (repaired {}), expected {attachments:?}",
                    attachment_facts(&reopened),
                    reopened.was_repaired()
                ));
            }
            let file = format!("{id}.pdf");
            suite.manifest.push(json!({
                "file": &file,
                "source": name,
                "mode": format!("{:?}", report.mode()),
                "pages": pages,
                "rotations": rotations,
                "title": info[0],
                "info": null,
                "encrypted": false,
                "direct_info": false,
                "invalidated": 0,
                "attachments": attachments,
            }));
            suite.outputs.push((file, output));
        }
    }
}
