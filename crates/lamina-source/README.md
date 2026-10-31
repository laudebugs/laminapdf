# lamina-source

Edge byte sources for lamina ([ADR 0005](../../docs/decisions/0005-document-model.md) §2, [L2c](../../docs/todos/done/L2c-range-byte-source.md)): inputs that are not one buffer in memory. The core reads synchronously; a source whose bytes haven't arrived answers `Pending`, the call fails with `data-pending` and the ranges it needs, and the shell fetches them and calls again.

- **`SparseSource`** (pure, builds for wasm32): a source of known length that starts empty. `supply(offset, bytes)` adds a chunk (chunks are `Arc<[u8]>`, merged when they touch; bytes already held win over overlapping ones; a byte budget bounds them, `SupplyError::Budget` past it); `missing(ranges)` lists what a fetch still needs; reads of a gap answer `Pending` up to the next chunk. Nothing is evicted: the document may have read it. Behind a lock, since `supply` runs while a document shares the source, which is why it lives here and not in `lamina-cos` (ADR 0005 §7).
- **`retry` and `open_with`** (pure): the fetch-and-retry loop. On `data-pending` the ranges are widened to blocks (`RetryOptions::block_size`, default 4 KiB: the reader checks every object's xref entry with a 32-byte probe, so large blocks around each probe would fetch most of the file) plus `readahead`, reduced to what the source doesn't hold, fetched with the shell's closure (one call per range, in file order) and supplied. It gives up with the last `data-pending` when a round asks for exactly what the round before asked (no progress) or after `max_rounds` (default 1 024: reading one large stream takes a few rounds, as the reader asks for each gap as it meets it). `pending_ranges(&error)` reads the ranges of a `data-pending`.
- **`FileSource`** (feature `native`): a file read with positioned reads (`pread`, `seek_read`), no mapping and no `unsafe`; a file truncated after open answers `SourceError::Failed`, not a crash. The default for large local files.
- **`MmapSource`** (feature `mmap`, [memmap2](https://crates.io/crates/memmap2), MIT OR Apache-2.0): the file mapped read-only, `as_slice` the mapping, so the parser reads in place. **Only for files nothing changes while open:** a truncation raises `SIGBUS`, which kills the process ([D2b](../../docs/todos/ready/D2b-engine-process.md) keeps it out of the desktop main process for that reason). The one `unsafe` block, with its `// SAFETY:` note, is in `src/mmap.rs`.

```rust
use std::sync::Arc;
use lamina_source::{RetryOptions, SparseSource, open_with, retry};

let source = Arc::new(SparseSource::new(size));
let doc = open_with(&source, &lamina::OpenOptions::default(), &RetryOptions::default(), |range| fetch(range))?;
let pages = retry(&source, &RetryOptions::default(), |range| fetch(range), || doc.page_infos())?;
```

The wasm edge wraps `SparseSource` as `WasmSparseSource` (`lamina-wasm`), and `@laminapdf/sdk` runs the same loop with `await` in `Document.openRange` / `doc.ensure` ([its README](../../packages/sdk/README.md#remote-and-large-files)).

```sh
cargo test -p lamina-source --all-features
```
