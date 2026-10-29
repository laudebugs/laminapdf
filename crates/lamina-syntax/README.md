# lamina-syntax

The PDF syntax layer of lamina's pure core (bytes in, bytes out): the object model (`Object`, `Dict`, `Name`, `PdfString`, `Stream`, `ObjRef`) with byte-exact serialization, the lexer, the object parser, the stream filters (Flate, PNG predictors), `crypt` (standard security handler decryption: password authentication for R2–R6, RC4/AES-128/AES-256, read only; ISO 32000-2 §7.6), and `Limits`, the resource limits of [ADR 0003](../../docs/decisions/0003-reader-tolerance-and-limits.md) that bound nesting, string length, decoded bytes and filter chains, and `text`: text-string decoding (PDFDocEncoding, UTF-16BE, UTF-8; §7.9.2.2) with the workspace's one PDFDocEncoding table (Annex D), in both directions. ISO 32000-2 §7.2–7.4.

Ported from the [`pdf-basics` spike](../../spikes/pdf-basics/rust/README.md) in architecture Round A; see its design notes for the reasoning behind the rules (number formatting, string escaping, the depth limit, predictor bounds). The file structure built on top of this crate is in [`lamina-cos`](../lamina-cos/README.md). Plan: [`docs/plans/architecture.md`](../../docs/plans/architecture.md).

```sh
cargo test -p lamina-syntax
```
