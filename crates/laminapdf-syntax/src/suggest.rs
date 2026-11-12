//! "Did you mean" candidates for a misspelt name: the `didYouMean` of ADR 0026 §1, from Lamina
//! PDF's own vocabulary (option names, op names, codes), never from the document.

/// The known names within two edits of `given`, closest first; names at the same distance in
/// code-point order. Edits count chars, after lowercasing ASCII letters only
/// (`fixtures/did-you-mean.json` pins the rule for both languages).
///
/// Added in 0.2.0.
pub fn did_you_mean(given: &str, known: &[String]) -> Vec<String> {
    let given = given.to_ascii_lowercase();
    let mut close: Vec<(usize, &String)> = known
        .iter()
        .map(|k| (distance(&given, &k.to_ascii_lowercase()), k))
        .filter(|(d, _)| *d <= 2)
        .collect();
    close.sort_unstable();
    close.into_iter().map(|(_, k)| k.clone()).collect()
}

/// Levenshtein distance over chars.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = i;
        let mut next = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let above = row.get(j + 1).copied().unwrap_or(usize::MAX);
            let left = next.last().copied().unwrap_or(usize::MAX);
            let cost = diagonal + usize::from(ca != *cb);
            next.push(cost.min(above.saturating_add(1)).min(left.saturating_add(1)));
            diagonal = above;
        }
        row = next;
    }
    row.last().copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::did_you_mean;
    use serde_json::Value;

    /// A string list of a case.
    fn strings(value: &Value) -> Vec<String> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect()
    }

    /// Every case of `fixtures/did-you-mean.json`, which `@laminapdf/sdk`'s
    /// `test/did-you-mean.test.ts` runs too, so both languages suggest the same names. Every
    /// failing case is listed, as Vitest's `it.each` lists them.
    #[test]
    fn did_you_mean_gives_the_shared_fixture_s_names() {
        let fixture = laminapdf_test_fixtures::json("did-you-mean.json");
        let failing: Vec<(String, Vec<String>, Vec<String>)> = fixture["cases"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|case| {
                let about = case["about"].as_str().unwrap_or_default().to_owned();
                let given = case["given"].as_str().unwrap_or_default();
                (
                    about,
                    did_you_mean(given, &strings(&case["known"])),
                    strings(&case["expected"]),
                )
            })
            .filter(|(_, suggested, expected)| suggested != expected)
            .collect();
        assert!(!fixture["cases"].as_array().is_none_or(Vec::is_empty));
        assert!(failing.is_empty(), "(about, suggested, expected): {failing:#?}");
    }
}
