# Spike: `pdf-basics` — the same minimal PDF core in Rust and TypeScript

Status: research spike (throwaway-quality API, production-quality correctness). Not the lamina architecture — input to it.

## Why

Chapter 05 recommends a Rust core shipped to web (WASM), desktop (native) and mobile (iOS/Android), with TypeScript as the first binding. This spike tests that recommendation by building the *same* small PDF core twice and comparing: code size, ergonomics, correctness, cross-platform builds, and what it takes to call the Rust core from TS via WASM.

Both implementations must pass the **same test suite** (same fixtures, same expected outputs).

## Platform targets

| Target | Rust | TypeScript |
| --- | --- | --- |
| Desktop / server | native (`cargo test`) | Node ≥ 20 |
| Web | `wasm32-unknown-unknown` | any modern browser (ESM, no bundler assumptions) |
| Mobile | `aarch64-apple-ios`, `aarch64-linux-android` (`cargo check` / `cargo build --lib`) | React Native / Hermes-compatible: standard JS only |

Hard rules for the **core library** (not tests/CLI):
- Bytes in, bytes out. No file system, no network, no threads, no clocks, no randomness.
- Rust: `std` allowed, but no `std::fs`/`std::net`/`std::process` in the library. No `unsafe` except in the WASM C-ABI shim (`#![deny(unsafe_code)]` elsewhere). No panics on any input: every parse path returns `Result`.
- TS: only ECMAScript + `TextEncoder`/`TextDecoder`. No `Buffer`, no `node:*` imports, no `fs`, no `process` in `src/`. Strict TS, ESM only.
- Dependencies: Flate only. Rust: `miniz_oxide` (MIT/Apache/Zlib). TS: `fflate` (MIT). Nothing else at runtime. Dev deps are fine.

## Scope

### 1. Object model (ISO 32000-1 §7.3)
`Null, Bool, Integer (i64), Real (f64), String (bytes; literal or hex form remembered), Name, Array, Dict (insertion-ordered), Stream (dict + raw bytes), Ref (obj, gen)`.

Serialization rules (must be exact — golden tests depend on them):
- Names: `/` + bytes; escape as `#xx` (uppercase hex, 2 digits) any byte outside `!`..`~`, plus `#` and the delimiters `( ) < > [ ] { } / %`.
- Literal strings: `(`…`)`, escape `\`, `(`, `)` with a backslash; also `\r` as `\r`; other bytes raw.
- Hex strings: `<` uppercase hex `>`.
- Integers: decimal. Reals: no exponent ever; up to 6 fractional digits, trailing zeros and trailing `.` trimmed (`0.5`, `1`, `-0.25`); `-0` → `0`. (Content-stream helpers below format their own operands.)
- Arrays: `[a b c]`. Dicts: `<< /K v /K2 v2 >>` (single spaces, as in the fixtures). Refs: `n g R`.

### 2. Writer
Produces: header `%PDF-x.y\n%\xE2\xE3\xCF\xD3\n`, objects `n 0 obj\n…\nendobj\n` in order, classic xref table, trailer, `startxref\n<off>\n%%EOF\n`.
- Xref entry EOL configurable: `CrLf` (`\r\n`) or `SpaceLf` (` \n`) — both 20-byte forms are legal (§7.5.4); the two golden fixtures use different ones.
- Stream objects: `<< …dict… /Length N >>\nstream\n<bytes>\nendstream` — the writer sets `/Length` (written last in the dict if not already present; if the caller supplied `/Length` in a position, keep that position and overwrite the value). Optional Flate encoding sets `/Filter /FlateDecode`.
- **Incremental update** (§7.5.6): given original bytes + changed/new objects, append body + xref subsection(s) + trailer with `/Prev` and all previous trailer keys (except `/Prev`) repeated; original bytes untouched.
- **Compact mode** (PDF 1.5+): non-stream objects packed into one object stream (`/Type /ObjStm /N /First`, Flate), plus an xref stream (`/Type /XRef /W [1 4 2] /Size /Index`, Flate). Must round-trip through the reader and be readable by pypdf.

### 3. Reader (tolerant parser)
- Lexer + object parser for everything in §1, including comments, all whitespace kinds, literal-string escapes (`\n \r \t \b \f \( \) \\ \ddd`, line continuation), balanced parens, hex strings with whitespace and odd length (pad 0).
- `startxref` → xref table **or** xref stream; follow `/Prev` chains (newest wins); hybrid `/XRefStm` optional.
- Object streams (type-2 xref entries), Flate decode with PNG predictors (`/Predictor 10–15`, `/Columns`) — xref streams commonly use predictor 12.
- **Repair**: if `startxref`/xref is missing or wrong (offset doesn't point at `xref` or at `n g obj` of an XRef stream, or an entry doesn't point at its object), rebuild the xref by scanning for `\d+ \d+ obj`, last definition wins; find the trailer by scanning for `trailer` or an XRef stream with `/Root`.
- API: `open(bytes) -> Document` with `trailer()`, `catalog()`, `resolve(ref)`, `page_count()`, `pages()` (walk the page tree, with inherited `/MediaBox`, `/Resources`, `/Rotate`, `/CropBox`), `revisions()` (number of xref sections reached via `/Prev`), `is_encrypted()` (trailer has `/Encrypt`; no decryption in this spike), `was_repaired()`.
- Robustness limits: max nesting depth 256, reference-cycle detection in resolve and page-tree walk, refuse decompressed streams > 64 MiB (configurable). Garbage / truncated input must produce an error, never a panic/exception escape/hang.

### 4. Content builder
A small builder that emits content-stream text. Number formatting for operands: same rule as reals in §1, except where a helper takes an explicit precision. Operators needed to reproduce the fixtures: `q Q cm w J j d RG rg m l c h re S f B W n sh gs BT ET Tf Td TL Tj ' %comments`. Plus a `circle(cx, cy, r)` helper using κ = 4(√2−1)/3 and 3-decimal formatting exactly like `examples/02-imaging-model/make_imaging_demo.py`.

## Tests (both languages, same cases)

Fixtures live in `fixtures/` (shared, read by both test suites):
- `fixtures/golden/hello.pdf` — from `docs/research/pdf/examples/01-file-structure/hello.py` (CrLf xref EOL).
- `fixtures/golden/imaging-demo.pdf` — from `docs/research/pdf/examples/02-imaging-model/make_imaging_demo.py` (SpaceLf).
- `fixtures/corpus/*.pdf` + `fixtures/corpus/expected.json` — every PDF in the research examples, with facts extracted independently by pypdf: `page_count, encrypted, revisions (count of %%EOF), has_xref_stream, first_page_mediabox, root_keys, header_version, size`.

Required cases:
1. **Golden, byte-identical**: rebuilding `hello.pdf` and `imaging-demo.pdf` with the writer + content builder yields the exact fixture bytes.
2. **Raw writer parity**: rebuild `examples/05-generation-approaches/raw_writer.py`'s document; compare everything except the Flate payload bytes (compare decompressed content instead; its decoded content is in that script).
3. **Corpus**: every corpus file opens; `page_count`, `encrypted`, `first_page_mediabox` and `root_keys` match `expected.json`; `hello-broken.pdf` reports `was_repaired() == true`; `hello-incremental.pdf` has 2 revisions and its page content decodes to text containing `Hello, Update`; `hello-xrefstm.pdf` resolves objects from the object stream.
4. **Round-trip**: write → read → re-serialize each object → identical. Cover all object types, name/string escaping edge cases, deep nesting at the limit, compact mode, incremental update (2 and 3 revisions, newest value wins).
5. **Robustness**: truncation of every corpus file at many offsets, random byte flips, empty input, `%PDF-` only, self-referencing `/Kids`, a ref cycle `1 0 obj 2 0 R endobj 2 0 obj 1 0 R endobj`, nesting depth 10 000, a Flate bomb over the limit → all return errors (or repaired docs), never crash or hang. Rust: these as unit/integration tests (optionally `proptest`); TS: same with a seeded PRNG.
6. **Portability**: Rust — `cargo check --target wasm32-unknown-unknown`, `--target aarch64-apple-ios`, `--target aarch64-linux-android` all pass (scripted). TS — a test that greps `src/` for forbidden APIs (`node:`, `Buffer`, `require(`, `process.`, `fs`), and the built ESM loads in a plain `import()`.
7. **Rust core from TS (WASM)**: the Rust crate exposes a tiny C-ABI WASM surface (alloc/free, `hello_pdf`, `page_count(bytes)`); a TS test loads the `.wasm` with `WebAssembly.instantiate` (no wasm-bindgen) and checks that its `hello_pdf` output is byte-identical to the golden fixture and that `page_count` agrees with the TS reader for every corpus file.

## Layout

```
spikes/pdf-basics/
├── SPEC.md          # this file
├── fixtures/        # shared golden + corpus files
├── rust/            # cargo crate `lamina-pdf-basics`
└── ts/              # npm package `@lamina/pdf-basics-spike` (pnpm)
```

`rust/README.md` and `ts/README.md`: how to build/test, and a results section (test counts, LOC of src, build sizes: rlib/wasm size, minified+gzipped ESM size, any spec questions hit).
