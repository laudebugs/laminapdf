//! `Document::disclosure`: what a file holds beyond its visible pages, on a file built to
//! hold something of every kind, on plain fixtures and on edge cases.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use laminapdf_model::{ActionKind, AttachmentHolder, Document, Limits, OpenOptions, Owner, Trigger};
use laminapdf_test_fixtures::{classic_with_root, disclosing, fixture, stream_body};

fn open(bytes: Vec<u8>) -> Document {
    Document::open(bytes, OpenOptions::default()).unwrap()
}

#[test]
fn a_file_built_to_disclose_lists_each_kind() {
    let doc = open(disclosing());
    let d = doc.disclosure().unwrap();

    let xmp = d.xmp.as_ref().unwrap();
    assert_eq!(xmp.object, Some(7));
    assert_eq!(xmp.namespaces.len(), 3);

    assert_eq!(d.attachments.len(), 1);
    let a = &d.attachments[0];
    assert_eq!(a.name.as_deref(), Some("notes.txt"));
    assert_eq!(
        (a.size, a.mime.as_deref(), a.object),
        (Some(12), Some("text/plain"), Some(14))
    );
    assert_eq!(a.holders, [AttachmentHolder::EmbeddedFiles]);

    let open_action = d.actions.iter().find(|x| x.trigger == Trigger::OpenAction).unwrap();
    assert_eq!(open_action.kind, ActionKind::JavaScript);
    assert_eq!(open_action.script_bytes, Some(20));
    let launch = d.actions.iter().find(|x| x.kind == ActionKind::Launch).unwrap();
    assert_eq!(
        (&launch.trigger, &launch.owner),
        (
            &Trigger::Activate,
            &Owner::Annotation {
                page: 0,
                object: Some(9)
            }
        )
    );

    let subtypes: Vec<&str> = d.annotations.iter().map(|a| a.subtype.as_str()).collect();
    assert_eq!(subtypes, ["Link", "Text", "Widget"]);
    let text = &d.annotations[1];
    assert_eq!(
        (text.author.as_deref(), text.contents.as_deref()),
        (Some("Reviewer"), Some("Fix the totals"))
    );
    assert!(!text.is_plain() && d.annotations[0].is_plain());

    assert_eq!(d.fields.len(), 1);
    let field = &d.fields[0];
    assert_eq!(
        (field.name.as_str(), field.kind.as_deref(), field.value.as_deref()),
        ("name", Some("Tx"), Some("Alice"))
    );
    assert!(!field.hidden);

    assert_eq!(d.layers_off, ["Draft notes"]);
    assert!(!d.xfa);
    assert_eq!(d.redactions(), [] as [(usize, u32); 0]);
    // The update replaced object 4 in place: no copy is left behind.
    assert!(d.unreferenced.is_empty(), "{:?}", d.unreferenced);
    assert_eq!(
        doc.info().unwrap().unwrap().keys(),
        ["Title", "Producer", "SourceModified"]
    );
}

#[test]
fn a_plain_file_discloses_nothing() {
    let d = open(fixture("corpus/hello.pdf")).disclosure().unwrap();
    assert!(d.xmp.is_none() && d.attachments.is_empty() && d.actions.is_empty());
    assert!(d.annotations.is_empty() && d.fields.is_empty() && d.layers_off.is_empty());
    assert!(d.unreferenced.is_empty() && d.metadata_streams.is_empty() && d.piece_info.is_empty());
}

#[test]
fn an_object_nothing_references_is_listed() {
    let bytes = classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
            "(an old copy nobody points at)",
            "<< /Self 4 0 R >>",
        ],
        "",
    );
    assert_eq!(open(bytes).disclosure().unwrap().unreferenced, [3, 4]);
}

#[test]
fn actions_are_followed_through_next_once_each() {
    // A loop: 3 → 4 → 3. Each is listed once, and /Next of a named script counts as fired by it.
    let bytes = classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R /OpenAction 3 0 R >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
            "<< /S /URI /URI (https://example.org) /Next 4 0 R >>",
            "<< /S /SubmitForm /F (https://example.org/collect) /Next [3 0 R] >>",
        ],
        "",
    );
    let actions = open(bytes).disclosure().unwrap().actions;
    let kinds: Vec<&ActionKind> = actions.iter().map(|a| &a.kind).collect();
    assert_eq!(kinds, [&ActionKind::Uri, &ActionKind::SubmitForm]);
    assert!(
        actions
            .iter()
            .all(|a| a.trigger == Trigger::OpenAction && a.owner == Owner::Document)
    );
}

#[test]
fn a_script_past_the_decode_limit_counts_its_stored_length() {
    let hex = "61".repeat(100) + ">";
    let script = stream_body("/Filter /ASCIIHexDecode", hex.as_bytes());
    let catalog = "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS 3 0 R >> >>";
    let pages = "<< /Type /Pages /Kids [] /Count 0 >>";
    let bodies: [&[u8]; 3] = [catalog.as_bytes(), pages.as_bytes(), &script];
    let bytes = laminapdf_test_fixtures::classic(&bodies, "<< /Size 4 /Root 1 0 R >>");
    let options = OpenOptions::default().with_limits(Limits::default().with_max_stream_decoded(16));
    let doc = Document::open(bytes, options).unwrap();
    let actions = doc.disclosure().unwrap().actions;
    assert_eq!(actions[0].script_bytes, Some(hex.len() as u64));
}

#[test]
fn field_names_join_their_ancestors_and_hidden_widgets_count() {
    let bytes = classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [3 0 R] /XFA 6 0 R >> >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
            "<< /T (person) /FT /Tx /Kids [4 0 R] >>",
            "<< /T (email) /Parent 3 0 R /Kids [5 0 R] /V <FEFF0061> >>",
            "<< /Type /Annot /Subtype /Widget /Parent 4 0 R /F 2 >>",
            "<< >>",
        ],
        "",
    );
    let d = open(bytes).disclosure().unwrap();
    assert!(d.xfa);
    assert_eq!(d.fields.len(), 1);
    let f = &d.fields[0];
    assert_eq!(
        (f.name.as_str(), f.kind.as_deref(), f.value.as_deref()),
        ("person.email", Some("Tx"), Some("a"))
    );
    assert!(f.hidden);
}

#[test]
fn every_fixture_scans_without_error() {
    for dir in ["corpus", "golden", "signed"] {
        for entry in std::fs::read_dir(laminapdf_test_fixtures::fixtures_dir().join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "pdf") {
                let Ok(doc) = Document::open(std::fs::read(&path).unwrap(), OpenOptions::default()) else {
                    continue;
                };
                if let Err(e) = doc.disclosure() {
                    panic!("{}: {e}", path.display());
                }
            }
        }
    }
}
