//! Stamps, file attachments and sound comments through the façade: read from other apps' files
//! (`fixtures/annotations/`), made from page-space boxes and points, saved and read back, undone,
//! refused when they can't be drawn, held to a PDF/A claim, and sound comments turned into WAV.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code: a panic is a test failure"
)]

use laminapdf::{
    AddReply, AnnotColor, AnnotDetails, AnnotSpec, AnnotSpecPatch, AttachmentIcon, ConformanceWarning, Document,
    DynamicStamp, Error, FileAttachmentPatch, FileToAttach, Op, OpenOptions, PageAnnotation, PointPt, Pt, RectPt,
    SaveOptions, SoundEncoding, StampKind, StampName, StampStyle, UpdateAnnot, Warning,
};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(name), OpenOptions::default()).unwrap()
}

fn area() -> RectPt {
    RectPt::from_points(300.0, 60.0, 180.0, 50.0)
}

fn at() -> PointPt {
    PointPt::new(Pt::new(40.0), Pt::new(40.0))
}

fn id(record: &PageAnnotation) -> laminapdf::AnnotId {
    record.id.parse().unwrap()
}

fn code(e: &Error) -> &str {
    e.code()
}

#[test]
fn another_apps_stamps_read_with_their_names() {
    let names: Vec<Option<String>> = open("annotations/stamps.pdf")
        .page_annotations(0)
        .unwrap()
        .into_iter()
        .map(|a| match a.details {
            AnnotDetails::Stamp { name } => name,
            other => panic!("{other:?}"),
        })
        .collect();
    // The fourteen standard names, in the standard's order, then the other apps' stamps.
    let standard = StampName::ALL.iter().take(14).map(|n| Some(n.name().to_owned()));
    let others = [Some("SBSignHere"), None, Some("#9A3F1C2B"), Some("Final")].map(|n| n.map(str::to_owned));
    assert_eq!(names, standard.chain(others).collect::<Vec<_>>());
}

#[test]
fn attachments_read_with_their_files_and_their_bytes_come_back() {
    let doc = open("annotations/attachments.pdf");
    let annots = doc.page_annotations(0).unwrap();
    let mut icons = Vec::new();
    for annot in &annots {
        let AnnotDetails::FileAttachment {
            icon,
            file,
            name,
            mime,
            size,
            ..
        } = &annot.details
        else {
            panic!("{:?}", annot.details);
        };
        icons.push(icon.clone().unwrap());
        let bytes = doc.attachment_bytes(file.as_ref().unwrap()).unwrap().bytes;
        assert_eq!(
            (mime.as_deref(), *size),
            (Some("text/plain"), Some(u64::try_from(bytes.len()).unwrap()))
        );
        assert!(name.as_ref().unwrap().contains(".txt"));
    }
    assert_eq!(icons, ["PushPin", "Paperclip", "Graph", "Tag"]);
}

/// The WAV's channels, rate, bits a sample, and its samples.
fn wav_parts(wav: &[u8]) -> (u16, u32, u16, &[u8]) {
    assert_eq!(
        (&wav[..4], &wav[8..12], &wav[36..40]),
        (&b"RIFF"[..], &b"WAVE"[..], &b"data"[..])
    );
    let u16_at = |i: usize| u16::from_le_bytes([wav[i], wav[i + 1]]);
    let rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
    (u16_at(22), rate, u16_at(34), &wav[44..])
}

/// capability: D.09 core read
#[test]
fn every_sound_format_plays_as_wav() {
    let doc = open("annotations/sound.pdf");
    let annots = doc.page_annotations(0).unwrap();
    let tone: Vec<f64> = (0..400)
        .map(|i| 12000.0 * (2.0 * std::f64::consts::PI * 440.0 * f64::from(i) / 8000.0).sin())
        .collect();
    let mut formats = Vec::new();
    for annot in &annots[..6] {
        let AnnotDetails::Sound {
            format: Some(format), ..
        } = &annot.details
        else {
            panic!("{:?}", annot.details);
        };
        formats.push(format.encoding);
        let wav = doc.sound_as_wav(&id(annot)).unwrap();
        let (channels, rate, bits, data) = wav_parts(&wav);
        assert_eq!((channels, rate), (format.channels, 8000));
        // Decoded back to the tone, each format within its precision.
        let (samples, tolerance): (Vec<f64>, f64) = if bits == 8 {
            let step = if format.bits == 4 { 4096.0 } else { 256.0 };
            (
                data.iter().map(|b| (f64::from(*b) - 128.0) * 256.0).collect(),
                step * 1.5,
            )
        } else {
            let all: Vec<f64> = data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| f64::from(i16::from_le_bytes(*c)))
                .collect();
            // Stereo's left channel is the tone.
            (all.into_iter().step_by(usize::from(channels)).collect(), 800.0)
        };
        assert_eq!(samples.len(), 400, "{:?}", format.encoding);
        let worst = samples
            .iter()
            .zip(&tone)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(
            worst <= tolerance,
            "{:?} {} bits: off by {worst}",
            format.encoding,
            format.bits
        );
    }
    assert_eq!(
        formats,
        [
            SoundEncoding::Raw,
            SoundEncoding::Signed,
            SoundEncoding::MuLaw,
            SoundEncoding::ALaw,
            SoundEncoding::Raw,
            SoundEncoding::Signed
        ]
    );
}

#[test]
fn a_sound_that_cant_play_and_a_mark_that_isnt_one_are_unavailable() {
    let doc = open("annotations/sound.pdf");
    let compressed = &doc.page_annotations(0).unwrap()[6];
    assert!(matches!(
        &compressed.details,
        AnnotDetails::Sound { format: Some(f), seconds: Some(s) } if f.compressed && (s - 0.05).abs() < 1e-9
    ));
    assert_eq!(
        code(&doc.sound_as_wav(&id(compressed)).unwrap_err()),
        "sound-unavailable"
    );
    let stamps = open("annotations/stamps.pdf");
    let stamp = &stamps.page_annotations(0).unwrap()[0];
    assert_eq!(code(&stamps.sound_as_wav(&id(stamp)).unwrap_err()), "sound-unavailable");
}

#[test]
fn a_stamp_saves_with_its_name_and_reads_back() {
    let mut doc = open("corpus/hello.pdf");
    let stamp = doc
        .stamp_at(0, area(), StampKind::Standard(StampName::NotApproved))
        .unwrap();
    doc.apply(vec![stamp.into()]).unwrap();
    let saved = doc.save(&SaveOptions::new()).unwrap().bytes;
    let reopened = Document::open_bytes(saved, OpenOptions::default()).unwrap();
    let annot = reopened.page_annotations(0).unwrap().pop().unwrap();
    assert_eq!(
        (annot.subtype.as_str(), annot.contents.as_deref(), annot.has_appearance),
        ("Stamp", Some("Not approved"), true)
    );
    assert_eq!(
        annot.details,
        AnnotDetails::Stamp {
            name: Some("NotApproved".to_owned())
        }
    );
    let rect = annot.rect.unwrap();
    assert!((rect.width().get() - 180.0).abs() < 1e-6 && (rect.height().get() - 50.0).abs() < 1e-6);
}

#[test]
fn an_attached_file_comes_back_byte_for_byte_and_lists_its_annotation() {
    let mut doc = open("corpus/hello.pdf");
    let bytes = b"day,reading\n1,12.5\n".to_vec();
    let op = doc
        .attach_file_at(0, at(), "readings.csv", bytes.clone(), "text/csv")
        .unwrap();
    let edited = doc.apply(vec![op.into()]).unwrap();
    let file = edited.created_attachments[0].clone();
    assert_eq!(doc.attachment_bytes(&file).unwrap().bytes, bytes);
    let listed = doc.attachments().unwrap();
    let holders = &listed.iter().find(|a| a.id == file).unwrap().holders;
    assert!(
        matches!(holders.as_slice(), [laminapdf::AttachmentHolder::FileAttachment { .. }]),
        "{holders:?}"
    );
    let annot = doc.page_annotations(0).unwrap().pop().unwrap();
    assert_eq!(annot.contents.as_deref(), Some("readings.csv"));
}

#[test]
fn a_new_file_takes_the_old_ones_place_and_the_annotation_keeps_its_reply() {
    let mut doc = open("corpus/hello.pdf");
    let attach = doc
        .attach_file_at(0, at(), "draft.csv", b"day\n1\n".to_vec(), "text/csv")
        .unwrap();
    doc.apply(vec![attach.into()]).unwrap();
    let attached = id(&doc.page_annotations(0).unwrap()[0]);
    doc.apply(vec![AddReply::new(attached.clone(), "Use the final figures").into()])
        .unwrap();
    let bytes = b"day,reading\n1,12.5\n".to_vec();
    let blob = doc.add_blob(bytes.clone());
    let mut replace = UpdateAnnot::new(attached.clone());
    replace.spec = Some(AnnotSpecPatch::FileAttachment(FileAttachmentPatch::file(
        FileToAttach::new("final.csv", blob, "text/csv"),
    )));
    doc.apply(vec![replace.into()]).unwrap();
    let annots = doc.page_annotations(0).unwrap();
    let file = match &annots[0].details {
        AnnotDetails::FileAttachment {
            file: Some(file),
            name: Some(name),
            ..
        } if name == "final.csv" => file.clone(),
        other => panic!("{other:?}"),
    };
    let replies = annots
        .iter()
        .filter(|a| a.in_reply_to.as_deref() == Some(attached.to_string().as_str()))
        .count();
    assert_eq!(
        (id(&annots[0]), doc.attachment_bytes(&file).unwrap().bytes, replies),
        (attached, bytes, 1)
    );
}

#[test]
fn an_attachments_icon_and_a_stamps_colour_are_drawn_again() {
    let mut doc = open("corpus/hello.pdf");
    let stamp = doc.stamp_at(0, area(), StampKind::Standard(StampName::Final)).unwrap();
    let attach = doc
        .attach_file_at(0, at(), "a.txt", b"a".to_vec(), "text/plain")
        .unwrap();
    doc.apply(vec![stamp.into(), attach.into()]).unwrap();
    let annots = doc.page_annotations(0).unwrap();
    let (stamp, attached) = (id(&annots[0]), id(&annots[1]));
    let mut recolour = UpdateAnnot::new(stamp);
    recolour.color = Some(Some(AnnotColor::Rgb(0.5, 0.0, 0.5)));
    let mut icon = UpdateAnnot::new(attached);
    icon.spec = Some(AnnotSpecPatch::FileAttachment(FileAttachmentPatch::icon(
        AttachmentIcon::Tag,
    )));
    doc.apply(vec![recolour.into(), icon.into()]).unwrap();
    let annots = doc.page_annotations(0).unwrap();
    assert_eq!(annots[0].color, vec![0.5, 0.0, 0.5]);
    assert!(
        matches!(&annots[1].details, AnnotDetails::FileAttachment { icon: Some(i), name: Some(n), .. }
        if i == "Tag" && n == "a.txt")
    );
}

#[test]
fn a_dynamic_stamp_needs_characters_helvetica_draws() {
    let mut doc = open("corpus/hello.pdf");
    let fine = StampKind::Dynamic(DynamicStamp::new(
        vec!["APPROVED".into(), "by R. Osei · 29 Oct 2026".into()],
        StampStyle::Green,
    ));
    let stamp = doc.stamp_at(0, area(), fine).unwrap();
    doc.apply(vec![stamp.into()]).unwrap();
    let chinese = StampKind::Dynamic(DynamicStamp::new(vec!["批准".into()], StampStyle::Green));
    let stamp = doc.stamp_at(0, area(), chinese).unwrap();
    assert_eq!(
        code(&doc.apply(vec![stamp.into()]).unwrap_err()),
        "annotation-glyph-missing"
    );
}

#[test]
fn an_image_stamp_needs_its_bytes_and_a_jpeg_or_png() {
    let mut doc = open("corpus/hello.pdf");
    let missing = laminapdf::BlobId::of(b"never added");
    let stamp = doc.stamp_at(0, area(), StampKind::Image(missing)).unwrap();
    assert_eq!(code(&doc.apply(vec![stamp.into()]).unwrap_err()), "unknown-blob");
    let text = doc.add_blob(b"GIF89a, not supported".to_vec());
    let stamp = doc.stamp_at(0, area(), StampKind::Image(text)).unwrap();
    assert_eq!(code(&doc.apply(vec![stamp.into()]).unwrap_err()), "stamp-image-invalid");
    let jpeg = doc.add_blob(fixture("images/photo-rgb.jpg"));
    let stamp = doc.stamp_at(0, area(), StampKind::Image(jpeg)).unwrap();
    doc.apply(vec![stamp.into()]).unwrap();
}

#[test]
fn a_stamp_on_a_turned_page_turns_with_it() {
    let doc = open("annotations/flatten-cases.pdf");
    let op = doc.stamp_at(1, area(), StampKind::Standard(StampName::Void)).unwrap();
    let AnnotSpec::Stamp(stamp) = &op.spec else { panic!() };
    assert_eq!(
        (
            stamp.rotation,
            stamp.rect.x1 - stamp.rect.x0,
            stamp.rect.y1 - stamp.rect.y0
        ),
        (90, 50.0, 180.0)
    );
}

fn attachment_warnings(doc: &Document) -> Vec<ConformanceWarning> {
    doc.plan(&SaveOptions::new())
        .unwrap()
        .conformance_warnings
        .into_iter()
        .filter_map(|w| match w {
            Warning::Conformance(
                c @ (ConformanceWarning::PdfaAttachmentForbidden { .. }
                | ConformanceWarning::PdfaAttachmentIncomplete { .. }),
            ) => Some(c),
            _ => None,
        })
        .collect()
}

#[test]
fn attaching_on_pdf_a_2_is_forbidden_and_pdf_a_3_needs_a_date() {
    let mut a2 = open("golden/examples/pdf_a_archive-2b.pdf");
    let op = a2
        .attach_file_at(0, at(), "a.txt", b"a".to_vec(), "text/plain")
        .unwrap();
    a2.apply(vec![op.into()]).unwrap();
    assert!(matches!(
        attachment_warnings(&a2).as_slice(),
        [ConformanceWarning::PdfaAttachmentForbidden { .. }]
    ));

    let mut a3 = open("golden/examples/pdf_a3_invoice_with_xml.pdf");
    let op = a3
        .attach_file_at(0, at(), "a.txt", b"a".to_vec(), "text/plain")
        .unwrap();
    a3.apply(vec![op.into()]).unwrap();
    assert!(matches!(
        attachment_warnings(&a3).as_slice(),
        [ConformanceWarning::PdfaAttachmentIncomplete { .. }]
    ));
    a3.undo().unwrap();
    let mut op = a3
        .attach_file_at(0, at(), "a.txt", b"a".to_vec(), "text/plain")
        .unwrap();
    let AnnotSpec::FileAttachment(file) = &mut op.spec else {
        panic!()
    };
    file.file.modified = Some("D:20261118120000Z".parse().unwrap());
    a3.apply(vec![Op::from(op)]).unwrap();
    assert_eq!(attachment_warnings(&a3), Vec::new());
}
