# lamina-syntax

The PDF syntax layer of lamina's pure core (bytes in, bytes out): the object model (`Object`, `Dict`, `Name`, `PdfString`, `Stream`, `ObjRef`) with byte-exact serialization, the lexer, the depth-limited object parser, and the stream filters (Flate with a size limit, PNG predictors). ISO 32000-2 §7.2–7.4.

Ported from the [`pdf-basics` spike](../../spikes/pdf-basics/rust/README.md) in architecture Round A; see its design notes for the reasoning behind the rules (number formatting, string escaping, the depth limit, predictor bounds). The file structure built on top of this crate is in [`lamina-cos`](../lamina-cos/README.md). Plan: [`docs/plans/architecture.md`](../../docs/plans/architecture.md).

```sh
cargo test -p lamina-syntax
```
