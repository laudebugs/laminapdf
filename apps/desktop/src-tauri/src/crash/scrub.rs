//! What keeps a crash report anonymous: [`scrub`] takes out of free text anything that could
//! name a file, a folder or a person, and [`script_frames`] turns a webview's error stack into
//! structured frames whose `file` is a bundle asset or `<external>`, never a path. The update
//! Worker refuses a report where either is left (apps/updates/src/crash.ts checks the same things),
//! so a bug here fails loudly instead of leaking.

use std::borrow::Cow;
use std::sync::LazyLock;

use laminapdf_desktop_protocol::crash::{MAX_FRAMES, truncate_on_char_boundary};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// What a removed path becomes.
pub(crate) const PATH: &str = "<path>";
/// What a removed document name becomes.
pub(crate) const FILE: &str = "<file>";
/// What a removed e-mail address becomes.
pub(crate) const EMAIL: &str = "<email>";
/// What a removed long number becomes.
pub(crate) const NUMBER: &str = "<number>";
/// A script frame's file outside the app's bundle.
pub(crate) const EXTERNAL: &str = "<external>";

/// A bundle asset, as Vite names its hashed output: the one `file` a script frame keeps. The schema
/// (apps/updates/schemas/crash-report.schema.json) and the Worker (`SCRIPT_FILE` in
/// apps/updates/src/crash.ts) hold frames to the same pattern; tests on both sides pin it.
pub(crate) const ASSET_PATTERN: &str = r"^assets/[A-Za-z0-9._-]+\.(js|mjs)$";

/// The origins the app's own pages load from: `tauri://localhost` on macOS and Linux,
/// `http(s)://tauri.localhost` on Windows.
const ORIGINS: [&str; 3] = [
    "tauri://localhost/",
    "http://tauri.localhost/",
    "https://tauri.localhost/",
];

/// The most bytes a frame's function name keeps.
pub(crate) const MAX_FUNCTION: usize = 256;

/// What [`scrub`] removes, in order, and what each becomes. A path is any run of characters with
/// two separators in it, so relative paths (`src/a/b.rs`) and URLs go too; since a folder or a file
/// name may hold spaces, so may a path, which runs to the next quote or the end of the line. That
/// takes out some text that isn't a path ("1/2 or 3/4"), which is the safe side.
static RULES: LazyLock<[(Regex, &str); 8]> = LazyLock::new(|| {
    let rule = |pattern: &str, with| (Regex::new(pattern).unwrap_or_else(|_| unreachable!()), with);
    [
        rule(r#"(?i)file:[^"'<>\n]*"#, PATH),
        rule(r#"\\\\[^\s\\"'<>]+\\[^"'<>\n]*"#, PATH),
        rule(r#"(?i)\b[a-z]:[\\/][^"'<>\n]*"#, PATH),
        rule(r#"~[\\/][^"'<>\n]*"#, PATH),
        rule(r#"[^\s"'<>]*[/\\][^/\\"'<>\n]+[/\\][^"'<>\n]*"#, PATH),
        rule(r#"(?i)[^\s"'<>]+\.(pdf|fdf|xfdf)\b"#, FILE),
        rule(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}", EMAIL),
        rule(r"\b\d{6,}\b", NUMBER),
    ]
});

static ASSET: LazyLock<Regex> = LazyLock::new(|| Regex::new(ASSET_PATTERN).unwrap_or_else(|_| unreachable!()));

/// `text` without paths, `file:` URLs, e-mail addresses or long numbers; borrowed when it had none.
pub(crate) fn scrub(text: &str) -> Cow<'_, str> {
    let mut out = Cow::Borrowed(text);
    for (pattern, with) in RULES.iter() {
        if let Cow::Owned(changed) = pattern.replace_all(&out, *with) {
            out = Cow::Owned(changed);
        }
    }
    out
}

/// One frame of a webview's stack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ScriptFrame {
    /// A bundle asset (`assets/index-3f2a.js`), or [`EXTERNAL`].
    pub file: String,
    pub line: u32,
    pub column: u32,
    /// The function, scrubbed; `None` for anonymous code.
    pub function: Option<String>,
}

/// The frames of `stack`, an `Error.stack` value in `WebKit`'s (`fn@url:1:2`) or V8's (`at fn (url:1:2)`)
/// form. Lines that aren't frames (V8's first line, the message) are skipped.
pub(crate) fn script_frames(stack: &str) -> Vec<ScriptFrame> {
    stack.lines().filter_map(script_frame).take(MAX_FRAMES).collect()
}

fn script_frame(line: &str) -> Option<ScriptFrame> {
    let line = line.trim();
    let (function, place) = if let Some(rest) = line.strip_prefix("at ") {
        match rest.strip_suffix(')').and_then(|rest| rest.split_once(" (")) {
            Some((function, place)) => (Some(function), place),
            None => (None, rest),
        }
    } else {
        let (function, place) = line.split_once('@')?;
        ((!function.is_empty()).then_some(function), place)
    };
    let (rest, column) = place.rsplit_once(':')?;
    let (url, line) = rest.rsplit_once(':')?;
    // A frame names a script by its URL. Without one, the line is the message (V8 puts it first,
    // and it can end like a frame), which a report never holds.
    if !url.contains("://") || url.contains(char::is_whitespace) {
        return None;
    }
    Some(ScriptFrame {
        file: asset(url),
        line: line.parse().ok()?,
        column: column.parse().ok()?,
        function: function.map(|name| truncate_on_char_boundary(scrub(name).into_owned(), MAX_FUNCTION)),
    })
}

/// `url`'s file within the app's bundle, or [`EXTERNAL`].
fn asset(url: &str) -> String {
    ORIGINS
        .iter()
        .find_map(|origin| url.strip_prefix(origin))
        .filter(|file| ASSET.is_match(file))
        .map_or_else(|| EXTERNAL.to_owned(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A JSON file of the update Worker's tests, which both sides read.
    fn shared(name: &str) -> serde_json::Value {
        let path = format!("{}/../../updates/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn the_scrubber_should_make_what_the_worker_expects_of_each_shared_case() {
        let cases = shared("test/fixtures/scrub-cases.json");
        let wrong: Vec<(String, String)> = cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|case| (case["input"].as_str().unwrap(), case["scrubbed"].as_str().unwrap()))
            .filter(|(input, scrubbed)| scrub(input) != *scrubbed)
            .map(|(input, _)| (input.to_owned(), scrub(input).into_owned()))
            .collect();
        assert_eq!(wrong, Vec::new());
    }

    #[test]
    fn the_asset_pattern_should_be_the_schemas() {
        let schema = shared("schemas/crash-report.schema.json");
        assert_eq!(
            schema["$defs"]["script_frame"]["properties"]["file"]["oneOf"][0]["pattern"],
            ASSET_PATTERN
        );
    }

    #[test]
    fn unix_paths_should_be_removed() {
        assert_eq!(
            scrub("cannot open /Users/ana/Tax 2026/return.pdf now"),
            "cannot open <path>"
        );
    }

    #[test]
    fn a_relative_path_should_be_removed() {
        assert_eq!(scrub("at src/engine/worker.rs:12"), "at <path>");
    }

    #[test]
    fn windows_drive_paths_should_be_removed() {
        assert_eq!(scrub(r"open C:\Users\ana\a.pdf failed"), "open <path>");
    }

    #[test]
    fn unc_paths_should_be_removed() {
        assert_eq!(scrub(r"read \\server\share\a.pdf"), "read <path>");
    }

    #[test]
    fn a_quoted_path_should_end_at_its_quote() {
        assert_eq!(scrub(r#"open "/a b/c.pdf": gone"#), r#"open "<path>": gone"#);
    }

    #[test]
    fn a_document_name_alone_should_be_removed() {
        assert_eq!(scrub("bad xref in Return 2026.PDF"), "bad xref in Return <file>");
    }

    #[test]
    fn a_home_path_should_be_removed() {
        assert_eq!(scrub("in ~/Documents"), "in <path>");
    }

    #[test]
    fn file_urls_should_be_removed() {
        assert_eq!(scrub("load file:///tmp/a.pdf"), "load <path>");
    }

    #[test]
    fn email_addresses_should_be_removed() {
        assert_eq!(scrub("from ana.b+pdf@example.co.uk"), "from <email>");
    }

    #[test]
    fn long_numbers_should_be_removed_but_hex_codes_kept() {
        assert_eq!(
            scrub("call 5551234567; crash: exit code 0xc0000005"),
            "call <number>; crash: exit code 0xc0000005"
        );
    }

    #[test]
    fn plain_text_should_be_borrowed_unchanged() {
        assert!(matches!(
            scrub("index out of bounds: the len is 3 but the index is 7"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn a_rust_function_name_should_survive() {
        assert_eq!(
            scrub("<laminapdf_desktop::engine::Engines as core::ops::Drop>::drop"),
            "<laminapdf_desktop::engine::Engines as core::ops::Drop>::drop"
        );
    }

    #[test]
    fn a_webkit_stack_should_keep_its_assets() {
        assert_eq!(
            script_frames(
                "render@tauri://localhost/assets/index-abc.js:1:234\n@tauri://localhost/assets/vendor-9f.js:5:6"
            ),
            [
                ScriptFrame {
                    file: "assets/index-abc.js".into(),
                    line: 1,
                    column: 234,
                    function: Some("render".into()),
                },
                ScriptFrame {
                    file: "assets/vendor-9f.js".into(),
                    line: 5,
                    column: 6,
                    function: None,
                },
            ]
        );
    }

    #[test]
    fn a_v8_stack_should_keep_its_assets_and_skip_the_message() {
        let stack = "TypeError: x is undefined\n    at Viewer.draw (http://tauri.localhost/assets/index-abc.js:3:9)\n    at http://tauri.localhost/assets/index-abc.js:4:1";
        let frames: Vec<_> = script_frames(stack).into_iter().map(|f| (f.file, f.function)).collect();
        assert_eq!(
            frames,
            [
                ("assets/index-abc.js".into(), Some("Viewer.draw".into())),
                ("assets/index-abc.js".into(), None),
            ]
        );
    }

    #[test]
    fn a_message_that_ends_like_a_frame_should_not_be_a_frame() {
        assert_eq!(script_frames("Error: mail ana@x.com:1:2"), []);
    }

    #[test]
    fn a_frame_outside_the_bundle_should_be_external() {
        let frames = script_frames("f@file:///Users/x/secret/a.js:1:2\ng@https://example.com/a.js:1:2");
        assert!(frames.iter().all(|frame| frame.file == EXTERNAL) && frames.len() == 2);
    }

    #[test]
    fn a_frame_climbing_out_of_the_bundle_should_be_external() {
        assert_eq!(
            script_frames("f@tauri://localhost/assets/../../etc/a.js:1:2")[0].file,
            EXTERNAL
        );
    }
}
