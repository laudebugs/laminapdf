//! The XMP metadata packet (ISO 32000-2 §14.3.2; ISO 16684-1; research 04 §5.2), written
//! from the same [`Metadata`] as the Info dictionary so the two agree.
//!
//! The packet is deterministic: one `rdf:Description` with the namespaces in a fixed order,
//! properties in a fixed order, values escaped for XML, no padding.

use std::fmt::Write as _;

use lamina_model::PdfDate;

use super::info::Metadata;

/// `&`, `<`, `>` and `"` escaped; tab, line feed and carriage return as character references;
/// other characters XML 1.0 can't hold (C0 controls, U+FFFE, U+FFFF) are left out.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            '\u{0}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}' => {}
            _ => out.push(ch),
        }
    }
    out
}

/// A date as XMP writes it (ISO 8601): `2026-10-29T12:00:00Z`, `…+01:00`, or no zone when
/// the PDF date gives none.
pub(crate) fn date(d: &PdfDate) -> String {
    let mut s = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        d.year(),
        d.month(),
        d.day(),
        d.hour(),
        d.minute(),
        d.second()
    );
    match d.utc_offset_minutes() {
        None => {}
        Some(0) => s.push('Z'),
        Some(m) => {
            let sign = if m < 0 { '-' } else { '+' };
            let m = m.unsigned_abs();
            let _ = write!(s, "{sign}{:02}:{:02}", m / 60, m % 60);
        }
    }
    s
}

/// The packet: `<?xpacket … ?>` around `x:xmpmeta`, UTF-8.
pub(crate) fn packet(meta: &Metadata) -> String {
    let mut body = String::new();
    let mut line = |s: &str| {
        body.push_str("   ");
        body.push_str(s);
        body.push('\n');
    };
    line("<dc:format>application/pdf</dc:format>");
    if let Some(title) = &meta.title {
        line(&format!(
            "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>",
            escape(title)
        ));
    }
    if let Some(author) = &meta.author {
        line(&format!(
            "<dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>",
            escape(author)
        ));
    }
    if let Some(subject) = &meta.subject {
        line(&format!(
            "<dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>",
            escape(subject)
        ));
    }
    for (tag, value) in [
        ("pdf:Keywords", &meta.keywords),
        ("pdf:Producer", &meta.producer),
        ("xmp:CreatorTool", &meta.creator),
    ] {
        if let Some(v) = value {
            line(&format!("<{tag}>{}</{tag}>", escape(v)));
        }
    }
    for (tag, value) in [
        ("xmp:CreateDate", &meta.creation_date),
        ("xmp:ModifyDate", &meta.mod_date),
    ] {
        if let Some(d) = value {
            line(&format!("<{tag}>{}</{tag}>", date(d)));
        }
    }
    format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n \
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n  \
         <rdf:Description rdf:about=\"\" \
         xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\" \
         xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\n\
         {body}  </rdf:Description>\n \
         </rdf:RDF>\n\
         </x:xmpmeta>\n\
         <?xpacket end=\"w\"?>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_escaped() {
        assert_eq!(escape("a<b & \"c\">\u{1}\n"), "a&lt;b &amp; &quot;c&quot;&gt;&#xA;");
    }

    #[test]
    fn dates_carry_their_zone() {
        let d = |s: &str| date(&s.parse().unwrap());
        assert_eq!(
            [d("D:20261029120000Z"), d("D:20261029120000-05'30'"), d("D:2026")],
            [
                "2026-10-29T12:00:00Z",
                "2026-10-29T12:00:00-05:30",
                "2026-01-01T00:00:00"
            ]
        );
    }

    #[test]
    fn the_packet_holds_every_value_once() {
        let meta = Metadata::default().with_title("T").with_producer("P");
        let p = packet(&meta);
        assert!(
            p.contains("<rdf:li xml:lang=\"x-default\">T</rdf:li>") && p.contains("<pdf:Producer>P</pdf:Producer>")
        );
        assert!(!p.contains("dc:creator"));
    }
}
