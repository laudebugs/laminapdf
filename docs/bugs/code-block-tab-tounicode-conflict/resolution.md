# Resolution: A tab in a code block raises `tounicode-conflict` with a CFF font

The flowing layout now replaces tabs before it shapes the text. In a code block a tab becomes spaces up to the next multiple of four columns, and anywhere else it becomes one space. What the page draws and the text it records now agree, so no font warns. The bug is described in [detail.md](detail.md).

## The fix

- `Items::code_block` (`crates/laminapdf-build/src/flow/items.rs`) runs each line through `expand_tabs`, which counts a column per grapheme cluster (`laminapdf_text::grapheme_boundaries`), so a letter and its combining marks take one column. Four columns is CommonMark's tab stop, and the one `expandTabs` used. Markdown and JSON documents both reach this function.
- `Source::push` (`crates/laminapdf-build/src/flow/measure.rs`) turns any other tab into a space: in a paragraph, a heading, a cell or inline code. A browser shows a tab in a paragraph as a space too.

A tab drawn through `Surface::text` directly keeps its old behaviour: it is drawn as a space, and a CFF font still marks it with `/ActualText` and warns. Callers that place text themselves choose what a tab means.

## Tests

In `crates/laminapdf-build/tests/markdown.rs`, with Noto Sans SC (CFF outlines):

- `a_tab_in_a_code_block_should_raise_no_tounicode_conflict_in_a_cff_font`: a fenced and an indented code block with tabs, one at the start of a line. Before the fix it raised `tounicode-conflict` for the tabs.
- `a_tab_in_a_code_block_should_extract_as_spaces_to_the_next_tab_stop`: `name\tqty`, `ab\t1` and `abcd\tx` extract as `name    qty`, `ab  1` and `abcd    x`. Before the fix they extracted with tabs.
- `a_tab_outside_a_code_block_should_be_a_space`: a tab in a paragraph and in inline code. Before the fix it raised five warnings.
- `a_tab_in_a_code_block_in_a_list_item_should_extract_as_spaces_to_the_next_tab_stop`: a list item holds its code block as paragraph text (`block_text` in `markdown/convert.rs`), so that text is expanded too.

In `crates/laminapdf-build/tests/document_schema.rs`, `a_tab_in_a_code_block_should_raise_no_tounicode_conflict_in_a_cff_font` builds a JSON document's `codeBlock` and paragraph with tabs.

The unit tests in `items.rs` check the tab stops, a combining mark and a line without tabs.

## The workaround removed

`expandTabs`, which #461 added to `apps/docs/plugins/guide.mjs` to expand tabs before the build, is gone with its test in `guide.test.mjs`. The guide PDF now passes the extraction guide's tab-separated example to the builder as written and builds without a warning.
