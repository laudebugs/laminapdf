---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Edit a document's metadata with the Info dictionary and the XMP metadata kept in step: `doc.setMetadata(patch, { at, replaceMalformed })` sets or removes the title, authors, subject, keywords, creator and producer tools and the dates, `doc.removeMetadata(keys)` removes fields, and `doc.metadata()` reads both copies together, saying where they disagree. In PDF 2.0 files only the dates go to the Info dictionary. `doc.plan(options)` shows how a save would go before it happens (the save constraint, the signatures and what the edits break of them, the refusal code, the PDF/A and PDF/UA claims the edits break), and `doc.wouldViolate(op)` checks one edit. Saves take `metadataDate` and `instanceId` to stamp the time of the save, and the report says whether it did (`metadataUpdated`, `metadataNotUpdated`). The `setInfo` op is deprecated in favour of `setMetadata`; it still applies, and now updates existing XMP metadata too. In Rust: `SetMetadata`, `RemoveMetadata`, `Document::metadata`, `Document::plan`, `Document::would_violate` and `SaveOptions::with_metadata_date`.
