//! Hand-built classic files (a header, numbered objects, a correct cross-reference table) for
//! tests that need exact bytes rather than what a writer produces.

/// A classic file around object bodies numbered `1..`, with a correct xref table and the
/// trailer dictionary `trailer` written as given.
#[must_use]
pub fn classic(objects: &[&[u8]], trailer: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes());
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n{trailer}\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// [`classic`] with object 1 as the catalog: the trailer is `<< /Size … /Root 1 0 R `
/// `extra >>`, so `extra` adds entries such as `/Info 3 0 R`.
#[must_use]
pub fn classic_with_root(objects: &[&str], extra: &str) -> Vec<u8> {
    let bodies: Vec<&[u8]> = objects.iter().map(|o| o.as_bytes()).collect();
    let trailer = format!("<< /Size {} /Root 1 0 R {extra} >>", objects.len() + 1);
    classic(&bodies, &trailer)
}

/// A stream object body: `dict` (without `/Length`) plus `data`, with a correct `/Length`.
#[must_use]
pub fn stream_body(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

#[cfg(test)]
mod tests {
    use super::{classic, classic_with_root};

    #[test]
    fn the_xref_offsets_point_at_each_object() {
        let file = classic(&[b"<< >>", b"42"], "<< /Size 3 >>");
        let text = String::from_utf8(file).unwrap();
        let second = text.find("2 0 obj").unwrap();
        assert!(text.contains(&format!("{second:010} 00000 n\r\n")), "{text}");
    }

    #[test]
    fn classic_with_root_names_object_1_the_root() {
        let file = classic_with_root(&["<< /Type /Catalog >>"], "/Info 2 0 R");
        let text = String::from_utf8(file).unwrap();
        assert!(
            text.contains("trailer\n<< /Size 2 /Root 1 0 R /Info 2 0 R >>\n"),
            "{text}"
        );
    }
}
