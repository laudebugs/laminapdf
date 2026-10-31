# `@laminapdf/pdf-basics-spike` (TypeScript)

The TypeScript half of the [`pdf-basics` spike](../SPEC.md). It is a minimal PDF core: object model, lexer/parser, tolerant reader with repair, a writer (classic, incremental and compact), Flate with PNG predictors, and a content-stream builder. The Rust twin lives in [`../rust`](../rust). Both run against the same fixtures in [`../fixtures`](../fixtures).

Private, ESM-only and research-quality. The only runtime dependency is [`fflate`](https://github.com/101arrowz/fflate) (MIT).

## Commands

```sh
pnpm install
pnpm build       # tsc → dist/ (ESM + .d.ts)
pnpm test        # build, then vitest run (everything)
pnpm typecheck   # tsc --noEmit over src/, test/, scripts/
pnpm lint        # biome check
pnpm size        # esbuild-minified bundle, min and min+gzip
# External validation: write outputs, then check each with pypdf strict=True
PDF_OUT_DIR=/some/dir PYPDF_PYTHON=/path/to/venv/bin/python pnpm validate
```

`pnpm-workspace.yaml` sets `allowBuilds: { esbuild: false }`. pnpm 11 requires an explicit decision on build scripts, and esbuild runs fine without its postinstall (it picks up the platform binary package).

## Results (2026-10-28, Node 24.19, TypeScript 7.0.2, vitest 5.0.5)

| | |
| --- | --- |
| Tests | **222 passed** (9 files). Two of them (`external.test.ts`) only run when `PDF_OUT_DIR` / `PYPDF_PYTHON` are set; without those it reports 220 passed, 2 skipped. `regressions.test.ts` holds one test per code-review finding, each confirmed to fail on the pre-fix code |
| Typecheck / lint | clean (`strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`; biome recommended) |
| `src/` size | 2,277 lines in 12 files (1,889 excluding comments and blank lines) |
| Bundle (esbuild `--minify`, ESM) | **16.4 KiB min+gz** with fflate inlined (43.2 KiB min). 11.6 KiB min+gz for the library alone (33.4 KiB min) |
| WASM interop (§7) | **ran, 17/17 pass.** Rust `hello_pdf()` is byte-identical to `golden/hello.pdf`, and `page_count` agrees with the TS reader on all 13 corpus files |
| pypdf `strict=True` | all 8 outputs read with the expected page count and text: compact, compact + Flate streams, compact re-pack of an xref-stream (Predictor 12) file using its own trailer, incremental 2-rev and 3-rev, incremental on top of a compact file, hello, imaging-demo |
| Wall time | about 2 s for the full suite (the robustness suite opens about 13k mutated inputs) |

### Coverage of the SPEC test cases

| SPEC case | File | Notes |
| --- | --- | --- |
| 1. Golden, byte-identical | `golden.test.ts` | `hello.pdf` and `imaging-demo.pdf`. Bonus: `hello-incremental.pdf` is rebuilt byte-for-byte from `hello.pdf` with `writeIncremental` |
| 2. Raw writer parity | `golden.test.ts` | Same objects, dicts and trailer. The Flate payload is compared decoded. The byte layout outside the payload is identical once `/Length` and the offsets are normalised |
| 3. Corpus | `corpus.test.ts` | All `expected.json` facts are checked, including `revisions`, `has_xref_stream`, `size` and `header_version`. `hello-broken` is repaired. `hello-incremental` has 2 revisions and reads "Hello, Update". `hello-xrefstm` resolves objects 1–4 from the ObjStm |
| 4. Round-trip | `roundtrip.test.ts` | Covers all object types and escaping edge cases (all 256 bytes in names, strings and streams), in classic CrLf, classic SpaceLf and compact modes. Also: depth 256 passes and 257 is rejected, the writer can Flate-encode streams, and incremental updates with 2 and 3 revisions, where the newest wins, over classic and compact bases |
| 5. Robustness | `robustness.test.ts` | Seeded PRNG (mulberry32). Each corpus file is truncated at about 700 offsets, with every offset in the last 64 bytes, and gets 150 random byte-flip mutants. Also 300 garbage buffers, empty input, `%PDF-`, self-referencing `/Kids`, an indirect page-tree loop, the SPEC's ref cycle, depth 10,000 (arrays and dicts), a 65 MiB Flate bomb (default limit and configured limit), a `/Prev` loop, and a self-referential `/Length`. Each input has a 2 s time budget. Only `PdfError` may escape |
| 6. Portability | `portability.test.ts` | Greps `src/` for `node:`, `Buffer`, `require(`, `process.` and `fs`. Checks that `src/` imports only relative modules and fflate. `import()`s the built `dist/index.js` and uses it. The build config also compiles `src/` with `lib: ES2022` and `types: []`, so using a host API is a compile error |
| 7. Rust via WASM | `wasm.test.ts` | Plain `WebAssembly.instantiate` with no imports. Uses `describe.skipIf` with a clear message when the `.wasm` file is missing. Adds a differential fuzz (next section) |

### Differential fuzz: TS reader vs Rust WASM (informational)

The test feeds 4,305 truncated or byte-mutated corpus inputs to both readers. The Rust build never trapped (a trap is what a panic looks like through WASM). **4,294 of 4,305 agree** (up from 3,783 in the first run and 4,274 before the code-review alignments). The 11 remaining disagreements have two causes:

- **Rust does not repair when `/Root` fails to resolve (10 cases, TS 1 / Rust -1).** The xref loads and its in-file entries point at the right headers, but the catalog is unreadable: a `/Prev` damaged out of an incremental trailer (the newest section lacks object 1), an xref stream whose `/Filter` key or Flate data is damaged (garbage rows, so `/Root` lands nowhere), an ObjStm that no longer decodes, or a catalog dict with a syntax error (`signed.pdf`/`tampered.pdf`, where repair recovers the older revision's catalog). TS treats "`/Root` doesn't resolve to a dictionary" as a repair trigger; Rust's `open_with` only checks that `/Root` is present. **Rust gap**: it should add the same check.
- **fflate is laxer than miniz (1 case, `pdflib.pdf` byte 514, TS 0 / Rust -1).** A flipped byte in the ObjStm's Flate data makes the Huffman code tables invalid. fflate doesn't validate code completeness and decodes 410 bytes of garbage (CPython's zlib rejects it with "invalid literal/lengths set"), so TS gets a damaged `/Pages` dict and returns 0 pages, while miniz fails and Rust's catalog lookup errors. This can't be fixed without replacing the inflater, so it is left as a known divergence.

## Design notes

- **Object model.** `PdfObject` is a discriminated union on `kind`: `null | bool | int | real | string | name | array | dict | stream | ref`.
  - Names and dict keys are *binary strings* (one char per byte, no slash). This makes them usable as `Map` keys and keeps `#xx` bytes exact.
  - Dicts are `Map`s, so they keep insertion order.
  - Strings keep their bytes and remember whether they were hex.
- **Integers** are JS numbers. An integer token outside ±(2^53 − 1) degrades to a `real` (like Rust's `integer_overflow_degrades_to_real`). Remaining divergence: Rust keeps integers up to the `i64` range exact, so tokens between 2^53 and 2^63 are `int` in Rust and `real` in TS.
- **Number formatting.** `formatFixed(x, d)` reproduces Python's `f"{x:.{d}f}"`: it rounds the exact binary value half-to-even, using BigInt arithmetic on the IEEE bits.
  - `Number#toFixed` would be wrong on exact ties: `0.125.toFixed(2)` gives `"0.13"`, Python gives `"0.12"`.
  - It was cross-checked against CPython on 18,266 random values with 0 mismatches.
  - `formatReal` (the §1 rule) is `formatFixed(x, 6)` with trailing zeros trimmed and `-0` mapped to `0`.
- **Errors.** There is one error class, `PdfError`, with a `code` (`Syntax`, `UnexpectedEof`, `DepthLimit`, `Cycle`, `Xref`, `Filter`, `UnsupportedFilter`, `LimitExceeded`, `Invalid`) and an optional byte `offset`. Internal "try" paths catch only `PdfError` and rethrow anything else, so the robustness tests expose real bugs instead of hiding them.
- **Reader pipeline:**
  1. `startxref` → classic table or xref stream. The `/Prev` chain is followed with a visited set; the newest section wins per number. Within one section a later entry for the same number wins, and unknown xref-stream row types are free (so they shadow older revisions). In a hybrid file the table's in-use entries win and `/XRefStm` fills numbers the table omits or marks free (the usual way compressed objects are hidden from pre-1.5 readers).
  2. Every in-use offset entry is then validated: it must land on its own `n g obj`.
  3. Any failure, a missing `/Root`, or a `/Root` that isn't a dict triggers **repair**:
     - scan for the `obj` keyword and walk back over `num gen`
     - parse the candidates in file order, ignoring candidates that fall inside an object already parsed (e.g. `1 0 obj` text in stream data); later definitions win
     - take trailers from the last 32 `trailer` keywords and from `/Type /XRef` streams, sort them all by file offset, and use the newest with a resolvable `/Root` (so a classic incremental trailer after an xref stream wins)
     - add ObjStm members as type-2 entries
     - if no trailer is found, use any `/Type /Catalog`
- **Linear-time guarantees** against hostile inputs:
  - Each object parse is bounded by the next known object offset: the next xref offset, the next scan candidate, or the next ObjStm member offset. Stream data is not bounded.
  - `endstream` positions are indexed once per buffer.
  - The trailer scan is capped at 32 candidates.
  - Flate is fed in 1 KiB chunks, so a bomb overshoots the 64 MiB cap by at most about 1 MiB before `LimitExceeded`. A truncated Flate stream returns what was decoded (an error only if nothing was), as in Rust.
  - PNG predictor parameters are range-checked (`/Colors` 1–32, `/BitsPerComponent` 1/2/4/8/16, `/Columns` 1–2^24) and a row longer than the stream data is rejected before anything is allocated. A short final row is decoded as far as it goes.
  - The page tree is walked with an explicit stack and a visited set.
  - Parser recursion is capped at 256.
- **Page tree.** `/MediaBox`, `/CropBox`, `/Rotate` and `/Resources` are inherited. Nodes and attributes that are broken or missing are read as `null` and skipped, including the root: a catalog without `/Pages`, or with a `/Pages` that doesn't load as a dict, has 0 pages (as in Rust). A missing or non-dict `/Root` catalog is still an error, and reference cycles always throw `Cycle`.
- **Writer.** `PdfWriter` (`alloc`/`add`/`set`/`toBytes`) or `writePdf(objects, trailer, opts)`.
  - Objects are written in ascending number order. Classic mode writes a single subsection `0..Size`, and gaps become a linked free list.
  - `/Size` is computed. It is overwritten in place if the caller gave it, otherwise written first, which matches the fixtures.
  - Streams get `/Length` per the SPEC rule. `compressStreams` adds `/Filter /FlateDecode` to unfiltered streams.
  - `raw(bytes)` emits a pre-serialized body verbatim (see the first spec issue below).
- **Compact mode.** Every gen-0, non-stream, non-raw object goes into one ObjStm (`n off` pairs, then each object + `\n`, Flate). The ObjStm is numbered `max+1` and the xref stream `max+2`.
  - The xref stream has `/W [1 4 2] /Index [0 Size]`, is Flate-encoded without a predictor, and carries the trailer keys minus xref-stream/chaining keys (`/Type /W /Index /Filter /DecodeParms /DP /Length /XRefStm /F /FFilter /FDecodeParms /DL /Prev /Size`), so an xref-stream file's own trailer can be passed straight back in.
  - Header versions below 1.5 are bumped to 1.5.
- **Incremental.** `writeIncremental(original, objects, {xrefEol, compressStreams, trailer})`.
  - It opens the original to get the trailer and `startxref`, and appends the new objects plus `xref` (entry `0 1` and one subsection per run of consecutive numbers) plus a trailer.
  - The trailer repeats the previous keys minus `/Prev`, `/XRefStm` and xref-stream-only keys, updates `/Size`, merges caller entries, and appends `/Prev`.
  - It works on top of xref-stream files (pypdf strict accepts the result).
  - It refuses (`Xref`) a document that needed repair, because `/Prev` would have nothing valid to point at.
- **Content builder.** Fluent methods named after the operators (`q Q cm w J j d RG rg m l c h re S f B W n sh gs BT ET Tf Td TL Tj`, `quote()` for `'`, `comment()`).
  - Operands are `number` (the real rule) or `fixed(v, digits)` (an explicit precision).
  - `inline(b => …)` puts several operators on one line.
  - Output is lines joined by `\n`, with an optional trailing `\n`.
  - `circle()` copies the Python fixture exactly, including how κ is computed.
- **Portability.** `src/` uses only ECMAScript built-ins. It doesn't use even `TextEncoder`/`TextDecoder`: PDF syntax is 8-bit, so latin1 conversion is hand-rolled.
  - The one modern feature is **BigInt**, in `formatFixed`. It is fine on Hermes ≥ 0.70 / React Native ≥ 0.70, but worth knowing if older Hermes builds matter.

## Spec ambiguities and issues found

1. **`hello.pdf` breaks the "single spaces" dict rule.** Its page dict is hand-formatted across three lines (`…[0 0 612 792]\n   /Resources …\n   /Contents 5 0 R >>`). No serializer that follows §1 can produce it. The TS writer adds a `raw(bytes)` body that is written verbatim; Rust needs the same escape hatch or a parsed-object-with-layout mode. The SPEC should say so.
2. **Content formatting varies between fixtures.** Reproducing them needs things §4 doesn't mention:
   - explicit per-operand precision: `0.20 0.40 0.80 rg`, and `cm` with four 6-digit operands followed by plain `0 0`
   - several operators per line: `W n`, and the raw_writer lines
   - a final newline in imaging-demo but not in hello
3. **The SPEC doesn't say how to round reals in §1.** I chose half-even on the exact binary value, matching Python's `.6f` and the content helpers, and believe Rust's `{:.6}` agrees. Rust should confirm, because the two outputs only diverge on exact binary ties like `0.0000005`-scale values.
4. **raw_writer's `<< /Length N /Filter /FlateDecode >>` order.** It only comes out under the "keep `/Length` where the caller put it" rule if the caller places a `/Length` placeholder first. zlib and fflate payloads differ, so excluding the payload from case 2 is the right call.
5. **Things the SPEC leaves undefined**, and what TS does:

   | Undefined in the SPEC | TS choice |
   | --- | --- |
   | Empty dict serialization | `<< >>` |
   | `/Size` placement in the trailer | first if absent |
   | Free entries for numbering gaps | linked list, gen 0 |
   | What "depth 256" means | 256 nested containers OK, 257 rejected |
   | `revisions()` after repair | number of trailer candidates found, min 1 |
   | Hybrid `/XRefStm` precedence | table in-use entries win; `/XRefStm` fills free/absent ones (same as Rust) |
6. **Repair triggers differ between the twins.** TS also repairs when the loaded xref's `/Root` doesn't resolve to a dict; Rust doesn't. See the differential section.
7. **The case 6 grep is blunt.** `node:` also matches object keys like `{ node: x }`; I had to rename a variable in the page walker. `fs` is matched as a whole word. A narrower pattern for import specifiers (`from "node:`) would avoid false positives. The compile-time guard (`types: []`, `lib: ES2022`) is the stronger check anyway.
8. **`expected.json` has no `revisions` definition for repaired files.** Its `revisions` is "count of `%%EOF`", which matches the `/Prev` chain length for every corpus file. They could diverge for files with updates appended without a new xref.
