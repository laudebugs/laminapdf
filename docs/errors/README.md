# Error and warning codes

One page per code in [`fixtures/codes.json`](../../fixtures/codes.json), as [ADR 0026](../decisions/0026-error-and-diagnostic-contract.md) §6 requires. Every `docsUrl` points here, `laminapdf::explain(code)` (the façade's `explain` feature) and `@laminapdf/sdk/explain` return these pages, and `laminapdf explain <code>` ([L4a](../todos/done/L4a-lamina-cli.md)) will print them.

A PR that adds a code adds:

- its entry in `codes.json` (`code`, `kind`, `summary`, `hint`, `since`; the file's schema is [`fixtures/codes.schema.json`](../../fixtures/codes.schema.json)): the summary is one lowercase sentence saying what happened, the hint one sentence saying what to do, neither with document text nor a trailing period (ADR 0026 §4, §5);
- the arm in its type's `code()` and `kind()` (the core crates' `kind()` is an exhaustive match, checked against `codes.json` by that crate's tests);
- its page here, with the headings *What happened*, *Why*, *How to fix*, *Example* and *Data fields* and a `- Kind:` line matching `codes.json`; *Data fields* lists the `data` keys, marking the ones that can carry document or caller text *untrusted*. The page is written for readers of the docs site: ADR numbers, todo IDs and CI go in an optional last section, *For contributors*, which the site leaves out (`apps/docs/plugins/error-pages.mjs`) and `cargo xtask check docs-prose` skips;
- the output of `cargo xtask gen codes`, which regenerates the façade's `ErrorCode`/`WarningCode`, `@laminapdf/sdk`'s unions, tables and explain text, the `laminapdf.error/1` schema and the tables below. `cargo xtask check docs` fails, naming the file, when any of them is stale.

`crates/laminapdf-cos/tests/codes.rs` checks the entries and pages, and that every page here is for a listed code.

<!-- @generated below by cargo xtask gen codes. DO NOT EDIT. -->

## Warnings

| Code | Kind | Summary |
| --- | --- | --- |
| [`missing-header`](missing-header.md) | `input` | no %PDF-x.y header in the first 1,024 bytes; parsing went ahead |
| [`xref-repaired`](xref-repaired.md) | `input` | the stored cross-reference data was unusable and was rebuilt by scanning |
| [`xrefstm-ignored`](xrefstm-ignored.md) | `input` | a hybrid file's /XRefStm could not be read and was ignored |
| [`stream-length-recovered`](stream-length-recovered.md) | `input` | a stream's /Length was wrong, so its end was found by scanning for endstream |
| [`stale-generation`](stale-generation.md) | `input` | a reference's generation did not match its xref entry, so it resolved to null |
| [`page-tree-node-skipped`](page-tree-node-skipped.md) | `input` | a page-tree node was missing, malformed or too deep, and was skipped |
| [`page-count-mismatch`](page-count-mismatch.md) | `input` | the page tree's /Count differs from the number of pages walked |
| [`flate-truncated`](flate-truncated.md) | `input` | Flate data was truncated or had a bad checksum; the partial output was kept |
| [`objstm-index-mismatch`](objstm-index-mismatch.md) | `input` | an object stream's index was stale and the object was found by searching |
| [`object-number-out-of-range`](object-number-out-of-range.md) | `limit` | an xref entry above max_object_number was dropped |
| [`aes-padding-invalid`](aes-padding-invalid.md) | `security` | AES-encrypted data had a bad length or padding and was kept as stored |
| [`perms-mismatch`](perms-mismatch.md) | `security` | an R5/R6 document's /Perms does not match its /P and /EncryptMetadata |
| [`warnings-truncated`](warnings-truncated.md) | `limit` | more than max_warnings warnings were raised and the rest were dropped |
| [`invalid-box`](invalid-box.md) | `input` | a page boundary box is not four numbers and reads as absent |
| [`invalid-rotate`](invalid-rotate.md) | `input` | a page's /Rotate is not a multiple of 90 and reads as 0 |
| [`name-tree-node-skipped`](name-tree-node-skipped.md) | `input` | a name- or number-tree node was malformed, repeated or too deep, and was skipped |
| [`outline-cycle`](outline-cycle.md) | `input` | an outline item was reached twice; that branch of the walk stopped |
| [`outline-item-skipped`](outline-item-skipped.md) | `input` | an outline item was missing, not a dictionary or too deep, and was skipped with its subtree |
| [`invalid-date`](invalid-date.md) | `input` | an Info date is not a valid PDF date; the raw string is kept |
| [`xmp-invalid`](xmp-invalid.md) | `input` | the XMP metadata packet is not readable XMP and reads as absent |
| [`annot-name-duplicate`](annot-name-duplicate.md) | `input` | two annotations on one page have the same name (/NM), which should be unique on its page |
| [`xmp-repaired`](xmp-repaired.md) | `input` | the XMP metadata packet had damage other readers read past, and was read the same way |
| [`invalid-page-label`](invalid-page-label.md) | `input` | a /PageLabels entry is malformed; the malformed part is dropped |
| [`invalid-entry`](invalid-entry.md) | `input` | an entry a view reads is unreadable or of the wrong type and reads as absent |
| [`content-unknown-operator`](content-unknown-operator.md) | `input` | an operator the spec does not define was skipped outside a BX/EX section |
| [`content-bad-operands`](content-bad-operands.md) | `input` | an operator had too few operands or operands of the wrong type, or extra ones |
| [`content-unbalanced-q`](content-unbalanced-q.md) | `input` | a Q without a matching q was ignored, or saved graphics states were left open |
| [`content-unbalanced-bt`](content-unbalanced-bt.md) | `input` | a text object was opened twice, closed outside one, or left open |
| [`content-unbalanced-marked`](content-unbalanced-marked.md) | `input` | an EMC without a matching BMC or BDC was ignored, or marked content was left open |
| [`content-syntax`](content-syntax.md) | `input` | content-stream bytes that are not a token were skipped |
| [`content-resource-missing`](content-resource-missing.md) | `input` | an operator named a resource that is missing, of the wrong type or unreadable |
| [`inline-image-unterminated`](inline-image-unterminated.md) | `input` | no EI ends an inline image; the rest of the content was skipped |
| [`xobject-cycle`](xobject-cycle.md) | `input` | a Form XObject draws itself, directly or through other forms |
| [`content-path-unpainted`](content-path-unpainted.md) | `input` | a content stream ended with a path no painting operator ended |
| [`font-invalid`](font-invalid.md) | `input` | a font dictionary has a missing or malformed entry, so its text was decoded with defaults |
| [`cmap-invalid`](cmap-invalid.md) | `input` | a font's CMap or ToUnicode CMap is malformed or names a predefined CMap Lamina PDF has no table for |
| [`unmapped-glyph`](unmapped-glyph.md) | `input` | a font shows glyphs that nothing maps to text |
| [`extract-limit-blocks`](extract-limit-blocks.md) | `limit` | a page had more runs of text than the extraction block limit; the runs past it were left out |
| [`extract-reading-order-ambiguous`](extract-reading-order-ambiguous.md) | `unsupported` | a page mixes horizontal and vertical writing, so the order of its blocks is a guess |
| [`extract-table-ambiguous`](extract-table-ambiguous.md) | `unsupported` | a table's rules marked out merged cells that aren't rectangles, so its cells were kept unmerged |
| [`extract-table-limit`](extract-table-limit.md) | `limit` | a page's ruling lines would make a table bigger than the extraction's cell limit; those lines gave no table |
| [`extract-struct-tree-ignored`](extract-struct-tree-ignored.md) | `input` | a tagged page's structure tree covered too little of its text, or its order jumped back across columns too often, so the layout gave the reading order |
| [`extract-untagged-content`](extract-untagged-content.md) | `input` | a page read in structure-tree order has visible text that is neither in the tree nor an artifact; it follows the tree's blocks, in layout order |
| [`extract-needs-ocr`](extract-needs-ocr.md) | `unsupported` | a page has no usable text layer: it is an image (with or without an invisible OCR layer), outlined glyphs, or glyphs without Unicode, so its extracted text is empty or unreliable |
| [`extract-column-order-uncertain`](extract-column-order-uncertain.md) | `unsupported` | the layout rules had to guess how a page's columns are read, so its blocks may be in the wrong order; the blocks concerned have a lower confidence |

## Errors

| Code | Kind | Summary |
| --- | --- | --- |
| [`not-pdf`](not-pdf.md) | `input` | the input does not look like a PDF at all |
| [`missing-root`](missing-root.md) | `input` | no document catalog (/Root) was found, even after repair |
| [`reference-cycle`](reference-cycle.md) | `input` | a reference chain or the page tree loops back on itself |
| [`limit-exceeded`](limit-exceeded.md) | `limit` | a hard Limits value was exceeded |
| [`strict`](strict.md) | `input` | strict mode refused something the tolerant reader would have repaired |
| [`data-pending`](data-pending.md) | `state` | the ByteSource has not fetched the byte ranges the call needs yet |
| [`source`](source.md) | `io` | the ByteSource failed to read |
| [`type`](type.md) | `input` | an object had a different type than the structure requires |
| [`invalid-structure`](invalid-structure.md) | `input` | an object stream, xref section or filter parameter was malformed |
| [`unset-object`](unset-object.md) | `internal` | a writer object number was reserved but never given a value |
| [`repaired-base`](repaired-base.md) | `state` | an incremental update was requested for a document whose xref needed repair |
| [`encrypted-base`](encrypted-base.md) | `unsupported` | a save of an encrypted document was requested |
| [`password-required`](password-required.md) | `security` | the document is encrypted with a user password and none was given |
| [`wrong-password`](wrong-password.md) | `security` | the given password is neither the user nor the owner password |
| [`unsupported-security-handler`](unsupported-security-handler.md) | `unsupported` | the document uses a security handler Lamina PDF cannot decrypt |
| [`unexpected-eof`](unexpected-eof.md) | `input` | the input ended before a complete token or object was read |
| [`syntax`](syntax.md) | `input` | a token or structure did not match the PDF grammar |
| [`unsupported-filter`](unsupported-filter.md) | `unsupported` | a stream uses a filter Lamina PDF does not implement |
| [`decompress`](decompress.md) | `input` | compressed stream data was corrupt |

## Op errors

| Code | Kind | Summary |
| --- | --- | --- |
| [`nested-transaction`](nested-transaction.md) | `state` | a transaction was started, or undo or redo called, while one is open |
| [`no-transaction`](no-transaction.md) | `state` | apply, commit or rollback was called without an open transaction |
| [`unknown-page`](unknown-page.md) | `usage` | an op names a page the document does not have |
| [`direct-page`](direct-page.md) | `unsupported` | the page is a direct dictionary, which ops cannot edit yet |
| [`unsupported-version`](unsupported-version.md) | `unsupported` | an op log has a schema version this reader does not know |
| [`invalid-op-log`](invalid-op-log.md) | `input` | an op log does not parse: an unknown op or field, a bad value, or not JSON |
| [`saved-not-rebased`](saved-not-rebased.md) | `state` | the document was saved to a sink Lamina PDF cannot read back and waits for rebase |
| [`no-pending-save`](no-pending-save.md) | `state` | rebase was called without a save waiting for it |
| [`malformed-xmp`](malformed-xmp.md) | `input` | a metadata edit would replace the document's XMP packet, which can't be read, losing what it holds |
| [`unknown-source`](unknown-source.md) | `usage` | an op or a call names a source document that was not added to the session |
| [`xmp-claim-read-only`](xmp-claim-read-only.md) | `usage` | an XMP edit would change the file's PDF/A or PDF/UA identification, which only conversion or a check may set |
| [`invalid-xmp-packet`](invalid-xmp-packet.md) | `input` | the XMP packet given to replace the document's doesn't parse |
| [`invalid-xmp-property`](invalid-xmp-property.md) | `usage` | an XMP property edit names a property, or gives a value, that can't be written |
| [`unknown-annotation`](unknown-annotation.md) | `usage` | the op names an annotation that no page of the document lists |
| [`annotation-locked`](annotation-locked.md) | `state` | the annotation's flags forbid the change: Locked forbids deleting it or changing its properties, LockedContents changing its contents |
| [`annotation-not-editable`](annotation-not-editable.md) | `unsupported` | the annotation can't be edited by the annotation ops: it is a form field's widget, or a direct dictionary with no object of its own |
| [`invalid-label-ranges`](invalid-label-ranges.md) | `usage` | page label ranges whose first isn't on the first page, that are out of page order or share a page, or that number from 0 |

## Op warnings

| Code | Kind | Summary |
| --- | --- | --- |
| [`pdfa-one-author`](pdfa-one-author.md) | `usage` | the file claims PDF/A-1, which allows one author, and its metadata lists more |
| [`pdfua-title-missing`](pdfua-title-missing.md) | `usage` | the file claims PDF/UA-1, which requires a title in its metadata, and it has none |
| [`pdfa-info-mismatch`](pdfa-info-mismatch.md) | `input` | the file claims PDF/A, which requires the Info dictionary and the XMP packet to agree, and they differ |

## Save errors

| Code | Kind | Summary |
| --- | --- | --- |
| [`sink`](sink.md) | `io` | the sink failed to take the bytes or could not give them back |
| [`not-rebased`](not-rebased.md) | `state` | the output was written, but rebasing the document onto it failed |
| [`sink-not-empty`](sink-not-empty.md) | `usage` | the sink already held bytes before the save started |
| [`would-invalidate-signatures`](would-invalidate-signatures.md) | `security` | the save would break one or more signatures |
| [`requires-rewrite`](requires-rewrite.md) | `state` | an incremental save was requested but only a full rewrite is safe |

## Text errors (`laminapdf-text`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`invalid-font`](invalid-font.md) | `input` | font bytes could not be read as a font |
| [`collection-index`](collection-index.md) | `usage` | a font collection member past the end of the collection was asked for |
| [`unknown-axis`](unknown-axis.md) | `usage` | an instance was asked for on a variation axis the font does not have |
| [`font-not-embeddable`](font-not-embeddable.md) | `usage` | the font's OS/2 fsType forbids embedding |
| [`missing-glyph`](missing-glyph.md) | `input` | the font has no glyph for a character of the text |
| [`glyph-out-of-range`](glyph-out-of-range.md) | `usage` | a glyph ID at or past the font's glyph count was used |
| [`unknown-font`](unknown-font.md) | `usage` | a FontKey was used with a registry that did not hand it out |
| [`subset-failed`](subset-failed.md) | `unsupported` | subsetting a font failed |
| [`invalid-axis-value`](invalid-axis-value.md) | `usage` | a variation axis value was NaN or infinite |
| [`wrong-document`](wrong-document.md) | `usage` | finish_fonts was given a document the registry did not reserve its fonts in |
| [`invalid-feature`](invalid-feature.md) | `usage` | an OpenType feature setting did not parse |
| [`invalid-text-range`](invalid-text-range.md) | `usage` | a byte range was out of bounds or not on character boundaries of the text |
| [`invalid-spans`](invalid-spans.md) | `usage` | the styled spans given for a paragraph do not cover its text once, in order |
| [`unsupported-layout-option`](unsupported-layout-option.md) | `unsupported` | a layout option that is not implemented yet was asked for |
| [`font-mismatch`](font-mismatch.md) | `usage` | a shaped run was given with a font key for a different font than it was shaped with |
| [`encoding-full`](encoding-full.md) | `limit` | a standard 14 font has no free one-byte code left for another glyph |
| [`font-not-shapable`](font-not-shapable.md) | `unsupported` | a standard 14 font was given to shaping or layout, which need a font program |

## Text warnings (`laminapdf-text`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`tounicode-conflict`](tounicode-conflict.md) | `usage` | one glyph was used for two different texts; ToUnicode keeps the first |
| [`whole-font-rebuilt`](whole-font-rebuilt.md) | `usage` | a font that forbids subsetting was rebuilt with every glyph kept |

## Build errors (`laminapdf-build`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`invalid-image`](invalid-image.md) | `input` | image bytes are not a readable image of the format they were given as |
| [`unsupported-image`](unsupported-image.md) | `unsupported` | the image uses a feature that PDF or Lamina PDF does not support |
| [`image-too-large`](image-too-large.md) | `limit` | the image needs more memory than the builder allows (a decoded PNG over 256 MiB, or raw dimensions whose size overflows) |
| [`image-data-length`](image-data-length.md) | `usage` | raw image samples are not width x height x channels bytes long |
| [`invalid-icc-profile`](invalid-icc-profile.md) | `input` | the bytes are not a usable ICC profile |
| [`color-components`](color-components.md) | `usage` | a colour has the wrong number of components for its colour space |
| [`invalid-color`](invalid-color.md) | `usage` | a colour string is not of the form #rgb or #rrggbb |
| [`invalid-geometry`](invalid-geometry.md) | `usage` | a page size, box, /UserUnit, drawing value (coordinate, matrix, line width, dash, miter limit) or image size is not usable |
| [`text-overflow`](text-overflow.md) | `usage` | a text box's lines don't all fit its height and its options asked for an error |
| [`page-index-out-of-range`](page-index-out-of-range.md) | `usage` | a link, outline item or destination names a page index the document doesn't have |
| [`unknown-destination`](unknown-destination.md) | `usage` | a link or outline item names a destination that was never defined |
| [`invalid-page-labels`](invalid-page-labels.md) | `usage` | page label ranges that don't start at page 0, don't increase, start past the last page or number from 0 |
| [`tag-element-not-open`](tag-element-not-open.md) | `usage` | a structure element resumed or closed isn't open |
| [`tag-element-unclosed`](tag-element-unclosed.md) | `usage` | structure elements opened across pages were never closed |
| [`profile-version-conflict`](profile-version-conflict.md) | `usage` | the PDF version asked for is one the profile doesn't allow (PDF/UA-1, PDF/A-2 and PDF/A-3 are based on PDF 1.7) |
| [`missing-title`](missing-title.md) | `usage` | PDF/UA-1 needs a document title, and the metadata gives none |
| [`missing-lang`](missing-lang.md) | `usage` | PDF/UA-1 needs the document's natural language, and none was set |
| [`missing-alt-text`](missing-alt-text.md) | `usage` | a Figure or Formula has no alternate text (/Alt or /ActualText), which PDF/UA-1 requires |
| [`untagged-content`](untagged-content.md) | `usage` | content was drawn outside every structure element and artifact, which PDF/UA-1 forbids |
| [`heading-level-skipped`](heading-level-skipped.md) | `usage` | a heading skips a level (the first is not H1, or an H3 follows an H1), which PDF/UA-1 forbids |
| [`unnumbered-heading-misused`](unnumbered-heading-misused.md) | `usage` | an unnumbered H heading shares its parent with another H, or the document also uses H1-H6, which PDF/UA-1 forbids |
| [`invalid-structure-nesting`](invalid-structure-nesting.md) | `usage` | a table or list element sits in a parent its type doesn't allow (a TD outside a TR, a P directly in a Table) |
| [`link-without-structure`](link-without-structure.md) | `usage` | a link annotation has no Link structure element (it was drawn inside an artifact), which PDF/UA-1 forbids |
| [`link-without-contents`](link-without-contents.md) | `usage` | a link annotation has no description (/Contents), which PDF/UA-1 requires |
| [`font-not-embedded`](font-not-embedded.md) | `usage` | a standard 14 font, which is never embedded, was used under a profile that needs every font embedded |
| [`glyph-without-unicode`](glyph-without-unicode.md) | `usage` | a glyph was drawn without a Unicode value, which PDF/A-2u and PDF/UA-1 forbid |
| [`device-color-without-output-intent`](device-color-without-output-intent.md) | `usage` | a DeviceCMYK colour or image was used, but the PDF/A output intent is sRGB |
| [`embedded-file-forbidden`](embedded-file-forbidden.md) | `usage` | PDF/A-2 doesn't allow attached files that aren't PDF/A themselves |
| [`embedded-file-without-mime-type`](embedded-file-without-mime-type.md) | `usage` | an attached file has no MIME type, which every PDF/A profile requires |
| [`flow-content-too-large`](flow-content-too-large.md) | `usage` | a line or a figure of a flow is taller than the page's body, so no page can hold it |
| [`flow-layout-unstable`](flow-layout-unstable.md) | `internal` | a flow's contents page kept changing its page numbers after four layouts |
| [`invalid-document`](invalid-document.md) | `usage` | a laminapdf.document/1 document has an unknown key, a missing one or a value of the wrong kind |
| [`template-data-missing`](template-data-missing.md) | `usage` | a template's $data, $each or $item points to a value the data doesn't have |
| [`template-data-type`](template-data-type.md) | `usage` | a template's placeholder found data of the wrong kind: $each over something that isn't a list, or text from an object |
| [`template-too-large`](template-too-large.md) | `usage` | a template would fill in more values, or nest deeper, than a document may hold |

## Build warnings (`laminapdf-build`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`flow-keep-broken`](flow-keep-broken.md) | `usage` | blocks kept together are taller than a page, so the flow split them |
| [`markdown-html-ignored`](markdown-html-ignored.md) | `unsupported` | the Markdown holds raw HTML, which the PDF leaves out |
| [`markdown-image-missing`](markdown-image-missing.md) | `usage` | the Markdown names an image that wasn't supplied, so the PDF leaves it out |

## Render errors (`laminapdf-render`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`render-too-large`](render-too-large.md) | `limit` | the requested tile is larger than the render limits allow; nothing was allocated |
| [`render-failed`](render-failed.md) | `input` | the render engine could not open the document or draw the page |
| [`render-backend-unavailable`](render-backend-unavailable.md) | `io` | the render engine is not available: its library or module could not be loaded |
| [`export-failed`](export-failed.md) | `internal` | an export's PNG, JPEG or TIFF encoder failed on a page that was drawn |

## Extract errors (`laminapdf-extract`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`model-failed`](model-failed.md) | `internal` | a layout model, OCR engine or page renderer plugged into an extraction failed; the extraction stops with no partial result |
| [`model-result-unexpected`](model-result-unexpected.md) | `usage` | a model result was handed to an extraction job that hadn't asked for it, or was the other model's result |
| [`model-needs-host`](model-needs-host.md) | `usage` | an extraction in one call was given a model whose results only the host can give: a host model, or a plug-in model without a page renderer |

## Façade errors (`laminapdf`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`cancelled`](cancelled.md) | `cancelled` | a CancelToken stopped the work; the job is finished and returned no partial result |
| [`job-finished`](job-finished.md) | `state` | a job was stepped after it had finished, failed or been cancelled |
| [`document-changed`](document-changed.md) | `state` | the document a job was created for changed (an edit, undo, redo, save or rebase) or a different document was passed to step |
| [`revision-index-out-of-range`](revision-index-out-of-range.md) | `usage` | an earlier revision was asked for by an index past the file's last revision |

## Façade warnings (`laminapdf`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`covered-text`](covered-text.md) | `security` | a page draws text and then paints a shape, an image or an annotation over it, so the text is hidden but still in the file |
| [`redaction-not-applied`](redaction-not-applied.md) | `security` | a page has redaction marks that were never applied, so the text under them is still in the file |
| [`text-in-earlier-revision`](text-in-earlier-revision.md) | `security` | an earlier revision of the file holds text that the latest revision no longer has |

## Binding errors (`laminapdf-wasm`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`builder-finished`](builder-finished.md) | `state` | a builder or one of its pages was used after it finished |
| [`disposed`](disposed.md) | `state` | a document, builder or other handle was used after free(), or after the using block that held it ended |
| [`invalid-argument`](invalid-argument.md) | `usage` | a value passed to a binding does not have the shape its type declares |
| [`source-budget-exceeded`](source-budget-exceeded.md) | `limit` | a sparse source was given more bytes than its byte budget allows |
| [`unknown-option`](unknown-option.md) | `usage` | an options record passed to a binding has a key Lamina PDF does not know |

## Package errors (`@laminapdf/sdk`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`already-initialized`](already-initialized.md) | `state` | init() or loadText() named a different source for a module that is already loaded or loading |
| [`extract-module-not-loaded`](extract-module-not-loaded.md) | `state` | page.text() was called on a document opened in the base module, which has no text extraction |
| [`fetch-failed`](fetch-failed.md) | `io` | a range source could not fetch input bytes: a network error, an HTTP status other than 200 and 206, a 206 whose Content-Range or length doesn't match the request, or a read that returned the wrong number of bytes |
| [`not-initialized`](not-initialized.md) | `state` | an @laminapdf/sdk function was called before init() finished loading the WebAssembly module |
| [`source-changed`](source-changed.md) | `io` | a remote file changed while it was being read: its ETag or length differs from the first response |
| [`text-module-not-loaded`](text-module-not-loaded.md) | `state` | a font or text call was made on a builder of the base module, which has no fonts or text |
| [`worker-version-mismatch`](worker-version-mismatch.md) | `usage` | the worker answered with a different protocol or package version than the client |
| [`worker-crashed`](worker-crashed.md) | `internal` | the worker running Lamina PDF stopped (an uncaught error, a WebAssembly trap or out of memory); calls in flight were rejected |
| [`worker-failed`](worker-failed.md) | `internal` | the worker client failed with an error that isn't Lamina PDF's (a bug in @laminapdf/sdk/worker, or a result it couldn't send) |
| [`wasm-trap`](wasm-trap.md) | `internal` | the WebAssembly module trapped (a bug in Lamina PDF), so its instance was discarded and the next call gets a fresh one |
| [`out-of-memory`](out-of-memory.md) | `limit` | the WebAssembly module ran out of memory (its maximum, or what the engine could give), so its instance was discarded and the next call gets a fresh one |
| [`instance-reset`](instance-reset.md) | `state` | a handle was used after its WebAssembly instance was replaced, by reset() or after a trap |

## CLI errors (`laminapdf-cli`)

| Code | Kind | Summary |
| --- | --- | --- |
| [`invalid-command-line`](invalid-command-line.md) | `usage` | the command line doesn't parse: an unknown command or flag, a missing argument, a bad value, or stdin named twice |
| [`invalid-page-range`](invalid-page-range.md) | `usage` | a page range doesn't fit the grammar, or names a page the document lacks |
| [`binary-output-to-terminal`](binary-output-to-terminal.md) | `usage` | the command would write a PDF to standard output, and standard output is a terminal |
| [`unknown-code`](unknown-code.md) | `usage` | explain was given a code that isn't a Lamina PDF error or warning code |
| [`overwrite-refused`](overwrite-refused.md) | `state` | the output file exists, or the command would modify its input in place, and --force wasn't given |
| [`input-unreadable`](input-unreadable.md) | `io` | an input file, standard input or the password file couldn't be read |
| [`output-unwritable`](output-unwritable.md) | `io` | the output couldn't be written: its folder is missing or read-only, the disk is full, or standard output closed |
| [`path-outside-roots`](path-outside-roots.md) | `security` | laminapdf mcp was given a path outside its --root folders, or a link that leads out of them |
