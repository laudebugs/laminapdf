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

use laminapdf_model::{FieldFlags, FieldValue, Form, SubmitFlags, SubmitForm, TerminalField};

use crate::fields::{DataValue, FormDataFormat};
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
    if let Some(mail) = url.strip_prefix("mailto:").or_else(|| url.strip_prefix("MAILTO:")) {
        let address = mail.split('?').next().unwrap_or_default();
        if address.is_empty() {
            return Err(InterchangeError::SubmitUrlRefused);
        }
        return Ok(SubmitRequest {
            url: format!("mailto:{address}"),
            host: format!("mailto:{address}"),
            target: SubmitTarget::Mail,
            method: SubmitMethod::Post,
            format: format_of(action.flags),
            body: Vec::new(),
            fields: Vec::new(),
            empty_required: Vec::new(),
        });
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
    let sent: Vec<&TerminalField> = covered.into_iter().filter(|f| include_empty || has_value(f)).collect();
    let format = format_of(action.flags);
    let data_format = match format {
        SubmitFormat::Fdf => FormDataFormat::Fdf,
        // HTML form encoding carries a button's export value, as XFDF does.
        SubmitFormat::Html | SubmitFormat::Xfdf => FormDataFormat::Xfdf,
    };
    let data = data_fields(&sent, data_format);
    let fields: Vec<SubmittedField> = data
        .iter()
        .map(|d| SubmittedField {
            name: d.name.clone(),
            value: shown(&d.value),
        })
        .collect();
    let get = format == SubmitFormat::Html && action.flags.has(SubmitFlags::GET_METHOD);
    let query = || {
        let mut out = String::new();
        for d in &data {
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
    };
    let (url, body, method) = match (format, get) {
        (SubmitFormat::Html, true) => {
            let q = query();
            let base = url.split('#').next().unwrap_or(url);
            let joiner = if base.contains('?') { '&' } else { '?' };
            let url = if q.is_empty() {
                base.to_owned()
            } else {
                format!("{base}{joiner}{q}")
            };
            (url, Vec::new(), SubmitMethod::Get)
        }
        (SubmitFormat::Html, false) => (url.to_owned(), query().into_bytes(), SubmitMethod::Post),
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

/// `https://host[:port]` of an `https:` address with a host and no user information; `None` for
/// anything else.
fn https_host(url: &str) -> Option<String> {
    let rest = url
        .get(..8)
        .filter(|s| s.eq_ignore_ascii_case("https://"))
        .map(|_| &url[8..])?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') || url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return None;
    }
    Some(format!("https://{authority}"))
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
        ] {
            assert_eq!(https_host(refused), None, "{refused}");
        }
    }
}
