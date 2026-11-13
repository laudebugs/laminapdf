use laminapdf_cos::{Dict, Object};

use super::target::checked_uri;
use super::types::{DestinationView, OutlineStyle, Rgb};
use crate::error::Error;

#[test]
fn only_absolute_http_https_and_mailto_addresses_pass() {
    for ok in [
        "https://example.com",
        "http://example.com/a?b=c#d",
        "HTTPS://EXAMPLE.COM",
        "mailto:a@example.com",
    ] {
        assert_eq!(checked_uri(ok).map_err(|e| e.code()), Ok(ok));
    }
    for refused in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "example.com",
        "https:/example.com",
        "https:///path",
        "https://",
        "mailto:",
        "https://exa mple.com",
        "https://café.example",
        "data:text/html,hi",
    ] {
        assert_eq!(checked_uri(refused), Err(Error::LinkUriRefused), "{refused}");
    }
}

#[test]
fn a_colour_reads_and_writes_as_hex() {
    let red: Rgb = "#ff0080".parse().unwrap();
    assert_eq!(red, Rgb::new(255, 0, 128));
    assert_eq!(red.to_string(), "#ff0080");
    for bad in ["ff0080", "#ff008", "#gg0080", "#ff00800"] {
        assert!(bad.parse::<Rgb>().is_err(), "{bad}");
    }
}

#[test]
fn a_colour_from_a_pdf_array_is_clamped_and_rounded() {
    let c = Rgb::from_pdf(&[Object::Real(1.5), Object::Real(0.5), Object::Integer(0)]);
    assert_eq!(c, Some(Rgb::new(255, 128, 0)));
    assert_eq!(Rgb::from_pdf(&[Object::Integer(1)]), None);
}

#[test]
fn a_style_writes_c_and_f_and_removes_them_at_the_default() {
    let mut dict: Dict = [("F", Object::Integer(4))].into_iter().collect();
    OutlineStyle::new(Some(Rgb::new(255, 0, 0)), true, true).write(&mut dict);
    assert_eq!(dict.get("F"), Some(&Object::Integer(7)));
    assert!(dict.get("C").is_some());
    OutlineStyle::default().write(&mut dict);
    assert_eq!(
        dict.get("F"),
        Some(&Object::Integer(4)),
        "a bit this type doesn't know stays"
    );
    assert!(dict.get("C").is_none());
    let mut plain = Dict::new();
    OutlineStyle::default().write(&mut plain);
    assert!(plain.is_empty());
}

#[test]
fn a_view_with_a_number_that_is_not_finite_writes_nothing() {
    let view = DestinationView::Xyz {
        left: Some(f64::NAN),
        top: None,
        zoom: None,
    };
    assert_eq!(view.to_pdf(), None);
    let fit_r = DestinationView::FitR {
        left: 0.0,
        bottom: 0.0,
        right: f64::INFINITY,
        top: 1.0,
    };
    assert_eq!(fit_r.to_pdf(), None);
    assert_eq!(DestinationView::Fit.to_pdf(), Some(vec![Object::name("Fit")]));
}
