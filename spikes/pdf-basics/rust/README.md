# `lamina-pdf-basics` (Rust)

The Rust half of the [`pdf-basics` spike](../SPEC.md): a small, panic-free PDF core
(object model, writer, tolerant reader, content builder) plus a C-ABI WASM shim. The
TypeScript twin in `../ts/` runs the same test cases against the same fixtures.

## Commands

```sh
source "$HOME/.cargo/env"            # if cargo is not on PATH
cargo test                            # unit + integration + doc tests
cargo clippy --all-targets --all-features --locked -- -D warnings
scripts/check-targets.sh              # everything: 3 cross-target checks, wasm build, fmt, clippy, tests
cargo build --release --target wasm32-unknown-unknown
#   -> target/wasm32-unknown-unknown/release/lamina_pdf_basics.wasm (loaded by the TS tests)
cargo run --example emit -- OUT_DIR   # writes classic/compact/incremental outputs for external checks
```

## Results

Toolchain: rustc 1.99.1, edition 2024. Measured 2026-10-28 on aarch64-apple-darwin.

| | |
| --- | --- |
| Tests | **169 passing**: 70 unit, 95 integration (golden 4, raw-writer 9, corpus 15, round-trip 16, robustness 18, policy 30, reader-cache 3), 4 doc tests |
| Clippy | clean with `-D warnings` (`clippy::all` denied, plus `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` in library code) |
| Cross-target `cargo check` | `wasm32-unknown-unknown`, `aarch64-apple-ios`, `aarch64-linux-android`: all pass |
| src LOC (`wc -l src/**/*.rs`) | 3 987 total, of which ~3 360 are library code (including doc comments) and ~620 are unit tests |
| Test LOC | 1 470 (`tests/`) |
| rlib (release) | 1 904 488 bytes (`target/release/liblamina_pdf_basics.rlib`, contains metadata) |
| wasm (release, `opt-level="s"`, LTO, `panic=abort`, stripped) | **171 625 bytes** (74 185 gzipped). `opt-level="z"` saved only ~3 KB in a trial. No `wasm-opt` was run |
| Runtime deps | `miniz_oxide` 0.9.1 (MIT/Zlib/Apache-2.0) → `adler2` (0BSD/MIT/Apache-2.0); `thiserror` 2.0.21 (MIT/Apache-2.0). Dev: `serde_json` (MIT/Apache-2.0) |

External validation: `cargo run --example emit` writes classic, compact (object stream +
xref stream), hello, 2-revision and 3-revision incremental updates, and an incremental
update on top of a compact file. All six open in **pypdf 6.21.3 with `strict=True`**, with
the right page counts and extracted text (e.g. `compact.pdf: 2 ['Compact page one', 'Compact
page two']`, `hello-incremental-3rev.pdf: 1 ['Hello, Update']`). I loaded the WASM module
in Node 24 with plain `WebAssembly.instantiate`: `hello_pdf` is byte-identical to the golden
file, and `page_count` returns 1 for every corpus file and -1 for empty input.

## Layout

| Module | Contents |
| --- | --- |
| `object` | `Object`, `Dict` (insertion-ordered `Vec`), `Name`, `PdfString` (remembers literal/hex form), `Stream`, `ObjRef`; exact serialization, `format_real` |
| `lexer` (private) | tokenizer: whitespace and comments, every literal-string escape, hex strings, `#xx` names, numbers |
| `parser` | `parse_object`, indirect objects, streams (indirect `/Length`, with an `endstream` scan when the length is wrong), depth limit `MAX_DEPTH = 256` |
| `filters` | Flate encode/decode with a size limit, PNG predictors 10–15 (parameters validated, row length bounded by the input) |
| `reader::xref` | classic tables, xref streams, `/Prev` chains (loop-safe), hybrid `/XRefStm` |
| `reader::objstm` | object stream members |
| `reader::repair` | rebuilds the xref by scanning `n g obj`, merges trailers and xref-stream dicts, unpacks object streams, falls back to `/Type /Catalog` |
| `reader` | `open`/`open_with`, `Document` (`trailer`, `catalog`, `resolve`, `object`, `deref`, `pages`, `page_count`, `page_content`, `decode_stream`, `revisions`, `is_encrypted`, `was_repaired`, `has_xref_stream`, `header_version`, `warnings`), `Page` with inherited attributes (shared `Rc` resources), `Resolved` (the result of `deref`), `Warning` |
| `writer` | `Writer` (`add`, `add_raw`, `reserve`/`set`, `finish`, `finish_compact`), `incremental_update`, `XrefEol` |
| `content` | `ContentBuilder` (all operators the fixtures need, `line()` grouping, `circle`), `Num::{Auto, Fixed}` |
| `samples` | `hello()` and `imaging_demo()`: the golden fixtures rebuilt with the API (also used by the WASM shim) |
| `wasm` (`cfg(target_arch = "wasm32")`, plus host unit tests) | `alloc`, `dealloc`, `hello_pdf`, `page_count`: the only `unsafe` code |

## Design notes

- **Bytes in, bytes out.** The library borrows the input (`Document<'a>`) and parses objects
  lazily on `resolve`. It never uses `std::fs`, `net`, `process`, `thread` or clocks, and
  `check-targets.sh` greps for them.
- **Object cache.** Each object is parsed once. `resolve`/`object` return `Rc<Object>`, and a
  `RefCell<HashMap<u32, Rc<Object>>>` (alongside the decoded object-stream cache) hands out the
  same `Rc` on every later lookup. Only successful loads are cached. `deref` returns
  `Resolved<'o>` (the borrowed direct object, or the shared indirect one; it derefs to
  `Object`). The page-tree walk passes inherited attributes down as `Rc<Inherited>` (one
  pointer copy per kid; a new node only where a node sets an attribute), and `Page::resources`
  is an `Rc<Object>` shared by every page that inherits or references the same `/Resources`.
  A generated 1 000-page file sharing one `/Resources n 0 R` opens and runs `page_count` +
  `pages` in ~20 ms in debug (`tests/reader_cache.rs`). The caches use `RefCell`/`Rc`, so
  `Document` is deliberately `!Send + !Sync`: use one per thread.
- **No panics by construction.** No `unwrap`/`expect`/`panic!`/slice indexing in library code
  (clippy-enforced). Every loop over file-supplied counts (xref subsections, `/Index`, `/N`)
  is bounded by the bytes actually present, so a count of 4 000 000 000 costs nothing.
  Recursion only happens in the object parser, which is depth-limited. `#![deny(unsafe_code)]`
  is set crate-wide, and only `wasm.rs` opts out, with `#![expect(unsafe_code, reason = …)]`.
- **Cycles.** `resolve` follows ref→ref chains with a visited list. Object loading keeps an
  "in-flight" stack, so a stream whose `/Length` refers to itself, or to an object inside an
  object stream whose own length needs resolving, cannot recurse forever: the length lookup
  fails, and the parser falls back to scanning for `endstream`. The page-tree walk is
  iterative with a visited set. A node reached twice is `Error::ReferenceCycle`. Other broken
  page-tree nodes are skipped (see the alignment section below).
- **When to repair.** The stored xref is used only if every in-file entry points at a
  matching `num gen obj` header and the trailer's `/Root` resolves to a dictionary (an
  entry table can look valid while the catalog sits in a damaged object stream or has a
  syntax error). Any failure in the chain
  (bad offset, `/Prev` loop, unparsable section) triggers a full rebuild. This is simple and
  catches `hello-broken.pdf`'s single wrong entry.
- **PNG predictor bounds.** `/Colors` must be 1–32, `/BitsPerComponent` 1/2/4/8/16 and
  `/Columns` 1–2^24. The row length is computed in `u64` with checked arithmetic (32 × 16 ×
  2^24 bits overflows a 32-bit `usize` on wasm32) and must not exceed the decoded data length
  + 1, so crafted parameters cannot force a large row-buffer allocation.
- **Error type.** `Error` (thiserror) is `Clone + PartialEq`, so tests assert exact variants,
  for example `Err(Error::DecompressLimit { limit })`.
- **Test-crate lints.** Integration test files `#![allow]` `unwrap_used`/`panic`/`indexing_slicing`
  with a `reason`. Clippy's `allow-*-in-tests` only covers `#[test]` fns and `#[cfg(test)]`
  modules, not shared helpers. `expect` would be brittle there because each test crate uses
  a different subset of `tests/common`.
- **Robustness tests** truncate every corpus file at ~400 offsets, apply 150 seeded random
  multi-byte flips per file (xorshift64*, seed `0x1A317A`), feed 300 random token soups, and
  run every read path on whatever opens. Each scenario also asserts a 30 s wall-clock budget
  as a hang guard. The whole suite runs in under 1 s in debug builds, plus about 4 s the first
  time to compress the 64 MiB Flate bomb. `miniz_oxide` is built at `opt-level=3` in the dev
  profile so this stays fast.

## Spec ambiguities and issues

1. **`hello.pdf` breaks the "single spaces" dict rule.** Object 3 is hand-wrapped
   (`…[0 0 612 792]\n   /Resources …\n   /Contents 5 0 R >>`). No serializer that follows the
   spec can reproduce it. *Resolution:* `Writer::add_raw(bytes)` writes a pre-serialized body
   verbatim. The TS twin needs the same escape hatch, or case 1 cannot pass byte-identically.
2. **Content-stream number formats are per operand.** `imaging-demo.pdf` mixes `0.20 0.40 0.80 rg`
   (2 fixed decimals), `0.866025 0.500000 -0.500000 0.866025 0 0 cm` (6 fixed decimals next to
   default-formatted zeros) and `0 0.6 0.3 rg` (default). *Resolution:* every numeric
   operand is `impl Into<Num>`, so `0.6`/`72` format by default and `fixed(v, n)` formats like
   Python `f"{v:.{n}f}"` (both round correctly on the exact binary value).
3. **Operator separators and trailing newline differ between fixtures.** `hello` and `raw_writer`
   join operators with `\n` and have no trailing newline. `raw_writer` puts several operators on
   one line separated by spaces. `imaging-demo` ends with `\n`, which gives a blank line before
   `endstream`. *Resolution:* `\n` join semantics, `line(|l| …)` for space-separated groups, and
   an explicit `end_line()`.
4. **`/Length` position in `raw.pdf`.** It writes `<< /Length 200 /Filter /FlateDecode >>`. With
   "Length is written last if absent", a Flate stream would come out as `/Filter … /Length`.
   *Resolution:* the caller puts `/Length` first (the writer keeps its position and fixes the
   value). `Stream::flate` sets `/Filter` and keeps any existing position.
5. **Raw-writer parity covers more than the payload.** miniz_oxide's output differs from
   CPython zlib, so `/Length`, obj 7's xref offset and `startxref` differ too. Also, for tiny
   inputs miniz at level 6 emits a *stored* deflate block, which is valid but not compressed.
   The test compares the bytes before obj 6 exactly, the obj 6 dict without `/Length`, the
   decoded content (equal to the script's string), obj 7 and the trailer exactly, and the tail
   with xref offsets masked.
6. **Incremental update and the "repeat all previous trailer keys" rule.** This is wrong when
   the previous section is an xref stream. A classic trailer must not repeat `/Type /W /Index
   /Filter /DecodeParms /Length` (or a stale `/XRefStm`), so those are dropped. `/Size` keeps
   its position and becomes `max(old, max_new + 1)`. `/Prev` goes last, which matches
   `hello-incremental.pdf`. The `0 1` free-list-head subsection that the fixture writes is
   always emitted, and contiguous numbers merge into one subsection. The update reproduces
   `corpus/hello-incremental.pdf` **byte for byte**. A repaired base is refused
   (`Error::RepairedBase`) because `/Prev` has nothing valid to point at. The API cannot
   add or change trailer keys (e.g. a new `/Info`). That would need an extra parameter.
7. **Compact mode details are unspecified.** Stream objects (and raw bodies) stay at the top
   level. All other objects go into one ObjStm (`N`/`First`, a single header line, members
   separated by `\n`). The xref stream dict is `/Type /XRef /W [1 4 2] /Size /Index` followed
   by the caller's trailer keys, in the order the SPEC lists them. There is no predictor on
   output. The header is raised to 1.5 when lower.
8. **What `revisions()` means.** expected.json counts `%%EOF` markers. The SPEC counts xref
   sections via `/Prev`. They agree on the whole corpus (signed/tampered = 2). For repaired
   files, I count the trailer and xref-stream dictionaries found by the scan (at least 1).
   Hybrid `/XRefStm` sections are not counted as revisions. For which entry wins in hybrid
   files, see alignment item 4.
9. **Reals that look like integers.** `Real(1.0)` serializes as `1` and reads back as
   `Integer(1)`. The bytes round-trip, but the variant does not. Non-finite reals (which PDF
   cannot represent) serialize as `0`. The SPEC is silent on this.
10. **`resolve` vs. "re-serialize each object".** `resolve` follows a top-level `n g R` body to
    its target. Re-serializing stored objects needs the raw value, so `Document::object(num)`
    returns it unfollowed.
11. **Minor reader choices the SPEC leaves open.** The `%PDF-` header is optional (alignment
    item 2). Raw CR/CRLF inside literal strings is normalized to LF
    (§7.3.4.2); this is round-trip-safe because the writer escapes `\r`. Free or missing objects
    resolve to `null` (§7.3.10). `page_count` counts leaves and does not trust `/Count`.
    Filters other than Flate (e.g. reportlab's `ASCII85Decode`) return `UnsupportedFilter`, so
    only page *content* decoding fails for that file. Page counts and boxes are unaffected.
12. **WASM ABI.** `out_len` in `hello_pdf(out_len_ptr)` is a `usize`, so it is a little-endian
    **u32** on wasm32. The TS side reads it with `DataView.getUint32(ptr, true)`.
    `dealloc(ptr, len)` frees both host-allocated buffers and `hello_pdf` results. Buffers come
    from `std::alloc::alloc` with `Layout::from_size_align(len, 8)`, so they are 8-byte
    aligned and `dealloc` uses the exact same layout. `alloc(0)` returns a non-null dangling
    pointer (freeing it is a no-op). Allocation failure or an oversize `len` returns null.
    `hello_pdf` writes `out_len` with an unaligned store, so any 4 writable bytes will do.

## Alignment with the TS twin (after differential fuzzing)

The TS agent fuzzed its reader against this WASM build: 4 305 mutated corpus inputs, no
traps, and 522 disagreements, all of them policy differences. Both readers now follow the
decisions below. `tests/policy.rs` covers each one, plus unit tests in `xref.rs`,
`content.rs` and `object.rs`.

1. **Missing objects are `null`** (§7.3.10). `pages()` / `page_count()` skip any page-tree
   child that is missing, unparsable or not a dictionary. They also skip a `/Kids` that is not
   an array and a missing `/Pages` root. A tree whose kids are all missing therefore has 0
   pages (WASM `page_count` returns 0, not -1). Reference cycles (a `/Kids` revisit or a
   ref→ref loop) are still `Error::ReferenceCycle`. A missing or non-dictionary `/Root`
   catalog is still an error.
2. **The `%PDF-` header is optional.** It is looked for in the first 1 024 bytes. When it is
   missing, opening goes ahead (xref first, then repair), `header_version()` is `None`, and
   `warnings()` contains `Warning::MissingHeader`. Empty input, and input without any
   `n g obj`, are still `NotPdf`.
3. **Repair is more thorough.** It records every `/Type /XRef` stream dictionary as a trailer
   candidate, with or without `/Root`. A stream whose data or `endstream` is damaged
   still yields its dictionary, because the parser falls back to the leading dict. Members of
   every `/Type /ObjStm` found by the `n g obj` scan are indexed as type-2 entries. The last
   definition of an object number that *parses* wins, so a damaged newer copy does not hide
   an intact older one. `/Root` is the newest candidate whose `/Root` points at a dictionary
   (or a reference, so ref cycles stay `ReferenceCycle` errors). A stored xref whose `/Root`
   does not resolve to a dictionary also triggers repair. After this, the TS differential
   fuzz agrees on 4 304 / 4 305 inputs. The one remaining case (`pdflib.pdf` byte 514 = 105,
   TS 0 pages, Rust -1) is in the object stream's deflate data: miniz_oxide reports invalid
   data where fflate decodes something. If there is none, the
   last `/Type /Catalog` object found, directly or inside an object stream, is used.
4. **Shared conventions.** These already matched or were changed to match: `<< >>` for an
   empty dict. Trailer `/Size` is written first when absent; a caller-supplied `/Size` keeps
   its position, and its value is always recomputed (this changed: previously it was always
   first). The depth limit is 256 (accepted) / 257 (rejected). After repair, `revisions()`
   equals the number of trailers found (minimum 1). The incremental writer refuses repaired
   documents. Fixed-precision formatting rounds half-to-even on the exact binary value, like
   Python `.Nf`: Rust's `{:.N}` gives `0.125`→`0.12`, `0.375`→`0.38`, `2.5`→`2`, verified
   against CPython.
   *Hybrid files:* the classic table's **in-use** entries win over `/XRefStm`, as agreed. I kept
   one refinement: a table entry marked **free** (`f`) is still filled in from the XRefStm.
   Hybrid writers commonly mark compressed objects as free in the table so that pre-1.5
   readers skip them. Letting that free entry win would hide every compressed object in a
   real hybrid file. Unit tests: `hybrid_table_in_use_entry_wins_over_xrefstm`,
   `hybrid_xrefstm_fills_in_entry_the_table_marks_free`.
5. **Integer range differs on purpose.** Rust parses any integer that fits in `i64` as
   `Object::Integer` (e.g. `9007199254740993`) and degrades larger ones to `Real`. The TS twin
   rejects integers outside ±2^53, because JS numbers lose exactness beyond that.
