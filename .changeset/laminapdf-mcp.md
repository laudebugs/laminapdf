---
"@laminapdf/sdk": minor
---

New: `laminapdf mcp`, an MCP server on standard input and output that serves the CLI's commands as tools, in builds with the `laminapdf-cli` crate's `mcp` feature (ADR 0031 §9). It touches only files under its `--root` folders, is read-only unless started with `--allow-write`, and keeps document text out of everything but report fields. A path outside the roots is the new error code `path-outside-roots`, so the TypeScript `ErrorCode` union lists it too. The Rust façade exposes the error schema as `laminapdf::ERROR_SCHEMA`.
