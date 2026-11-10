# Resolution: shape only language tags made of tag characters

The shaper now passes HarfRust only language tags made of ASCII letters, digits and hyphens. Any other tag is ignored, as an unknown tag is: the text shapes with the font's default language system. The bug is described in [detail.md](detail.md).

## The fix

`HarfRust::shape_range` (`crates/laminapdf-text/src/shape/engine.rs`) filters the tag with `is_tag_text` before it builds HarfRust's `Language`. The check sits at the one place where every caller's tag reaches HarfRust, so Markdown, the builder and the bindings are all covered.

Upgrading HarfRust wasn't the fix. The workspace pins 0.8.0, and 0.15 changes its API; whether a newer release still slices tags this way is a question for that upgrade.

## Tests

- `crates/laminapdf-text/tests/shaping_languages.rs` sweeps 159 tags: é, U+FFFD and 😀 at every position of `cm`, `zh-`, `zh-Hant`, `cmn`, `art-lojban`, `x-hbo`, `i-`, `ja`, `sr-Latn` and `en`. Each must shape exactly as with no language. The old rule panicked on 28 of them.
- `fixtures/regressions/markdown/lang-tag-not-ascii`, the fuzzer's minimised input, replays through the fuzz target's entry point in `crates/laminapdf-build/tests/regressions.rs`. It failed with the same panic before the fix.
- The shaping goldens (`crates/laminapdf-text/tests/shaping_goldens.rs`) still match HarfBuzz with the tags `ar`, `he`, `hi`, `zh` and `en`, so well-formed tags still reach the shaper.

## Left for later

Markdown front matter still writes such a tag to the document's `/Lang` unchanged. A `/Lang` that isn't a language tag fails PDF/UA-1, which the builder's profile checks don't test yet.
