---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Merge and split documents. `Document.merge(files)` makes one document from several files, with a bookmark named after each (`{ title }`) holding its own bookmarks; `{ outline: "keep" }` or `"none"` changes that. `doc.mergeDocuments(sources, at)` merges source documents into an open one as one edit. The files' form fields, named destinations, layers and page labels come along; a destination whose name is taken becomes `<name>#2` (`renamedDestinations`), and bookmarks to pages left behind are dropped (`bookmarksDropped`). Merging into a file that claims a PDF/A level from one that doesn't claim it warns with the new code `imported-pdfa-unchecked`. `doc.split({ count })`, `split({ ranges })` and `split({ bookmarks: level })` cut a document into new ones, each with the bookmarks that go to its pages.

In Rust: `MergeDocuments`, `MergeSource`, `MergeOptions`, `OutlineMerge`, `MergeInput`, `RenamedDestination`, `SplitBy`, `Document::merge`, `merge_with`, `merge_job` and `MergeJob`, `Document::split`, `split_with` and `split_job`, with `Edited::renamed_destinations` and `Edited::bookmarks_dropped`. `Document::extract_pages` now carries the bookmarks that go to its pages and the layers.
