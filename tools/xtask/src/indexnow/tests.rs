use std::path::PathBuf;

use super::*;

const KEY: &str = "98a22658c5b8802dbad352a2fc9a6929";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> String {
    root()
        .join("fixtures/indexnow")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| (*a).to_owned()).collect()
}

#[test]
fn a_urlset_should_give_each_page_with_its_lastmod() {
    let parsed = parse(
        "<urlset><url><loc> https://a.example/x?p=1&amp;q=2 </loc><lastmod>2026-11-01</lastmod></url>\
         <url><loc>https://a.example/y</loc></url></urlset>",
    );
    assert_eq!(
        parsed.urls,
        [
            ("https://a.example/x?p=1&q=2".to_owned(), Some("2026-11-01".to_owned())),
            ("https://a.example/y".to_owned(), None),
        ]
    );
    assert_eq!(parsed.sitemaps, Vec::<String>::new());
}

#[test]
fn a_sitemap_index_should_name_its_sitemaps() {
    let parsed = parse("<sitemapindex><sitemap><loc>https://a.example/sitemap-0.xml</loc></sitemap></sitemapindex>");
    assert_eq!(parsed.sitemaps, ["https://a.example/sitemap-0.xml"]);
    assert_eq!(parsed.urls, Vec::<(String, Option<String>)>::new());
}

/// The acceptance check: the two fixtures differ by one page added, one whose date moved, one
/// removed (not submitted) and two unchanged, one of them undated. The new sitemap is an index
/// whose sitemap sits beside it, as in a site's `dist/`.
#[test]
fn the_fixtures_should_differ_by_exactly_the_added_and_changed_pages() {
    let before = read_sitemap(&fixture("before.xml")).unwrap();
    let after = read_sitemap(&fixture("after/sitemap-index.xml")).unwrap();
    assert_eq!(
        changes(&before, &after),
        [
            ("https://docs.laminapdf.com/install/".to_owned(), Change::Changed),
            (
                "https://docs.laminapdf.com/recipes/edit-text/".to_owned(),
                Change::Added
            ),
        ]
    );
}

#[test]
fn a_snapshot_should_read_back_as_the_sitemap_it_recorded() {
    let dir = std::env::temp_dir().join(format!("xtask-indexnow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("before.json");
    run(&Task::Snapshot {
        sitemap: fixture("before.xml"),
        out: out.clone(),
    })
    .unwrap();
    let again = read_before(&out.to_string_lossy()).unwrap();
    assert_eq!(again, read_sitemap(&fixture("before.xml")).unwrap());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_request_should_carry_the_host_key_key_location_and_pages() {
    let body: serde_json::Value = serde_json::from_str(&request_body(
        "docs.laminapdf.com",
        KEY,
        &["https://docs.laminapdf.com/install/"],
    ))
    .unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "host": "docs.laminapdf.com",
            "key": KEY,
            "keyLocation": format!("https://docs.laminapdf.com/{KEY}.txt"),
            "urlList": ["https://docs.laminapdf.com/install/"],
        })
    );
}

#[test]
fn pages_on_two_hosts_should_be_refused() {
    let error = host_of(&["https://laminapdf.com/", "https://docs.laminapdf.com/"]).unwrap_err();
    assert!(error.to_string().contains("isn't on laminapdf.com"), "{error}");
    assert_eq!(
        host_of(&["https://laminapdf.com/a", "https://laminapdf.com/b"]).unwrap(),
        "laminapdf.com"
    );
}

/// Each site serves the same key at `/<key>.txt`, from its `public/` folder.
#[test]
fn both_sites_should_serve_the_key_file() {
    for site in ["apps/site/public", "apps/docs/public"] {
        let file = root().join(site).join(format!("{KEY}.txt"));
        assert_eq!(read_key(&file).unwrap(), KEY, "{site}");
    }
}

#[test]
fn a_key_file_should_hold_its_own_name() {
    let dir = std::env::temp_dir().join(format!("xtask-indexnow-key-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{KEY}.txt"));
    std::fs::write(&file, "another-key\n").unwrap();
    assert!(read_key(&file).is_err());
    let short = dir.join("abc.txt");
    std::fs::write(&short, "abc").unwrap();
    assert!(read_key(&short).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_arguments_should_name_each_step() {
    assert_eq!(
        parse_args(&strings(&[
            "snapshot",
            "https://laminapdf.com/sitemap.xml",
            "--out",
            "b.json"
        ]))
        .unwrap(),
        Task::Snapshot {
            sitemap: "https://laminapdf.com/sitemap.xml".to_owned(),
            out: PathBuf::from("b.json"),
        }
    );
    assert_eq!(
        parse_args(&strings(&[
            "submit",
            "--before",
            "b.json",
            "--after",
            "dist/sitemap.xml",
            "--key-file",
            "k.txt",
            "--dry-run"
        ]))
        .unwrap(),
        Task::Submit {
            before: "b.json".to_owned(),
            after: "dist/sitemap.xml".to_owned(),
            key_file: PathBuf::from("k.txt"),
            dry_run: true,
        }
    );
    assert!(parse_args(&strings(&["submit", "--before", "b.json"])).is_err());
    assert!(parse_args(&strings(&["snapshot", "--out", "b.json"])).is_err());
}
