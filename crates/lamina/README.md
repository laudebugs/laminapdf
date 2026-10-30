# `lamina`

The façade crate: one dependency for Rust users, and the API every binding wraps one-to-one (`lamina-wasm`, `lamina-ffi`, Tauri commands; [ADR 0002](../../docs/decisions/0002-binding-strategy.md) §3, [ADR 0028](../../docs/decisions/0028-facade-rules-jobs-and-cancellation.md)). It re-exports and wraps the core crates (`lamina-cos`, `lamina-model`, `lamina-ops`, `lamina-save`, and with the `text` feature `lamina-text` and `lamina-build`). The crate docs (`cargo doc -p lamina --open`) open with an invoice built, saved, reopened, edited and saved incrementally.

## What it offers

| Area | API |
| --- | --- |
| Open and read | `Document::open_bytes(Vec<u8>, OpenOptions)`, `open_source(Arc<dyn ByteSource>, …)`; `page_count`, `page_info(index)`, `page_infos()`, `page_by_id(&PageId)` (one page-tree walk, cached until the next change), `info`, `warnings()` (a `WarningCursor` with `next_batch(max)`; build warnings first) |
| Edit | `apply(Vec<Op>)` (one transaction), `apply_log(json)`, `undo`, `redo` |
| Save | `save(&SaveOptions, &CancelToken) -> Saved { bytes, report }`, or `save_job(&SaveOptions)` stepped with a `Budget`; after `not-rebased`, `Error::saved_output()` and `rebase(bytes)` |
| Build (`text`) | `Builder` (fonts, pages and their `Surface`, destinations, outline, page labels, metadata) → `finish()` → `Document` |
| Errors | `Error` with `code()`, `kind()`, `location()`, `hint()`, `docs_url()`, `diagnostic()`, and `Serialize` to the [ADR 0026](../../docs/decisions/0026-error-and-diagnostic-contract.md) §1 object; `Warning` likewise |

The ADR 0028 rules it follows, by number: no lifetimes on `Document` (F1: pages by index or `PageId`, owned `PageInfo`); no generics or closures in its signatures (F3: `open_bytes`/`open_source`, ops as values); `&self` reads and `&mut self` changes with no interior mutability (F4); owned records with `u32` counts (F5); options as `with_*` builders (F6); `Vec<u8>` in and out (F7); a cursor for warnings (F8); one error type (F9); jobs plus a run-to-completion form taking a `CancelToken`, no callbacks (F10). `Document` is `Send` and not `Sync` (`tests/facade.rs` asserts both at compile time): the lock lives in the edge crates (§2). The `Builder` keeps `lamina-build`'s Rust shape (borrowed page builders, closures for paths); its binding form belongs to the edge crates.

## Jobs, budgets and cancellation

Every job records the document's instance and revision when it is created, and every `step` checks them first: a committed `apply`, an `undo`, a `redo`, a save or a rebase moves the revision, and a job stepped after that fails with `document-changed` and finishes. Stepping a finished job is `job-finished`; a cancelled token is `cancelled`. Errors end the job, and a failed or cancelled save leaves the document as it was.

`SaveJob`'s work units: planning is 1 unit (it also checks that a save may start); for a full rewrite of an opened document, 1 unit per object of the base the session didn't change, parsed and cached ahead of the writer (`Phase::Parse`); then writing and rebasing is 1 indivisible unit (`Phase::Write`), because `lamina-save`'s writers are not resumable yet. A step that reaches the write therefore overshoots its budget by serialising the whole file (cancellation lands before it, not inside it).

Measured with `cargo test --release -p lamina --test budget -- --ignored --nocapture`. The test uses a 2,000-page document from bytes (4,027 base objects) and saves it as a full rewrite: planning as one step, the whole parse phase as the next, then the write. Three runs on an Apple M1 Pro (macOS 27, rustc 1.99.1, a shared machine):

| Phase | Measured | Starting budget for an 8 ms slice (Web Worker) | for 50 ms (native) |
| --- | --- | ---: | ---: |
| Plan (1 unit) | ≈ 0.03 ms | | |
| Parse (1 unit = 1 object) | 550–990 units/ms | ≈ 4,000 units | ≈ 25,000 units |
| Write + rebase (1 unit) | 12–17 ms for 2,000 pages (310 kB) | one step | one step |

Edges should still adapt the next budget from the time the last step took (ADR 0028 §4); these numbers are only where to start.

## Features

- `text` (default): fonts (`lamina-text`) and the builder (`lamina-build`). Without it the crate opens, reads, edits and saves only, the structure-only build for WASM splitting; `cargo xtask portability` checks both builds for wasm32, iOS and Android. The builder needs `lamina-text`, so it goes with the feature.

## Examples and goldens

`examples/`: `hello`, `invoice`, `multilingual` (Latin, Arabic, Devanagari, Hebrew, CJK), `vectors` (paths, colour, opacity, blend, clip, images), `links_and_outlines`, `generate_then_edit` (build → save → open → `SetInfo` → incremental save). Fonts and images come from `fixtures/` as bytes (`include_bytes!`); the library never reads files.

```sh
cargo run -p lamina --example invoice -- target/examples-out   # writes target/examples-out/invoice.pdf
cargo xtask examples                  # every example: byte-compare with fixtures/golden/examples/, qpdf --check
cargo xtask golden --update           # rewrite the byte goldens (with every other golden)
cargo xtask render-golden [--update]  # PDFium + hayro renders vs fixtures/render-golden/ (see its README)
```

`tests/examples.rs` builds every example in process and compares it with its golden, twice (determinism); `tests/facade.rs` covers the ADR 0028 contract.
