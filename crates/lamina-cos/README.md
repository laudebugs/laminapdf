# lamina-cos

The PDF file structure (COS layer) of lamina's pure core (bytes in, bytes out), built on [`lamina-syntax`](../lamina-syntax/README.md):

- **Reader** (`Document::open(source, OpenOptions)`): owns its input as a shared `ByteSource` (`Vec<u8>`, `Arc<[u8]>`, `&'static [u8]` or your own; ADR 0005) and is `Send` but not `Sync`. In-memory sources are parsed in place; any other source is read in 64 KiB windows around the offsets the parser needs (doubled while an object runs past a window; the 64 most recent are cached per `Source`), xref entries are checked with small probes, and repair scans in overlapping chunks, with the same results as in memory. A source that answers `SourceError::Pending` makes the call (open or any lazy call: `resolve`, `pages`, …) fail with `Error::DataPending { ranges }`, listing only the windows still needed: fetch them and retry. Classic xref tables and xref streams, `/Prev` chains, hybrid `/XRefStm` files, object streams, repair by scanning when the stored xref is unusable, an object cache, and the page tree with inherited attributes. Encrypted files (standard security handler R2–R6: RC4, AES-128, AES-256, crypt filters, `/EncryptMetadata`) open with `OpenOptions::with_password` (or the empty user password) and are decrypted as objects load; `encryption()`, `permissions()` and `matched_password()` describe them. Read only: incremental updates on an encrypted base are refused (`EncryptedBase`) until full-editor E11. Reading is bounded by `Limits` and every repair is a `Warning` with a stable code; strict mode turns warnings into errors (ADR 0003). The tolerance policy is in the crate docs (`src/lib.rs`).
- **Writer** (`Writer`, `incremental_update(&Document, …)`): classic, compact (object stream + xref stream) and incremental output, byte-deterministic. It moves into `lamina-save` in Round B.

Ported from the [`pdf-basics` spike](../../spikes/pdf-basics/rust/README.md) in architecture Round A, with the spike's tests. Integration tests read the shared [`fixtures/`](../../fixtures/README.md); `tests/common/` also holds the content builder and the golden samples, which are generation code that moves to `lamina-build` in Round B. Plan: [`docs/plans/architecture.md`](../../docs/plans/architecture.md).

```sh
cargo test -p lamina-cos
```
