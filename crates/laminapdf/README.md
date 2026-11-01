# `lamina`

The façade crate: one dependency for Rust users, and the API every binding wraps one-to-one (`laminapdf-wasm`, `laminapdf-ffi`, Tauri commands; [ADR 0002](../../docs/decisions/0002-binding-strategy.md) §3, [ADR 0028](../../docs/decisions/0028-facade-rules-jobs-and-cancellation.md)). It re-exports and wraps the core crates (`laminapdf-cos`, `laminapdf-model`, `laminapdf-ops`, `laminapdf-save`, and with the `build` and `text` features `laminapdf-build` and `laminapdf-text`). The crate docs (`cargo doc -p laminapdf --open`) open with an invoice built, saved, reopened, edited and saved incrementally.

## What it offers

| Area | API |
| --- | --- |
| Open and read | `Document::open_bytes(Vec<u8>, OpenOptions)`, `open_source(Arc<dyn ByteSource>, …)`; `page_count`, `page_info(index)`, `page_infos()`, `page_by_id(&PageId)` (one page-tree walk, cached until the next change), `info`, `warnings()` (a `WarningCursor` with `next_batch(max)`; build warnings first) |
| Edit | `apply(Vec<Op>)` (one transaction), `apply_log(json)`, `undo`, `redo` |
| Save | `save(&SaveOptions, &CancelToken) -> Saved { bytes, report }`, or `save_job(&SaveOptions)` stepped with a `Budget`; after `not-rebased`, `Error::saved_output()` and `rebase(bytes)` |
| Build (`build`; fonts and text with `text`) | `Builder` (fonts, pages and their `Surface`, destinations, outline, page labels, metadata; tagged PDF: `Surface::tag`, `figure`, `artifact`, `tagging_diagnostics`) → `finish()` → `Document` |
| Extract (`text`) | `page_text(index, &TextOptions)` → `extract::PageText` (`page_texts` for every page in one walk): glyphs with Unicode and quads in page space, runs (font, size, MCID, `ActualText`, invisible), `lines()` (todo B9d); fonts parsed once per document and revision (a `RefCell` cache like the page facts') |
| Errors | `Error` with `code()`, `kind()`, `location()`, `hint()`, `docs_url()`, `diagnostic()`, and `Serialize` to the [ADR 0026](../../docs/decisions/0026-error-and-diagnostic-contract.md) §1 object; `Warning` likewise |

The ADR 0028 rules it follows, by number: no lifetimes on `Document` (F1: pages by index or `PageId`, owned `PageInfo`); no generics or closures in its signatures (F3: `open_bytes`/`open_source`, ops as values); `&self` reads and `&mut self` changes with no interior mutability (F4); owned records with `u32` counts (F5); options as `with_*` builders (F6); `Vec<u8>` in and out (F7); a cursor for warnings (F8); one error type (F9); jobs plus a run-to-completion form taking a `CancelToken`, no callbacks (F10). `Document` is `Send` and not `Sync` (`tests/facade.rs` asserts both at compile time): the lock lives in the edge crates (§2). The `Builder` keeps `laminapdf-build`'s Rust shape (borrowed page builders, closures for paths); its binding form belongs to the edge crates.

## Jobs, budgets and cancellation

Every job records the document's instance and revision when it is created, and every `step` checks them first: a committed `apply`, an `undo`, a `redo`, a save or a rebase moves the revision, and a job stepped after that fails with `document-changed` and finishes. Stepping a finished job is `job-finished`; a cancelled token is `cancelled`. Errors end the job, and a failed or cancelled save leaves the document as it was.

`SaveJob`'s work units: planning is 1 unit (it also checks that a save may start); for a full rewrite of an opened document, 1 unit per object of the base the session didn't change, parsed and cached ahead of the writer (`Phase::Parse`); then writing and rebasing is 1 indivisible unit (`Phase::Write`), because `laminapdf-save`'s writers are not resumable yet. A step that reaches the write therefore overshoots its budget by serialising the whole file (cancellation lands before it, not inside it).

Measured with `cargo test --release -p laminapdf --test budget -- --ignored --nocapture`. The test uses a 2,000-page document from bytes (4,027 base objects) and saves it as a full rewrite: planning as one step, the whole parse phase as the next, then the write. Three runs on an Apple M1 Pro (macOS 27, rustc 1.99.1, a shared machine):

| Phase | Measured | Starting budget for an 8 ms slice (Web Worker) | for 50 ms (native) |
| --- | --- | ---: | ---: |
| Plan (1 unit) | ≈ 0.03 ms | | |
| Parse (1 unit = 1 object) | 550–990 units/ms | ≈ 4,000 units | ≈ 25,000 units |
| Write + rebase (1 unit) | 12–17 ms for 2,000 pages (310 kB) | one step | one step |

Edges should still adapt the next budget from the time the last step took (ADR 0028 §4); these numbers are only where to start.

## Features

- `build`: the builder without text (`laminapdf-build` without its `text` feature): pages, paths, colour, images, links, navigation and metadata. The WASM base module's builder ([ADR 0016](../../docs/decisions/0016-package-design.md) §4).
- `text` (default; implies `build`): fonts and text (`laminapdf-text`). Without `build` or `text` the crate opens, reads, edits and saves only; `cargo xtask portability` checks the three builds for wasm32, iOS and Android.
- `saslprep` (default): SASLprep (RFC 4013) for R5/R6 passwords. Left out of the WASM modules, whose TS layer normalises passwords to NFKC (ADR 0016 §7).

## Examples and goldens

`examples/`: `hello`, `invoice`, `multilingual` (Latin, Arabic, Devanagari, Hebrew, CJK), `vectors` (paths, colour, opacity, blend, clip, images), `links_and_outlines`, `generate_then_edit` (build → save → open → `SetInfo` → incremental save). Fonts and images come from `fixtures/` as bytes (`include_bytes!`); the library never reads files.

```sh
cargo run -p laminapdf --example invoice -- target/examples-out   # writes target/examples-out/invoice.pdf
cargo xtask examples                  # every example: byte-compare with fixtures/golden/examples/, qpdf --check
cargo xtask golden --update           # rewrite the byte goldens (with every other golden)
cargo xtask render-golden [--update]  # PDFium + hayro renders vs fixtures/render-golden/ (see its README)
```

`tests/examples.rs` builds every example in process and compares it with its golden, twice (determinism); `tests/facade.rs` covers the ADR 0028 contract.
