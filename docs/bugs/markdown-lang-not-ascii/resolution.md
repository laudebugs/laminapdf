# Resolution: A language tag with non-ASCII text crashes text shaping

The shaper now passes HarfRust only language tags made of ASCII letters, digits and hyphens. The shaper ignores any other tag, as it ignores an unknown one, and the text shapes with the font's default language system. The bug is described in [detail.md](detail.md).

## The fix

`HarfRust::shape_range` (`crates/laminapdf-text/src/shape/engine.rs`) filters the tag with `is_tag_text` before it builds HarfRust's `Language`. The check sits at the one place where every caller's tag reaches HarfRust, so Markdown, the builder and the bindings are all covered.

We kept HarfRust at 0.8.0, which the workspace pins, because 0.15 changes its API.

## Tests

- `crates/laminapdf-text/tests/shaping_languages.rs` sweeps 159 tags: é, U+FFFD and 😀 at every position of `cm`, `zh-`, `zh-Hant`, `cmn`, `art-lojban`, `x-hbo`, `i-`, `ja`, `sr-Latn` and `en`. Each must shape exactly as it does with no language. Before the fix, 28 of them panicked.
- `fixtures/regressions/markdown/lang-tag-not-ascii`, the fuzzer's minimised input, replays through the fuzz target's entry point in `crates/laminapdf-build/tests/regressions.rs`. It failed with the same panic before the fix.
- The shaping goldens (`crates/laminapdf-text/tests/shaping_goldens.rs`) still match HarfBuzz with the tags `ar`, `he`, `hi`, `zh` and `en`, so well-formed tags still reach the shaper.

## Left for later

Markdown front matter still writes a malformed tag, such as `cm�`, to the document's `/Lang` unchanged. A `/Lang` that isn't a language tag fails PDF/UA-1, which the builder's profile checks don't test yet.

Whether newer HarfRust releases still slice tags by bytes is for the HarfRust upgrade to check; the filter stays either way.
