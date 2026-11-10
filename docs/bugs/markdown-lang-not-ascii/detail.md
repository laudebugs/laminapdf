# A language tag with non-ASCII text crashes text shaping

Markdown whose front matter set `lang` to text with a non-ASCII character crashed Lamina PDF: HarfRust, the text shaper, panicked while it looked the tag up. The nightly fuzz run found it on 2026-11-10. The fix is in [resolution.md](resolution.md).

## What the fuzzer found

The nightly lane's `markdown` fuzz target (run 42805606993, commit 83a11137) failed with a panic inside HarfRust 0.8.0:

```text
panicked at harfrust-0.8.0/src/hb/tag.rs:167:7:
end byte index 3 is not a char boundary; it is inside '�' (bytes 1..4 of string)
```

The backtrace runs from `Builder::markdown` through the flowing layout (`flow`, `paginate`, `break_spans`) into `HarfRust::shape_range`, then into HarfRust's `tags_from_language` and its `strncmp`.

## How to reproduce it

The minimised input is 16 bytes, kept as `fixtures/regressions/markdown/lang-tag-not-ascii`:

```text
---
lang:cm\x86
---
```

The byte `0x86` isn't valid UTF-8, so the Markdown reader turns it into U+FFFD (`�`), three bytes in UTF-8. The language tag becomes `cm�`, and laying out any text with it panics. Any program can reach the same panic without Markdown, by passing a tag such as `cm�` to `ShapeOptions::with_language` or `LayoutOptions::with_language`.

## The cause

HarfRust 0.8's `strncmp` (`src/hb/tag.rs`) compares the first `n` bytes of two tags by slicing each `&str` at byte `n`. When byte `n` falls inside a multi-byte character, the slice panics. For `cm�` and the three-byte prefix `cmn` in HarfRust's language table, the cut at byte 3 lands inside `�`.

Lamina PDF passed every non-empty tag through to HarfRust, although a BCP 47 tag can only contain ASCII letters, digits and hyphens (RFC 5646 §2.1). A tag with any other character can't name a language. Every caller of the shaper passed tags the same way: Markdown front matter, the builder's text options and the WASM and TypeScript bindings.
