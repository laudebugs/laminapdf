//! `cargo xtask gen licence [--check]`: the licence the macOS disk image shows before it opens,
//! `apps/desktop/src-tauri/licence.rtf`, from `LICENSE-MIT`.
//!
//! macOS shows a plain-text licence in its serif face and keeps the file's 72-column line
//! breaks, so lines end mid-sentence. The RTF holds the same words, each paragraph on one line,
//! in the system's sans-serif face. `tauri.macos.conf.json` points the bundle at it; the Windows
//! installers keep the text file. `--check` fails when the committed RTF isn't what
//! `LICENSE-MIT` gives, so the two never say different things.

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result, bail};

use crate::step;
use crate::workspace::Workspace;

/// The licence the RTF is made from.
const SOURCE: &str = "LICENSE-MIT";
/// The RTF the disk image shows.
const TARGET: &str = "apps/desktop/src-tauri/licence.rtf";

/// `text`'s paragraphs, each with its lines joined by spaces.
fn paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect()
}

/// `text` with RTF's three special characters escaped and anything past ASCII as `\u`.
fn escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_ascii() => out.push(c),
            c => {
                for unit in c.encode_utf16(&mut [0; 2]) {
                    // RTF writes a UTF-16 unit as a signed 16-bit number, then a fallback character.
                    let _ = write!(out, "\\u{}?", unit.cast_signed());
                }
            }
        }
    }
    out
}

/// The RTF for `licence`: Helvetica Neue at 12 pt, a blank line between paragraphs, the first
/// (the licence's name) bold.
pub(crate) fn render(licence: &str) -> String {
    let mut out = String::from(
        "{\\rtf1\\ansi\\ansicpg1252\\deff0\n{\\fonttbl{\\f0\\fswiss\\fcharset0 Helvetica Neue;}}\n\\f0\\fs24\n",
    );
    for (i, p) in paragraphs(licence).iter().enumerate() {
        if i == 0 {
            let _ = writeln!(out, "{{\\b {}}}\\par\\par", escape(p));
        } else {
            let _ = writeln!(out, "{}\\par\\par", escape(p));
        }
    }
    out.push_str("}\n");
    out
}

/// Writes the RTF, or with `check` fails when the committed one differs.
pub(crate) fn generate(ws: &Workspace, check: bool) -> Result<()> {
    step("licence: the macOS disk image's licence.rtf matches LICENSE-MIT");
    let licence = fs::read_to_string(ws.root.join(SOURCE)).with_context(|| format!("reading {SOURCE}"))?;
    let rtf = render(&licence);
    let path = ws.root.join(TARGET);
    if check {
        let committed = fs::read_to_string(&path).unwrap_or_default();
        if committed != rtf {
            bail!("{TARGET} isn't what {SOURCE} gives: run `cargo xtask gen licence`");
        }
        return Ok(());
    }
    fs::write(&path, rtf).with_context(|| format!("writing {TARGET}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_join_their_lines_and_rtf_escapes_its_specials() {
        let rtf = render("MIT License\n\nOne line\nand the next.\n\nA {brace} and \\ é\n");
        assert_eq!(
            rtf.lines().skip(3).collect::<Vec<_>>(),
            [
                "{\\b MIT License}\\par\\par",
                "One line and the next.\\par\\par",
                "A \\{brace\\} and \\\\ \\u233?\\par\\par",
                "}"
            ]
        );
    }
}
