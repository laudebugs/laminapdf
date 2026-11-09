//! The front matter and the headings' slugs.

use super::FrontMatter;
use super::convert::{front_matter, slug};

#[test]
fn front_matter_reads_its_four_keys_and_leaves_out_the_rest() {
    let markdown =
        "---\ntitle: \"Annual report\"\nlang: fr\nauthor: Ada\nsubject: Sales\ntags: [a, b]\n---\n\n# Body\n";
    assert_eq!(
        front_matter(markdown),
        FrontMatter {
            title: Some("Annual report".to_owned()),
            lang: Some("fr".to_owned()),
            author: Some("Ada".to_owned()),
            subject: Some("Sales".to_owned()),
        }
    );
}

#[test]
fn a_document_without_front_matter_has_none() {
    assert_eq!(
        front_matter("# Title\n\ntitle: not front matter\n"),
        FrontMatter::default()
    );
}

#[test]
fn slugs_follow_github() {
    assert_eq!(slug("Getting started"), "getting-started");
    assert_eq!(slug("What's new, in 2.0?"), "whats-new-in-20");
    assert_eq!(slug("Café crème"), "café-crème");
}
