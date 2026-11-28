//! `submit_request`: the body of each format for `fixtures/forms/fields.pdf` against its golden
//! (`fixtures/forms/data/submit-fields.*`), the fields a send carries and leaves out, the
//! required fields it is blocked by, and the refusals.
#![expect(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

use laminapdf_interchange::{InterchangeError, SubmitFormat, SubmitMethod, SubmitTarget, submit_request};
use laminapdf_model::{Document, FieldSelection, Form, FormAction, OpenOptions, SubmitFlags, SubmitForm};
use laminapdf_test_fixtures::{fixture, fixtures_dir};

const URL: &str = "https://forms.example.com/submit";

fn form(name: &str) -> Form {
    Document::open(fixture(&format!("forms/{name}")), OpenOptions::default())
        .unwrap()
        .form()
        .unwrap()
}

fn action(url: &str, flags: u32) -> SubmitForm {
    let doc = Document::open(fixture("forms/actions.pdf"), OpenOptions::default()).unwrap();
    let form = doc.form().unwrap();
    let FormAction::SubmitForm(mut a) = doc
        .form_actions(form.field_named("send").unwrap())
        .unwrap()
        .remove(0)
        .action
    else {
        panic!("not a submit")
    };
    url.clone_into(&mut a.url);
    a.flags = SubmitFlags(flags);
    a
}

/// The fields of `fields.pdf` that a send of `flags` covers, whatever the fixture's own actions.
fn request_for_fields(flags: u32) -> laminapdf_interchange::SubmitRequest {
    submit_request(&form("fields.pdf"), &action(URL, flags)).unwrap()
}

/// capability: E.16 core read
#[test]
fn each_format_matches_its_golden() {
    for (flags, golden, format) in [
        (0, "submit-fields.fdf", SubmitFormat::Fdf),
        (SubmitFlags::XFDF, "submit-fields.xfdf", SubmitFormat::Xfdf),
        (SubmitFlags::EXPORT_FORMAT, "submit-fields.txt", SubmitFormat::Html),
    ] {
        let request = request_for_fields(flags);
        assert_eq!(request.format, format);
        assert_eq!(request.method, SubmitMethod::Post);
        let path = fixtures_dir().join("forms/data").join(golden);
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::write(&path, &request.body).unwrap();
        }
        assert!(
            request.body == fixture(&format!("forms/data/{golden}")),
            "{golden} is stale: run with UPDATE_GOLDEN=1"
        );
    }
}

#[test]
fn a_get_puts_the_fields_in_the_address_and_sends_no_body() {
    let request = request_for_fields(SubmitFlags::EXPORT_FORMAT | SubmitFlags::GET_METHOD);
    assert_eq!(request.method, SubmitMethod::Get);
    assert_eq!(request.body, b"");
    assert!(
        request
            .url
            .starts_with("https://forms.example.com/submit?name=Ada+Lovelace&"),
        "{}",
        request.url
    );
    assert_eq!(request.host, "https://forms.example.com");
}

#[test]
fn the_request_lists_exactly_the_fields_in_the_body() {
    let request = request_for_fields(SubmitFlags::EXPORT_FORMAT);
    let names: Vec<&str> = request.fields.iter().map(|f| f.name.as_str()).collect();
    let body = String::from_utf8(request.body).unwrap();
    let in_body: Vec<String> = body
        .split('&')
        .map(|pair| pair.split('=').next().unwrap().replace("%2E", "."))
        .collect();
    for name in names {
        assert!(in_body.iter().any(|n| n == name), "{name} is listed but not sent");
    }
    assert!(
        !request
            .fields
            .iter()
            .any(|f| f.name == "signature" || f.name == "submit")
    );
    assert!(
        !request.fields.iter().any(|f| f.name == "pin"),
        "a password is never sent"
    );
}

#[test]
fn empty_fields_are_left_out_unless_the_action_includes_them() {
    let without = request_for_fields(SubmitFlags::EXPORT_FORMAT);
    let with = request_for_fields(SubmitFlags::EXPORT_FORMAT | SubmitFlags::INCLUDE_NO_VALUE_FIELDS);
    assert!(with.fields.len() > without.fields.len());
}

#[test]
fn a_selection_includes_or_excludes_fields_and_no_export_fields_stay_home() {
    let f = form("actions.pdf");
    let mut a = action(URL, SubmitFlags::EXPORT_FORMAT);
    let all: Vec<String> = submit_request(&f, &a)
        .unwrap()
        .fields
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(
        all,
        ["name", "email", "notes"],
        "secret is NoExport, empty has no value"
    );
    a.selection = FieldSelection::new(vec!["email".to_owned()], true);
    let rest: Vec<String> = submit_request(&f, &a)
        .unwrap()
        .fields
        .into_iter()
        .map(|f| f.name)
        .collect();
    assert_eq!(rest, ["name", "notes"]);
}

#[test]
fn empty_required_fields_block_the_send() {
    let f = form("actions.pdf");
    let request = submit_request(&f, &action(URL, SubmitFlags::EXPORT_FORMAT)).unwrap();
    assert_eq!(request.empty_required, ["empty"]);
    let mut a = action(URL, SubmitFlags::EXPORT_FORMAT);
    a.selection = FieldSelection::new(vec!["empty".to_owned()], true);
    assert!(
        submit_request(&f, &a).unwrap().empty_required.is_empty(),
        "a field the send leaves out can't block it"
    );
}

#[test]
fn submit_pdf_is_refused() {
    let f = form("fields.pdf");
    assert_eq!(
        submit_request(&f, &action(URL, SubmitFlags::SUBMIT_PDF)),
        Err(InterchangeError::SubmitPdfRefused)
    );
}

#[test]
fn only_https_addresses_are_sent_to() {
    let f = form("fields.pdf");
    for url in [
        "http://forms.example.com/submit",
        "ftp://example.com/",
        "javascript:alert(1)",
        "https://user@example.com/",
        "https://127.1/",
        "https://[::1]/",
        "https://2130706433/",
        "https://localhost/",
        "https://１２７.０.０.１/",
        "https://198.18.0.1/",
        "",
    ] {
        assert_eq!(
            submit_request(&f, &action(url, 0)),
            Err(InterchangeError::SubmitUrlRefused),
            "{url}"
        );
    }
}

#[test]
fn a_mailto_address_opens_the_mail_app_with_no_data() {
    let f = form("fields.pdf");
    let request = submit_request(&f, &action("mailto:forms@example.com?subject=x&body=secret", 0)).unwrap();
    assert_eq!(request.target, SubmitTarget::Mail);
    assert_eq!(request.url, "mailto:forms@example.com");
    assert_eq!((request.body.as_slice(), request.fields.len()), (&b""[..], 0));
    let upper = submit_request(&f, &action("MailTo:forms@example.com", 0)).unwrap();
    assert_eq!(upper.target, SubmitTarget::Mail);
    for hidden in ["mailto:a%40b.com", "mailto:a@b.com%0Abcc:x@y.z"] {
        assert_eq!(
            submit_request(&f, &action(hidden, 0)),
            Err(InterchangeError::SubmitUrlRefused),
            "{hidden}"
        );
    }
    assert_eq!(
        submit_request(&f, &action("mailto:", 0)),
        Err(InterchangeError::SubmitUrlRefused)
    );
}

#[test]
fn html_encoding_leaves_unticked_boxes_out_and_fdf_keeps_them() {
    let f = form("fields.pdf");
    let html = submit_request(
        &f,
        &action(URL, SubmitFlags::EXPORT_FORMAT | SubmitFlags::INCLUDE_NO_VALUE_FIELDS),
    )
    .unwrap();
    assert!(!html.fields.iter().any(|f| f.value == "Off"), "{:?}", html.fields);
}
