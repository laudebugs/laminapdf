---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Rust callers can now lay out a paragraph whose words change font, size or language: a bold word inside a sentence, a superscript, an Arabic name in an English line. `laminapdf_text::break_spans` takes the text and a list of `StyledSpan`s, each covering a stretch of it with its own fonts, size, baseline rise, language and OpenType features. Lines still break only where the text allows, so a style change inside a word never splits it. Spans of one style are shaped together, so kerning carries across them. Each line reports its ascent and descent from its tallest run, and `pdf_line_run` draws a superscript's rise. `break_lines` is unchanged. Spans that don't cover the text once, in order, that split a letter from its accent, or that name no font or a font past the list fail with the new error code `invalid-spans`.
