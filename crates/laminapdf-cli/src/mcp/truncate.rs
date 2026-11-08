//! `--max-output-bytes`: a report cut down to fit, still valid against its schema.
//!
//! A cut removes whole items from the end of the report's arrays, at any depth (pages, page
//! sizes, warnings, a revision's objects), keeping one item in each at first, then shortens free text (a page's text, a title), then
//! drops the last items too. It never shortens a string whose value the schema fixes (`schema`,
//! a code, a kind), so the result still matches the tool's `outputSchema`, and it ends with
//! `"truncated": true`. Each round measures the report once and removes about as much as it is
//! over, so a large report takes a few passes, not one per item.

use serde_json::Value;

/// Keys whose strings are free text, which a cut may shorten.
const TEXT_KEYS: [&str; 9] = [
    "text", "title", "author", "subject", "keywords", "creator", "producer", "label", "page",
];

/// What `"truncated": true` adds to an object.
const MARK: usize = r#","truncated":true"#.len();

/// The serialised size of `value`, in bytes.
fn size(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |bytes| bytes.len())
}

/// `report` cut down to at most `max` bytes of JSON where it can be, and marked when it was.
pub(super) fn truncate(mut report: Value, max: usize) -> Value {
    if size(&report) <= max {
        return report;
    }
    let budget = max.saturating_sub(MARK);
    for keep_one in [true, false] {
        // Each round removes about the overflow; a few rounds suffice.
        for _ in 0..32 {
            let over = size(&report).saturating_sub(budget);
            if over == 0 || !(drop_items(&mut report, over, keep_one) || cut_text(&mut report, over)) {
                break;
            }
        }
    }
    if let Some(map) = report.as_object_mut() {
        map.insert("truncated".to_owned(), Value::Bool(true));
    }
    report
}

/// Arrays a cut leaves whole: `reveal`'s `found` names the categories with findings, and an
/// emptied one would read as "nothing found".
const KEEP_KEYS: [&str; 1] = ["found"];

/// The JSON pointer, total item size and length of every array in `value`, at any depth, but
/// those under [`KEEP_KEYS`].
fn arrays(value: &Value, path: &str, out: &mut Vec<(String, usize, usize)>) {
    match value {
        Value::Array(items) => {
            out.push((path.to_owned(), items.iter().map(size).sum(), items.len()));
            for (i, item) in items.iter().enumerate() {
                arrays(item, &format!("{path}/{i}"), out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map.iter().filter(|(k, _)| !KEEP_KEYS.contains(&k.as_str())) {
                let key = key.replace('~', "~0").replace('/', "~1");
                arrays(item, &format!("{path}/{key}"), out);
            }
        }
        _ => {}
    }
}

/// Drops items from the end of the report's largest array, wherever it sits, until about
/// `over` bytes are gone, keeping its first item when `keep_one`; whether it dropped any.
fn drop_items(report: &mut Value, over: usize, keep_one: bool) -> bool {
    let floor = usize::from(keep_one);
    let mut found = Vec::new();
    if let Some(map) = report.as_object() {
        // The report itself is an object; its own fields start the paths.
        for (key, item) in map.iter().filter(|(k, _)| !KEEP_KEYS.contains(&k.as_str())) {
            arrays(
                item,
                &format!("/{}", key.replace('~', "~0").replace('/', "~1")),
                &mut found,
            );
        }
    }
    let Some((path, _, _)) = found
        .into_iter()
        .filter(|(_, _, len)| *len > floor)
        .max_by_key(|(_, total, _)| *total)
    else {
        return false;
    };
    let Some(items) = report.pointer_mut(&path).and_then(Value::as_array_mut) else {
        return false;
    };
    let sizes: Vec<usize> = items.iter().map(size).collect();
    let mut removed = 0;
    let mut keep = sizes.len();
    while keep > floor && removed < over {
        keep -= 1;
        removed += sizes.get(keep).copied().unwrap_or(0) + 1;
    }
    items.truncate(keep);
    keep < sizes.len()
}

/// Shortens the longest free-text string by about `over` bytes; whether it shortened one.
fn cut_text(report: &mut Value, over: usize) -> bool {
    let Some(longest) = longest_text(report, None) else {
        return false;
    };
    if longest <= 1 {
        return false;
    }
    shorten(report, None, longest, longest.saturating_sub(over.max(1)))
}

/// The length of the longest free-text string in `value` (under `key`).
fn longest_text(value: &Value, key: Option<&str>) -> Option<usize> {
    match value {
        Value::String(text) if key.is_some_and(|k| TEXT_KEYS.contains(&k)) => Some(text.len()),
        Value::Object(map) => map.iter().filter_map(|(k, v)| longest_text(v, Some(k))).max(),
        Value::Array(items) => items.iter().filter_map(|v| longest_text(v, key)).max(),
        _ => None,
    }
}

/// Cuts the first free-text string of length `length` down to `keep` bytes (on a character
/// boundary); whether it found one.
fn shorten(value: &mut Value, key: Option<&str>, length: usize, keep: usize) -> bool {
    match value {
        Value::String(text) if text.len() == length && key.is_some_and(|k| TEXT_KEYS.contains(&k)) => {
            let mut cut = keep.min(text.len());
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
            true
        }
        Value::Object(map) => map.iter_mut().any(|(k, v)| shorten(v, Some(k), length, keep)),
        Value::Array(items) => items.iter_mut().any(|v| shorten(v, key, length, keep)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_report_over_the_limit_should_be_cut_and_marked() {
        let report = json!({ "schema": "laminapdf.text/1", "pages": [
            { "index": 0, "text": "a".repeat(400) }, { "index": 1, "text": "b".repeat(400) }, { "index": 2, "text": "c".repeat(400) }
        ], "warnings": [] });
        let cut = truncate(report.clone(), 300);
        assert!(size(&cut) <= 300, "{cut}");
        assert_eq!(
            (cut["truncated"].as_bool(), cut["schema"].as_str()),
            (Some(true), Some("laminapdf.text/1"))
        );
        assert_eq!(
            cut["pages"].as_array().map(Vec::len),
            Some(1),
            "the first page stays, its text cut"
        );
        assert_eq!(truncate(report.clone(), 10_000), report);
    }

    #[test]
    fn fixed_strings_should_never_be_cut() {
        let warning = json!({ "code": "xref-repaired", "kind": "input", "message": "m".repeat(50) });
        let report = json!({ "schema": "laminapdf.info/1", "warnings": [warning.clone(), warning] });
        let cut = truncate(report, 80);
        assert_eq!(cut["schema"], "laminapdf.info/1");
        for item in cut["warnings"].as_array().into_iter().flatten() {
            assert_eq!(
                (item["code"].as_str(), item["kind"].as_str()),
                (Some("xref-repaired"), Some("input"))
            );
        }
    }

    #[test]
    fn a_large_report_should_take_few_rounds() {
        let pages: Vec<Value> = (0..20_000)
            .map(|i| json!({ "index": i, "text": "x".repeat(100) }))
            .collect();
        let cut = truncate(json!({ "schema": "laminapdf.text/1", "pages": pages }), 10_000);
        assert!(size(&cut) <= 10_000);
    }

    #[test]
    fn nested_lists_should_be_cut_and_found_left_whole() {
        let added: Vec<u32> = (0..2000).collect();
        let report = json!({ "schema": "laminapdf.reveal/1", "found": ["history", "metadata"],
            "history": { "revisions": [{ "index": 0, "added": added }] }, "warnings": [] });
        let cut = truncate(report, 400);
        assert!(size(&cut) <= 400, "{cut}");
        assert_eq!(cut["found"], json!(["history", "metadata"]));
        assert!(
            cut["history"]["revisions"][0]["added"]
                .as_array()
                .is_some_and(|a| a.len() < 2000)
        );
    }
}
