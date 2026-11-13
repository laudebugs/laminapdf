---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Choose how a PDF opens, and read its article threads. `doc.setInitialView(patch)` sets the page and zoom a file opens at, the panel beside the page (bookmarks, thumbnails, attachments, layers or full screen), the page layout and the reader window's requests, such as the title in the title bar; every PDF reader reads them. `doc.initialView()` reads them back, page targets in page space, and `doc.threads()` lists the file's article threads with each part's page and rectangle. An open action that runs a script is reported, never run, and never replaced: an edit that names the opening page fails with `open-action-not-destination`. In Rust: the `SetInitialView` op, `Document::initial_view` and `Document::threads`.
