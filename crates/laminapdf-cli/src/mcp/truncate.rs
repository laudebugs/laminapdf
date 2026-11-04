//! `--max-output-bytes`: a report cut down to fit, still valid against its schema.
//!
//! A cut removes whole items from the end of the report's arrays (pages, page sizes, warnings),
//! keeping one item in each at first, then shortens free text (a page's text, a title), then
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

/// Drops items from the end of the report's largest array until about `over` bytes are gone,
/// keeping its first item when `keep_one`; whether it dropped any.
fn drop_items(report: &mut Value, over: usize, keep_one: bool) -> bool {
    let floor = usize::from(keep_one);
    let Some(map) = report.as_object_mut() else {
        return false;
    };
    let largest = map
        .values_mut()
        .filter_map(Value::as_array_mut)
        .filter(|items| items.len() > floor)
        .max_by_key(|items| items.iter().map(size).sum::<usize>());
    let Some(items) = largest else {
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
}
