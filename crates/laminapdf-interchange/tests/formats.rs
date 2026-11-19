//! The exchange formats on their own: XFDF and FDF read, written and read back, a DTD refused,
//! a malformed file's line named, and what an import leaves out listed.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test code: a panic is a test failure"
)]

use laminapdf_cos::Object;
use laminapdf_interchange::{Error, Exchange, SkipReason, read, write_fdf, write_xfdf};
use laminapdf_syntax::Limits;

const REVIEW: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<xfdf xmlns="http://ns.adobe.com/xfdf/" xml:space="preserve">
<annots>
<highlight page="0" rect="72,700,180,714" name="h1" title="Ada" color="#FFFF00" opacity="0.5" flags="print" date="D:20261101100000Z" coords="72,714,180,714,72,700,180,700">
<contents>Revenue is up</contents>
<popup open="no" page="0" rect="200,600,380,700"/>
</highlight>
<text page="0" rect="72,700,92,720" name="r1" title="Ben" inreplyto="h1" replyType="reply" icon="Comment"><contents>Agreed</contents></text>
<text page="0" rect="72,700,92,720" name="s1" title="Ben" inreplyto="h1" state="Accepted" statemodel="Review"/>
<ink page="1" rect="10,10,60,60" name="i1" width="2" style="dash" dashes="3,2"><inklist><gesture>10,10;20,30;60,60</gesture><gesture>15,50;55,15</gesture></inklist></ink>
<line page="1" rect="0,0,100,100" name="l1" start="10,10" end="90,90" head="None" tail="OpenArrow" interior-color="#0000FF"/>
<polygon page="1" rect="0,0,100,100" name="p1" style="cloudy" intensity="2"><vertices>10,10,90,10,50,90</vertices></polygon>
<caret page="0" rect="100,100,110,110" name="c1" symbol="paragraph"/>
<freetext page="0" rect="300,300,400,340" name="f1" justification="centered" rotation="90"><contents>Note</contents><defaultappearance>/Helv 12 Tf 0 g</defaultappearance></freetext>
<fileattachment page="0" rect="20,20,40,40" name="a1" icon="Paperclip" file="readings.csv" mimetype="text/csv"><data MODE="raw" encoding="hex" length="9">6461792C310A</data></fileattachment>
<sound page="2" rect="0,0,20,20" name="snd"/>
</annots>
<fields><field name="Name"><value>Ada</value></field></fields>
<f href="report.pdf"/>
<ids original="0A0B" modified="0C0D"/>
</xfdf>
"##;

fn review() -> Exchange {
    read(REVIEW.as_bytes(), &Limits::default()).unwrap()
}

/// Each comment as its page, subtype, name and parent.
fn outline(e: &Exchange) -> Vec<(u32, String, Option<String>, Option<String>)> {
    e.comments
        .iter()
        .map(|c| (c.page, c.subtype().to_owned(), c.name(), c.in_reply_to.clone()))
        .collect()
}

#[test]
fn xfdf_reads_each_comment_with_its_entries_and_the_file_it_was_made_for() {
    let e = review();
    let highlight = &e.comments[0];
    let quads = highlight
        .entries
        .get("QuadPoints")
        .and_then(Object::as_array)
        .map(<[Object]>::len);
    let ink = e.comments[3]
        .entries
        .get("InkList")
        .and_then(Object::as_array)
        .map(<[Object]>::len);
    assert_eq!(
        (
            e.file.as_deref(),
            e.ids.clone(),
            e.comments.len(),
            quads,
            highlight.popup.map(|p| p.rect),
            ink,
            e.comments[8]
                .file
                .as_ref()
                .map(|f| (f.name.as_str(), f.bytes.as_slice())),
        ),
        (
            Some("report.pdf"),
            Some([vec![0x0A, 0x0B], vec![0x0C, 0x0D]]),
            9,
            Some(8),
            Some([200.0, 600.0, 380.0, 700.0]),
            Some(2),
            Some(("readings.csv", b"day,1\n".as_slice())),
        )
    );
}

#[test]
fn a_sound_and_a_form_field_are_listed_as_skipped() {
    let skipped: Vec<(String, SkipReason)> = review().skipped.into_iter().map(|s| (s.kind, s.reason)).collect();
    assert_eq!(
        skipped,
        vec![
            ("Sound".to_owned(), SkipReason::Unsupported),
            ("field".to_owned(), SkipReason::FormField)
        ]
    );
}

#[test]
fn xfdf_written_reads_back_as_it_was() {
    let e = review();
    let again = read(&write_xfdf(&e), &Limits::default()).unwrap();
    assert_eq!(
        (outline(&again), entries(&again), again.file.clone(), again.ids.clone()),
        (outline(&e), entries(&e), e.file.clone(), e.ids.clone())
    );
}

#[test]
fn fdf_written_reads_back_as_the_xfdf_it_came_from() {
    let e = review();
    let fdf = write_fdf(&e);
    let again = read(&fdf, &Limits::default()).unwrap();
    assert_eq!(
        (outline(&again), entries(&again), again.file.clone(), again.ids.clone()),
        (outline(&e), entries(&e), e.file.clone(), e.ids.clone())
    );
}

/// Every comment's entries by key, its popup and its file.
fn entries(e: &Exchange) -> Vec<String> {
    e.comments
        .iter()
        .map(|c| {
            let mut keyed: Vec<String> = c.entries.iter().map(|(k, v)| format!("{k:?}={v:?}")).collect();
            keyed.sort();
            format!("{keyed:?} {:?} {:?}", c.popup, c.file)
        })
        .collect()
}

#[test]
fn an_xfdf_file_with_a_dtd_is_refused() {
    let xfdf =
        b"<?xml version=\"1.0\"?>\n<!DOCTYPE xfdf [<!ENTITY x \"boom\">]>\n<xfdf xmlns=\"http://ns.adobe.com/xfdf/\"/>";
    assert!(matches!(read(xfdf, &Limits::default()), Err(Error::Dtd { line: 2 })));
}

#[test]
fn a_malformed_file_names_its_line() {
    let broken = REVIEW.replace("<contents>Agreed</contents>", "<contents>Agreed</content>");
    let err = read(broken.as_bytes(), &Limits::default()).unwrap_err();
    let fdf = b"%FDF-1.2\n1 0 obj\n<< /Nothing true >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";
    let fdf_err = read(fdf, &Limits::default()).unwrap_err();
    assert_eq!(
        (err.code(), err.to_string(), fdf_err.code()),
        (
            "interchange-malformed",
            "the file isn't valid XFDF near line 8: it isn't well-formed XML".to_owned(),
            "interchange-malformed"
        )
    );
}

#[test]
fn neither_xfdf_nor_fdf_is_an_unknown_format() {
    assert_eq!(
        read(b"%PDF-1.7", &Limits::default()).unwrap_err().code(),
        "interchange-unknown-format"
    );
}
