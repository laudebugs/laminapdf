# Resolution: images and copied pages added to a PDF/A file ignored its output intent

[Diagnosis](detail.md).

## The fix

The save planner now warns `pdfa-color-space` when what the edits added paints in a device space the output intent doesn't allow. The warning names the pages whose added objects do so (`pages`) and the annotations whose new appearances do (`annotations`), such as image stamps. Like every PDF/A problem an edit introduces, it is a warning at save and never a blocked edit (E6 plan §3, [#749](https://github.com/laudebugs/laminapdf/issues/749)).

The rule lives in `laminapdf-ops`' `metadata/color_space/`. `intent` reads the first `GTS_PDFA1` intent's kind from its profile's header (the colour space field, bytes 16–19), else from its `/N`; `annot::add::intent_space` shares it. `Rules::of` turns the kind and the PDF/A part into the device spaces allowed. `fits` (in `scan.rs`) reads a form's content with `laminapdf-content`'s tokenizer (`ContentIter`), tracks the fill and stroke colours and the text render mode across `q` and `Q`, and judges what it paints with:

- the fill and stroke colours of each painted path and of shown text, by its render mode, whether set with `g`, `rg`, `k` or `cs` and a named space;
- each image: an image XObject's `/ColorSpace`, an inline image's `/CS` (a named one through the resources), a stencil mask's fill colour;
- each shading (`sh`), and each pattern a colour names: a shading pattern's space, and the content of a coloured tiling pattern, once each, two patterns deep;
- each form's transparency group space (`/Group /CS`), and the forms it draws (`Do`), four deep, each once.

A first version ran the content through `laminapdf-content`'s interpreter. It linked the interpreter's text, font and graphics-state machinery into the base module, which hadn't carried it, for 16.9 KB gzipped; the check needs only the colour operators, `cs`/`CS`, `Do`, `sh`, inline images and `Tr`. The tokenizer was already in every module (`added::place`), and the state machine on it costs 6.3 KB, measured with twiggy on named builds of the merge base and the branch.

`Indexed`, `Pattern`, `Separation` and `DeviceN` spaces are judged by their base or alternate, and a family name or space may be indirect, as `laminapdf-content` reads them. `added::pdfa_pages` runs the check on every added form of an edited page. `annot::conformance` runs it on the normal appearance of an overlay annotation whose `/AP /N` the edits wrote, so an appearance the file brought is never blamed on the edit.

## Judgement calls

- **Only what is painted counts.** A colour space a resource dictionary names but nothing paints with never warns, nor does a colour set and never used.
- **`Default` spaces.** ISO 32000-2 §8.6.5.6 looks a `Default` space up in the current resource dictionary when a device space is selected, and ISO 19005-2 §6.2.4.3 allows a device space that one replaces. The check accepts one in the current dictionary or any enclosing one, the page's included for an object placed on it. Reading the spec strictly, only the current dictionary counts. The lenient reading was chosen because a warning that cries wolf costs more than a rare miss.
- **PDF/A-1.** ISO 19005-1 §6.2.3.3 allows `DeviceRGB` and `DeviceCMYK` only through a matching output intent and names no `Default` exception, so on PDF/A-1 a `Default` space doesn't stand in.
- **The intent's kind.** The profile header outranks `/N`, so a Lab profile with `/N 3` allows only grey, not RGB. An intent with neither a readable header nor `/N` is unknown, and nothing is judged: the warning would rest on a guess. A file with no `GTS_PDFA1` intent allows no device space.
- **Lamina PDF's own drawing.** Shapes and appearances paint in the intent's space when it is RGB or CMYK. In a file without an intent, or with a grey or Lab one, they paint `DeviceRGB` and warn, which is what such a file's claim asks.

## Why warn and not convert

Converting the image when it is written was the alternative. It loses on four counts:

- **The convention.** Every other PDF/A rule an edit can break warns at save and lets the user decide, as the owner confirmed for transparency in #749. A user may mean to drop the claim (`removeConformanceClaims`).
- **Colour management.** A faithful CMYK-to-RGB conversion needs ICC transforms and a CMYK profile to read the image with. The core has no colour engine, and no CMYK profile under a licence we can ship. A naive formula changes the photo's colours without telling anyone.
- **Quality.** A JPEG is written as it is (`DCTDecode` pass-through). Converting it means decoding and re-encoding it, which loses detail on every edit.
- **Copied pages.** Their content streams paint with `rg` and `k` operators, which conversion would have to rewrite.

Tagging the image `ICCBased` with the intent's own profile is also wrong: it makes a CMYK image read as RGB samples.

## How it was checked

- `crates/laminapdf-ops/tests/pdfa_color_space.rs` sweeps the intent (none, grey, RGB, CMYK) against the image (grey, RGB and CMYK JPEGs; an ICC image only through a copied page, since the image writer never writes one) for each path, in three tests: `an_image_placed_as_content_warns_with_its_page_when_the_intent_doesnt_fit`, `an_image_stamp_warns_with_its_id_when_the_intent_doesnt_fit` and `a_copied_page_warns_with_its_page_when_its_images_dont_fit_the_intent`. Each case warns exactly when the rule says, naming the page or the stamp.
- The same file's other tests cover a copied page's content: `a_copied_pages_operators_count_only_where_they_paint` (`rg` on an RGB intent passes, `k`, `K` and `cs` with `DeviceCMYK` warn, a colour set but not painted with doesn't), `a_copied_pages_text_counts_by_its_render_mode`, `a_copied_pages_inline_image_counts_by_its_space`, `a_copied_pages_named_space_counts_when_painted_with_and_by_its_base` (`Indexed`, `Separation`, `DeviceN`, an unused name), `a_copied_pages_patterns_and_shadings_count` (tiling and shading patterns, `sh`), `a_nested_forms_paint_and_group_space_count`, `an_image_mask_takes_the_fill_colour_and_a_jpx_image_its_own_file`, `a_default_space_stands_in_for_its_device_space_on_pdf_a_2_but_not_pdf_a_1`, `the_intents_kind_comes_from_its_profiles_header_then_its_n` and `undoing_the_edit_takes_the_warning_away`.
- All 13 fail when the scan is given empty content instead of the form's. The first version of the fix (five tests) failed in full with the check stubbed to pass everything, as the code before the fix behaved.
- `crates/laminapdf-save/tests/stamps.rs`' `a_cmyk_image_stamp_breaks_the_pdfa_examples_rgb_intent_and_an_rgb_one_doesnt` reproduces the report on the PDF/A-2b example through the save planner.
- `metadata/color_space/`'s unit tests check device, ICC, `Indexed`, `Separation`, `DeviceN` and indirect spaces against each intent.
- The façade's `data_fields` test covers the new code's data keys. `tests/markup.rs`' PDF/A tests leave `pdfa-color-space` out: their fixture has no output intent, so every appearance now warns.

## Left for later

- Type 3 glyphs' colours, and the luminosity groups of soft masks (`/SMask` in an ExtGState), aren't read.
- Content edits that replace an image on a page (E7) write the new image the same way and aren't checked yet.
- An ICC profile's own requirements (ISO 19005-2 §6.2.4.2) aren't checked: any `ICCBased` space counts as allowed.
