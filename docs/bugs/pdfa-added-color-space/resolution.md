# Resolution: images and copied pages added to a PDF/A file ignored its output intent

[Diagnosis](detail.md).

## The fix

The save planner now warns `pdfa-color-space` when what the edits added paints in a device space the output intent doesn't allow. The warning names the pages whose added objects do so (`pages`) and the annotations whose new appearances do (`annotations`), such as image stamps. Like every PDF/A problem an edit introduces, it is a warning at save and never a blocked edit (E6 plan §3, [#749](https://github.com/laudebugs/laminapdf/issues/749)).

The rule lives in `laminapdf-ops`' `metadata/color_space.rs`. `intent_components` reads the first `GTS_PDFA1` intent's profile `/N`, which `annot::add::intent_space` now shares, and `DeviceSpaces::of_intent` turns it into the device spaces allowed. `colors_fit` then walks an XObject's resources without parsing a content stream. It checks an image's `/ColorSpace` and a form's named colour spaces, its shadings' spaces and its transparency group's `/CS`. It also walks the form's XObjects down to `added::MAX_FORM_DEPTH`, the depth the font check uses. `Indexed`, `Pattern`, `Separation` and `DeviceN` spaces are judged by their base or alternate, and a form's `DefaultGray`, `DefaultRGB` or `DefaultCMYK` space allows the device space it replaces. `added::pdfa_pages` runs the check on every added form of an edited page. `annot::conformance` runs it on the normal appearance of an overlay annotation whose `/AP /N` the edits wrote, so an appearance the file brought is never blamed on the edit.

## Why warn and not convert

Converting the image when it is written was the alternative, and it loses on four counts:

- **The convention.** Every other PDF/A rule an edit can break warns at save and lets the user decide, as the owner confirmed for transparency in #749. A user may mean to drop the claim (`removeConformanceClaims`).
- **Colour management.** A faithful CMYK-to-RGB conversion needs ICC transforms and a CMYK profile to read the image with. The core has no colour engine, and no CMYK profile under a licence we can ship. A naive formula changes the photo's colours without telling anyone.
- **Quality.** A JPEG is written as it is (`DCTDecode` pass-through). Converting it means decoding and re-encoding it, which loses detail on every edit.
- **Copied pages.** Their content streams paint with `rg` and `k` operators, which conversion would have to rewrite, and the one-stream grammar of added content never parses them.

Tagging the image `ICCBased` with the intent's own profile is also wrong: it makes a CMYK image read as RGB samples.

## How it was checked

- `crates/laminapdf-ops/tests/pdfa_color_space.rs` sweeps the intent (none, RGB, CMYK) against the image (gray, RGB, CMYK JPEGs; an ICC image only through a copied page, since the image writer never writes one) for each path: an image placed as content, an image stamp and a copied page. Each case warns exactly when the rule says, naming the page or the stamp. Two more tests cover a copied page's named `Indexed` space, its `DefaultCMYK` space and an undo that takes the warning away. With `colors_fit` stubbed to always pass, as the code was before, all five fail.
- `crates/laminapdf-save/tests/stamps.rs`' `a_cmyk_image_stamp_breaks_the_pdfa_examples_rgb_intent_and_an_rgb_one_doesnt` reproduces the report on the PDF/A-2b example through the save planner.
- `metadata/color_space.rs`' unit tests check device, ICC, `Indexed` and `Separation` spaces against each intent.
- The façade's `data_fields` test covers the new code's data keys.

## Left for later

- Content streams aren't read, so a copied page that paints `DeviceRGB` with an `rg` operator, or an inline image, isn't caught. Only its resources are.
- Content edits that replace an image on a page (E7) write the new image the same way and aren't checked yet.
- An ICC profile's own requirements (ISO 19005-2 §6.2.4.2) aren't checked: any `ICCBased` space counts as allowed.
