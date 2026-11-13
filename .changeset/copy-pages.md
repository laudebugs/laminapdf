---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Copy pages between documents. `doc.addSource(bytes)` opens another PDF as a source and returns its ID and page IDs. `doc.insertPagesFrom(id, pages, at)` copies its pages in, and `doc.replacePages(targets, id, pages)` puts them in other pages' places, keeping the bookmarks, links and named destinations that pointed at the old pages. A copy keeps its annotations and form fields; a field whose name is taken becomes `<name>#2` (or shares the value, with `{ fields: "link" }`), a link to a page left behind is removed (`linksRemoved`), and tags are dropped (`tagsDropped`, with the new warning `imported-tags-dropped` on a PDF/UA-1 file). `doc.extractPages(ids)` makes a new document from pages, with their labels, and `doc.exportRanges(ranges)` makes one per range.

In Rust: `InsertPagesFrom`, `ReplacePages`, `Carry`, `FieldClash`, `Document::add_source`, `source_pages`, `extract_pages`, `export_ranges`, `export_ranges_with` and `ExportJob`, with `Edited::links_removed` and `Edited::tags_dropped`. `laminapdf-model` gains `FieldNames::clashes`, `Document::field_by_name` and `Outline::named_destination_value`.
