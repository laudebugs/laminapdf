//! Make a PDF/A-3 invoice a Factur-X (ZUGFeRD) one: attach its structured data as
//! factur-x.xml, the document's alternative form, and record the Factur-X properties in the XMP
//! metadata, which keeps the file valid PDF/A-3.
//!
//! The XML here is a placeholder: a real one is the invoice in the EN 16931 Cross Industry
//! Invoice syntax, made by your invoicing system. The file must already be PDF/A-3 (as the
//! `pdf_a3_invoice_with_xml` example writes).
//!
//! ```sh
//! cargo run -p laminapdf --example attach_factur_x -- invoice.pdf <save-time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AfRelationship, AttachTarget, Document, Op, OpenOptions, PdfDate, SaveMode, SaveOptions, SetXmpProperty, XmpValue,
};

/// The Factur-X XMP namespace and prefix (Factur-X 1.0, §6.2.2).
const FX: &str = "urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#";

/// A placeholder for the invoice's Cross Industry Invoice XML.
const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<rsm:CrossIndustryInvoice xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\">
  <!-- The invoice's data, from your invoicing system. -->
</rsm:CrossIndustryInvoice>
";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: attach_factur_x invoice.pdf <save-time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // The time of the save, which your app takes from its clock.
    let saved_at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let invoice = attach(std::fs::read(input)?, saved_at)?;
    std::fs::write(out.join("attach_factur_x.pdf"), invoice)?;
    Ok(())
}

/// The invoice with `factur-x.xml` attached and the Factur-X properties recorded, saved
/// incrementally at `saved_at`.
pub(crate) fn attach(bytes: Vec<u8>, saved_at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;

    // PDF/A-3 asks every attached file for an association, a MIME type and a modification date.
    let mut xml = doc
        .attach_file("factur-x.xml", XML.as_bytes().to_vec(), "text/xml")
        .with_target(AttachTarget::Document, AfRelationship::Alternative);
    xml.description = Some("Factur-X invoice".into());
    xml.modified = Some(saved_at);
    let fx = |name: &str, value: &str| Op::from(SetXmpProperty::new(FX, "fx", name, XmpValue::Text(value.into())));
    doc.apply(vec![
        Op::from(xml),
        fx("DocumentType", "INVOICE"),
        fx("DocumentFileName", "factur-x.xml"),
        fx("Version", "1.0"),
        fx("ConformanceLevel", "MINIMUM"),
    ])?;

    let options = SaveOptions::new()
        .with_mode(SaveMode::Incremental)
        .with_metadata_date(saved_at);
    // An attachment that breaks the file's PDF/A level says so before the save; this one
    // keeps it.
    for warning in doc.plan(&options)?.conformance_warnings {
        eprintln!("{}", warning.code());
    }
    Ok(doc.save(&options)?.bytes)
}
