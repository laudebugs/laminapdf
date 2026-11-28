//! What a `SubmitForm` action would send (ISO 32000-2 §12.7.6.2), built without sending it: the
//! address, the format, the body and the fields in it, and the required fields that are still
//! empty ([E8 plan §2.3]). The reader shows this to the person and sends it only after they
//! confirm that one send; this crate does no input or output.
//!
//! The body carries field values only. A send of the whole PDF (`SubmitPDF`) is refused, and
//! so are an address that isn't `https:` and a field that is a password; the FDF and XFDF
//! bodies name no file and carry no document ID.
//!
//! [E8 plan §2.3]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e8-form-filling.md#23-behaviour-actions-and-submit

use std::fmt::Write as _;

use laminapdf_model::{FieldFlags, FieldKind, FieldValue, Form, SubmitFlags, SubmitForm, TerminalField};

use crate::fields::{DataValue, FieldData, FormDataFormat};
use crate::form_data::data_fields;
use crate::{InterchangeError, Result, write_form_data};

/// How a send encodes the fields.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SubmitFormat {
    /// HTML form encoding: `name=value` pairs joined by `&`, as a web form sends them.
    Html,
    /// FDF (ISO 32000-2 §12.7.8).
    Fdf,
    /// XFDF (ISO 19444-1).
    Xfdf,
}

impl SubmitFormat {
    /// `html`, `fdf` or `xfdf`, as the bindings spell it.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Fdf => "fdf",
            Self::Xfdf => "xfdf",
        }
    }

    /// The media type of a body in this format.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Html => "application/x-www-form-urlencoded",
            Self::Fdf => "application/vnd.fdf",
            Self::Xfdf => "application/vnd.adobe.xfdf",
        }
    }
}

/// The HTTP method of a send.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SubmitMethod {
    /// The body goes in a POST.
    Post,
    /// HTML form encoding only: the fields are the address's query, and there is no body.
    Get,
}

impl SubmitMethod {
    /// `POST` or `GET`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Post => "POST",
            Self::Get => "GET",
        }
    }
}

/// Where a send goes.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SubmitTarget {
    /// A web address: the request is made from the device.
    Web,
    /// A `mailto:` address: the mail app opens with the address and no data, since a mail
    /// can't carry the fields without the person attaching a file.
    Mail,
}

/// One field of a send and the value that leaves the device.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SubmittedField {
    /// The fully qualified name.
    pub name: String,
    /// The value as a person reads it; several choices are joined by a comma and a space.
    pub value: String,
}

/// A send, built and not made.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SubmitRequest {
    /// The address. For a GET, the query is already part of it.
    pub url: String,
    /// The address without its path and query: the scheme and host the dialog names.
    pub host: String,
    /// Where it goes.
    pub target: SubmitTarget,
    /// The method.
    pub method: SubmitMethod,
    /// The encoding.
    pub format: SubmitFormat,
    /// The body; empty for a GET and for a mail.
    pub body: Vec<u8>,
    /// The fields in the body, in the form's order; empty for a mail.
    pub fields: Vec<SubmittedField>,
    /// The fully qualified names of required fields that the send covers and that are empty.
    /// A send with any is blocked until they are filled.
    pub empty_required: Vec<String>,
}

/// `action`'s request on `form`: the fields it covers that hold a value (or all of them with
/// `IncludeNoValueFields`), except those flagged `NoExport` and password fields.
///
/// # Errors
/// `submit-pdf-refused` for a `SubmitPDF` action, and `submit-url-refused` for an address that
/// isn't `https:` or `mailto:`, or is empty.
///
/// Added in 0.7.0.
pub fn submit_request(form: &Form, action: &SubmitForm) -> Result<SubmitRequest> {
    if action.flags.has(SubmitFlags::SUBMIT_PDF) {
        return Err(InterchangeError::SubmitPdfRefused);
    }
    let url = action.url.trim();
    if let Some(mail) = url
        .get(..7)
        .filter(|p| p.eq_ignore_ascii_case("mailto:"))
        .map(|_| &url[7..])
    {
        return mail_request(mail, action.flags);
    }
    let host = https_host(url).ok_or(InterchangeError::SubmitUrlRefused)?;
    let include_empty = action.flags.has(SubmitFlags::INCLUDE_NO_VALUE_FIELDS);
    let covered: Vec<&TerminalField> = action
        .selection
        .fields(form)
        .into_iter()
        .filter(|f| !f.no_export() && !f.flags.has(FieldFlags::PASSWORD))
        .collect();
    let empty_required = covered
        .iter()
        .filter(|f| f.required() && !has_value(f))
        .map(|f| f.name.clone())
        .collect();
    let format = format_of(action.flags);
    // HTML form encoding leaves an unticked box or radio group out, as Acrobat does: a form that
    // wants `Off` asks for FDF or XFDF, which carry it.
    let leaves_off = |f: &TerminalField| {
        format == SubmitFormat::Html && matches!(f.kind, FieldKind::CheckBox | FieldKind::RadioGroup) && !has_value(f)
    };
    let sent: Vec<&TerminalField> = covered
        .into_iter()
        .filter(|f| (include_empty || has_value(f)) && !leaves_off(f))
        .collect();
    let data_format = match format {
        SubmitFormat::Fdf => FormDataFormat::Fdf,
        // HTML form encoding carries a button's export value, as XFDF does.
        SubmitFormat::Html | SubmitFormat::Xfdf => FormDataFormat::Xfdf,
    };
    let data = data_fields(&sent, data_format);
    let fields: Vec<SubmittedField> = data
        .iter()
        .map(|d| SubmittedField {
            name: plain(&d.name),
            value: plain(&shown(&d.value)),
        })
        .collect();
    let get = format == SubmitFormat::Html && action.flags.has(SubmitFlags::GET_METHOD);
    let (url, body, method) = match (format, get) {
        (SubmitFormat::Html, true) => {
            let q = query_of(&data);
            let base = url.split('#').next().unwrap_or(url);
            let joiner = if base.contains('?') { '&' } else { '?' };
            let url = if q.is_empty() {
                base.to_owned()
            } else {
                format!("{base}{joiner}{q}")
            };
            (url, Vec::new(), SubmitMethod::Get)
        }
        (SubmitFormat::Html, false) => (url.to_owned(), query_of(&data).into_bytes(), SubmitMethod::Post),
        (SubmitFormat::Fdf, _) => (
            url.to_owned(),
            write_form_data(FormDataFormat::Fdf, data, None, None),
            SubmitMethod::Post,
        ),
        (SubmitFormat::Xfdf, _) => (
            url.to_owned(),
            write_form_data(FormDataFormat::Xfdf, data, None, None),
            SubmitMethod::Post,
        ),
    };
    Ok(SubmitRequest {
        url,
        host,
        target: SubmitTarget::Web,
        method,
        format,
        body,
        fields,
        empty_required,
    })
}

/// The request of a `mailto:` address: the mail app opens with the address (its query dropped) and
/// no answers.
fn mail_request(mail: &str, flags: SubmitFlags) -> Result<SubmitRequest> {
    let address = mail.split('?').next().unwrap_or_default();
    if address.is_empty()
        || address
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || is_invisible(c))
    {
        return Err(InterchangeError::SubmitUrlRefused);
    }
    Ok(SubmitRequest {
        url: format!("mailto:{address}"),
        host: format!("mailto:{address}"),
        target: SubmitTarget::Mail,
        method: SubmitMethod::Post,
        format: format_of(flags),
        body: Vec::new(),
        fields: Vec::new(),
        empty_required: Vec::new(),
    })
}

/// `data` as `name=value` pairs of HTML form encoding, one pair for each choice.
fn query_of(data: &[FieldData]) -> String {
    let mut out = String::new();
    for d in data {
        for value in values(&d.value) {
            if !out.is_empty() {
                out.push('&');
            }
            out.push_str(&encode(&d.name));
            out.push('=');
            out.push_str(&encode(&value));
        }
    }
    out
}

/// The encoding `flags` asks for: HTML form encoding, XFDF, else FDF (Table 238).
const fn format_of(flags: SubmitFlags) -> SubmitFormat {
    if flags.has(SubmitFlags::EXPORT_FORMAT) {
        SubmitFormat::Html
    } else if flags.has(SubmitFlags::XFDF) {
        SubmitFormat::Xfdf
    } else {
        SubmitFormat::Fdf
    }
}

/// `https://host[:port]` of a public `https:` address; `None` for anything else: another scheme,
/// no host, user information, whitespace or control characters, and a host that is local or
/// private. The host is read the way a browser reads it (an IPv4 address in any spelling is one
/// address) and refused when it is `localhost` or under it, a name ending `.local`, `.internal`
/// or `.localhost`, a name with no dot, or an address in a loopback, private, link-local, shared
/// (CGNAT), multicast or reserved range, an IPv6 one that embeds such an IPv4 address included.
fn https_host(url: &str) -> Option<String> {
    let rest = url
        .get(..8)
        .filter(|s| s.eq_ignore_ascii_case("https://"))
        .map(|_| &url[8..])?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty()
        || authority.contains('@')
        || url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || is_invisible(c))
    {
        return None;
    }
    let (host, port) = split_port(authority)?;
    if port.is_some_and(|p| {
        p.is_empty()
            || p.len() > 5
            || !p.bytes().all(|b| b.is_ascii_digit())
            || p.parse::<u32>().map_or(true, |n| n > 65_535)
    }) {
        return None;
    }
    public_host(host).then(|| format!("https://{authority}"))
}

/// `authority` as its host and, when it has one, its port text.
fn split_port(authority: &str) -> Option<(&str, Option<&str>)> {
    if authority.starts_with('[') {
        let end = authority.find(']')?;
        let host = authority.get(..=end)?;
        return match authority.get(end + 1..)? {
            "" => Some((host, None)),
            tail => Some((host, Some(tail.strip_prefix(':')?))),
        };
    }
    Some(match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    })
}

/// Whether `host` (a name, an IPv4 address in any spelling, or a bracketed IPv6 address) is one a
/// form may be sent to.
fn public_host(host: &str) -> bool {
    if let Some(inner) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        return inner.parse::<std::net::Ipv6Addr>().is_ok_and(|ip| !private_v6(ip));
    }
    let host = host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase();
    if host.is_empty()
        || host.contains('%')
        || host.split('.').any(str::is_empty)
        || host
            .chars()
            .any(|c| matches!(c, '[' | ']' | '\\' | '<' | '>' | '^' | '|' | '"' | '`' | '{' | '}'))
    {
        return false;
    }
    let last = host.rsplit('.').next().unwrap_or_default();
    if last.bytes().all(|b| b.is_ascii_digit()) || last.starts_with("0x") {
        // An IPv4 address in decimal, octal, hex or short form: read it as a browser does.
        return ipv4_of(&host).is_some_and(|ip| !private_v4(ip));
    }
    !(host == "localhost"
        || !host.contains('.')
        || ["localhost", "local", "internal"]
            .iter()
            .any(|s| host.ends_with(&format!(".{s}"))))
}

/// The address a WHATWG browser reads `host` as: one to four numbers (decimal, `0x` hex or
/// leading-zero octal) with the last filling the remaining bytes; `None` for anything else.
fn ipv4_of(host: &str) -> Option<u32> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() > 4 {
        return None;
    }
    let mut numbers = Vec::new();
    for part in &parts {
        let (digits, radix) = match part {
            p if p.len() > 1 && (p.starts_with("0x") || p.starts_with("0X")) => (&p[2..], 16),
            p if p.len() > 1 && p.starts_with('0') => (&p[1..], 8),
            p => (*p, 10),
        };
        numbers.push(if digits.is_empty() {
            0
        } else {
            u64::from_str_radix(digits, radix).ok()?
        });
    }
    let (last, head) = numbers.split_last()?;
    if head.iter().any(|n| *n > 255) || *last >= 256u64.pow(u32::try_from(5 - numbers.len()).ok()?) {
        return None;
    }
    let mut ip = *last;
    for (i, n) in head.iter().enumerate() {
        ip += n << (8 * (3 - i));
    }
    u32::try_from(ip).ok()
}

/// Whether `ip` is loopback, private, link-local, shared, multicast or reserved.
const fn private_v4(ip: u32) -> bool {
    let [a, b, _, _] = ip.to_be_bytes();
    a == 0
        || a == 10
        || a == 127
        || (a == 100 && b >= 64 && b < 128)
        || (a == 169 && b == 254)
        || (a == 172 && b >= 16 && b < 32)
        || (a == 192 && b == 168)
        || a >= 224
}

/// Whether `ip` is loopback, unspecified, unique local, link-local or multicast, or embeds an
/// IPv4 address that is private (mapped, compatible or NAT64).
fn private_v6(ip: std::net::Ipv6Addr) -> bool {
    let s = ip.segments();
    let embedded = |hi: u16, lo: u16| private_v4((u32::from(hi) << 16) | u32::from(lo));
    ip.is_loopback()
        || ip.is_unspecified()
        || (s[0] & 0xfe00) == 0xfc00
        || (s[0] & 0xffc0) == 0xfe80
        || (s[0] & 0xff00) == 0xff00
        || (s[..5] == [0; 5] && s[5] == 0xffff && embedded(s[6], s[7]))
        || (s[..6] == [0; 6] && embedded(s[6], s[7]))
        || (s[0] == 0x64 && s[1] == 0xff9b && s[2..6] == [0; 4] && embedded(s[6], s[7]))
}

/// A format character (bidi controls, zero-width marks, the byte-order mark): invisible, and able
/// to make an address or a name read as another.
const fn is_invisible(c: char) -> bool {
    matches!(c, '\u{ad}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{206f}' | '\u{feff}' | '\u{e0001}' | '\u{e0020}'..='\u{e007f}')
}

/// `text` as shown to a person: control and invisible characters left out, except line breaks and
/// tabs in a value.
fn plain(text: &str) -> String {
    text.chars()
        .filter(|c| matches!(c, '\n' | '\t') || !(c.is_control() || is_invisible(*c)))
        .collect()
}

/// Whether the field holds a value a send carries.
fn has_value(field: &TerminalField) -> bool {
    match &field.value {
        None | Some(FieldValue::Button(None)) => false,
        Some(FieldValue::Text(t)) => !t.is_empty(),
        Some(FieldValue::Choice(v)) => !v.is_empty(),
        Some(_) => true,
    }
}

/// A value as a person reads it.
fn shown(value: &DataValue) -> String {
    match value {
        DataValue::Text(t) | DataValue::State(t) => t.clone(),
        DataValue::Checked(on) => (if *on { "Yes" } else { "No" }).to_owned(),
        DataValue::Choices(v) => v.join(", "),
        DataValue::Empty => String::new(),
    }
}

/// The values a field adds to a query: one, or one for each choice.
fn values(value: &DataValue) -> Vec<String> {
    match value {
        DataValue::Choices(v) if !v.is_empty() => v.clone(),
        other => vec![shown(other)],
    }
}

/// `text` as `application/x-www-form-urlencoded` (the WHATWG rules): UTF-8, line breaks as
/// CRLF, a space as `+`, every byte but ASCII letters, digits and `*-._` percent-encoded.
fn encode(text: &str) -> String {
    let normalised = text.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n");
    let mut out = String::with_capacity(normalised.len());
    for b in normalised.bytes() {
        match b {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'*' | b'-' | b'.' | b'_' => out.push(char::from(b)),
            b' ' => out.push('+'),
            _ => {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_should_follow_the_form_encoding_rules() {
        assert_eq!(encode("a b&c=d\né"), "a+b%26c%3Dd%0D%0A%C3%A9");
        assert_eq!(encode("Az09*-._"), "Az09*-._");
    }

    #[test]
    fn https_host_should_take_public_addresses_in_any_case_and_with_a_trailing_dot() {
        for (url, host) in [
            ("https://8.8.8.8/", "https://8.8.8.8"),
            ("https://172.32.0.1/", "https://172.32.0.1"),
            ("https://100.128.0.1/", "https://100.128.0.1"),
            ("https://[2606:4700::1111]:8443/x", "https://[2606:4700::1111]:8443"),
            ("https://Forms.Example.com./x", "https://Forms.Example.com."),
        ] {
            assert_eq!(https_host(url).as_deref(), Some(host), "{url}");
        }
    }

    #[test]
    fn plain_should_leave_out_what_can_make_text_read_as_other_text() {
        assert_eq!(plain("a\u{202e}b\u{200b}c\x07d\ne"), "abcd\ne");
    }

    #[test]
    fn https_host_should_take_only_plain_https_addresses() {
        assert_eq!(
            https_host("https://forms.example.com/x?y=1").as_deref(),
            Some("https://forms.example.com")
        );
        assert_eq!(
            https_host("HTTPS://example.com:8443").as_deref(),
            Some("https://example.com:8443")
        );
        for refused in [
            "http://example.com/",
            "https://",
            "https:///x",
            "https://user:pw@example.com/",
            "ftp://example.com/",
            "javascript:alert(1)",
            "https://exa mple.com/",
            "",
            "https://localhost/",
            "https://LOCALHOST./x",
            "https://app.localhost/",
            "https://printer.local/",
            "https://db.internal/",
            "https://intranet/",
            "https://[::1]/",
            "https://[::]/",
            "https://[fd00::1]/",
            "https://[fe80::1]/",
            "https://[::ffff:127.0.0.1]/",
            "https://[::ffff:7f00:1]/",
            "https://[::ffff:a00:1]/",
            "https://[64:ff9b::7f00:1]/",
            "https://127.0.0.1/",
            "https://127.1/",
            "https://2130706433/",
            "https://0x7f.0.0.1/",
            "https://0177.0.0.1/",
            "https://017700000001/",
            "https://0.0.0.0/",
            "https://10.1.2.3/",
            "https://172.16.0.1/",
            "https://172.31.255.255/",
            "https://192.168.1.1/",
            "https://169.254.169.254/latest/meta-data",
            "https://100.64.0.1/",
            "https://224.0.0.1/",
            "https://example.com:99999/",
            "https://example.com:/",
            "https://user:pw@example.com/",
            "https://exa%6dple.com/",
            "https://example.com\u{202e}.evil/",
            "https://1.2.3.4.5/",
        ] {
            assert_eq!(https_host(refused), None, "{refused}");
        }
    }
}
