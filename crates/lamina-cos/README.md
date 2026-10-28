# lamina-cos

The PDF file structure (COS layer) of lamina's pure core (bytes in, bytes out), built on [`lamina-syntax`](../lamina-syntax/README.md):

- **Reader** (`Document::open(source, OpenOptions)`): owns its input as a shared `ByteSource` (`Vec<u8>`, `Arc<[u8]>`, `&'static [u8]` or your own; ADR 0005) and is `Send` but not `Sync`. Classic xref tables and xref streams, `/Prev` chains, hybrid `/XRefStm` files, object streams, repair by scanning when the stored xref is unusable, an object cache, and the page tree with inherited attributes. Reading is bounded by `Limits` and every repair is a `Warning` with a stable code; strict mode turns warnings into errors (ADR 0003). The tolerance policy is in the crate docs (`src/lib.rs`).
- **Writer** (`Writer`, `incremental_update(&Document, …)`): classic, compact (object stream + xref stream) and incremental output, byte-deterministic. It moves into `lamina-save` in Round B.

Ported from the [`pdf-basics` spike](../../spikes/pdf-basics/rust/README.md) in architecture Round A, with the spike's tests. Integration tests read the shared [`fixtures/`](../../fixtures/README.md); `tests/common/` also holds the content builder and the golden samples, which are generation code that moves to `lamina-build` in Round B. Plan: [`docs/plans/architecture.md`](../../docs/plans/architecture.md).

```sh
cargo test -p lamina-cos
```
