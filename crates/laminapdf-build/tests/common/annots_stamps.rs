//! `fixtures/golden/annots/stamps.pdf`: the stamps and file attachments `AnnotationAppearances`
//! draws. Page 1, from the top: the sixteen standard stamps in two columns; a JPEG and a
//! translucent PNG image stamp; three dynamic stamps, one per colour; the four attachment icons,
//! the last in red. Page 2 is turned a quarter clockwise and holds a standard and a dynamic
//! stamp drawn upright as it is shown.

use laminapdf_build::{AnnotationAppearances, BuildOptions, Builder, PageSize, Rotation};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AddAnnot, AnnotColor, AnnotSpec, AttachmentFile, AttachmentIcon, DynamicStamp, FileAttachment, Op, Origin, Stamp,
    StampKind, StampName, StampStyle,
};
use laminapdf_test_fixtures::fixture;

use super::{png, rgba_gradient, save};

const HEIGHT: f64 = 841.89;

fn at(x: f64, y: f64) -> UserSpacePoint {
    UserSpacePoint::new(x, HEIGHT - y)
}

/// A box in user space from a page-space box's left, top, width and height.
fn area(left: f64, top: f64, width: f64, height: f64) -> UserSpaceRect {
    UserSpaceRect::from_corners(left, HEIGHT - top - height, left + width, HEIGHT - top)
}

fn two_pages() -> laminapdf_ops::Document {
    let mut b = Builder::new(
        BuildOptions::default()
            .with_id(*b"lamina-annots-5c")
            .with_producer("lamina-build golden")
            .with_creation_date("D:20261118120000Z".parse().unwrap())
            .with_compress(false),
    );
    b.page(PageSize::A4).finish().unwrap();
    let mut turned = b.page(PageSize::A4);
    turned.rotate(Rotation::Clockwise90);
    turned.finish().unwrap();
    b.finish().unwrap()
}

fn lines(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|l| (*l).to_owned()).collect()
}

pub fn stamps() -> Vec<u8> {
    let mut doc = two_pages().with_appearances(AnnotationAppearances);
    let jpeg = doc.add_blob(fixture("images/photo-rgb.jpg"));
    let gradient = png(
        (64, 64),
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &rgba_gradient(64),
        None,
        None,
    );
    let png = doc.add_blob(gradient);
    let csv = doc.add_blob(b"day,reading\n1,12.5\n2,13.1\n".to_vec());
    let pages = doc.pages().unwrap();
    let (first, second) = (pages.get(0).unwrap().id().clone(), pages.get(1).unwrap().id().clone());
    drop(pages);
    let mut specs: Vec<(laminapdf_model::PageId, AnnotSpec, Option<AnnotColor>)> = Vec::new();
    for (name, i) in StampName::ALL.into_iter().zip(0_u8..) {
        let (column, row) = (f64::from(i % 2), f64::from(i / 2));
        let rect = area(50.0 + 260.0 * column, 40.0 + 52.0 * row, 220.0, 42.0);
        specs.push((
            first.clone(),
            AnnotSpec::Stamp(Stamp::new(rect, StampKind::Standard(name))),
            None,
        ));
    }
    for (image, i) in [jpeg, png].into_iter().zip(0_u8..) {
        let rect = area(50.0 + 140.0 * f64::from(i), 470.0, 120.0, 90.0);
        specs.push((
            first.clone(),
            AnnotSpec::Stamp(Stamp::new(rect, StampKind::Image(image))),
            None,
        ));
    }
    let dynamic = [
        (["APPROVED", "by R. Osei", "29 Oct 2026, 10:14"], StampStyle::Green),
        (["NOT APPROVED", "by A. Mensah", "12 Nov 2026"], StampStyle::Red),
        (["RECEIVED", "Café Müller · Accounts", "1 Dec 2026"], StampStyle::Blue),
    ];
    for ((text, style), i) in dynamic.into_iter().zip(0_u8..) {
        let rect = area(50.0 + 170.0 * f64::from(i), 590.0, 160.0, 70.0);
        let stamp = StampKind::Dynamic(DynamicStamp::new(lines(&text), style));
        specs.push((first.clone(), AnnotSpec::Stamp(Stamp::new(rect, stamp)), None));
    }
    let icons = [
        AttachmentIcon::Paperclip,
        AttachmentIcon::PushPin,
        AttachmentIcon::Graph,
        AttachmentIcon::Tag,
    ];
    for (icon, i) in icons.into_iter().zip(0_u8..) {
        let file = AttachmentFile::new(format!("readings-{i}.csv"), csv, "text/csv")
            .with_modified("D:20261118120000Z".parse().unwrap());
        let spec = AnnotSpec::FileAttachment(FileAttachment::new(at(60.0 + 40.0 * f64::from(i), 700.0), icon, file));
        let color = (i == 3).then_some(AnnotColor::Rgb(0.85, 0.15, 0.15));
        specs.push((first.clone(), spec, color));
    }
    // Page 2's user space runs up its unturned page, which is shown turned clockwise.
    specs.push((
        second.clone(),
        AnnotSpec::Stamp(
            Stamp::new(
                UserSpaceRect::from_corners(100.0, 300.0, 150.0, 520.0),
                StampKind::Standard(StampName::Final),
            )
            .with_rotation(90),
        ),
        None,
    ));
    let turned = StampKind::Dynamic(DynamicStamp::new(
        lines(&["REVIEWED", "on a turned page"]),
        StampStyle::Blue,
    ));
    specs.push((
        second,
        AnnotSpec::Stamp(Stamp::new(UserSpaceRect::from_corners(200.0, 300.0, 270.0, 480.0), turned).with_rotation(90)),
        None,
    ));
    doc.transact(Origin::Local, |tx| {
        for (page, spec, color) in specs {
            let mut op = AddAnnot::new(page, spec);
            op.common.color = color;
            tx.apply(Op::from(op))?;
        }
        Ok(())
    })
    .unwrap();
    save(doc)
}
