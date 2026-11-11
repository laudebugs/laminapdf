---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Edit a document's metadata with the Info dictionary and the XMP metadata kept in step:

- `doc.setMetadata(patch, { at, replaceMalformed })` sets or removes the title, the authors, the subject, the keywords, the creating and producing applications and the dates. In PDF 2.0 files only the dates go to the Info dictionary.
- `doc.removeMetadata(keys)` removes fields from both.
- `doc.metadata()` reads both copies together and says where they disagree.
- `doc.plan(options)` says how a save would go, without saving: the save constraint, the signatures, the refusal code, and the PDF/A and PDF/UA claims the edits break.
- `doc.wouldViolate(op)` checks one edit against the signatures.
- Saves take `metadataDate`, the time of the save, to stamp it as the modification date, and `instanceId` to set the XMP instance ID; either option turns the stamp on. The report says whether it happened (`metadataUpdated`, `metadataNotUpdated`).
- `Transaction.setMetadata` and `removeMetadata` take the same options.
- The `setInfo` op is deprecated in favour of `setMetadata`. It still applies, and now updates existing XMP metadata too.

In Rust: `SetMetadata`, `RemoveMetadata`, `Document::metadata`, `Document::plan`, `Document::would_violate` and `SaveOptions::with_metadata_date`; `SetInfo` is `#[deprecated]`.
