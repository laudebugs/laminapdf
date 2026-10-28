# `@lamina/pdf-basics-spike` (TypeScript)

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
| Tests | **206 passed** (8 files). Two of them (`external.test.ts`) only run when `PDF_OUT_DIR` / `PYPDF_PYTHON` are set; without those it reports 204 passed, 2 skipped |
| Typecheck / lint | clean (`strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`; biome recommended) |
| `src/` size | 2,242 lines in 12 files (1,878 excluding comments and blank lines) |
| Bundle (esbuild `--minify`, ESM) | **16.3 KiB min+gz** with fflate inlined (42.8 KiB min). 11.5 KiB min+gz for the library alone (33.0 KiB min) |
| WASM interop (§7) | **ran, 17/17 pass.** Rust `hello_pdf()` is byte-identical to `golden/hello.pdf`, and `page_count` agrees with the TS reader on all 13 corpus files |
| pypdf `strict=True` | all 7 outputs read with the expected page count and text: compact, compact + Flate streams, incremental 2-rev and 3-rev, incremental on top of a compact file, hello, imaging-demo |
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

The test feeds 4,305 truncated or byte-mutated corpus inputs to both readers. The Rust build never trapped (a trap is what a panic looks like through WASM). **3,783 of 4,305 agree.** The disagreements are policy differences, not crashes, and both teams should align them:

- **Missing page objects** (mostly truncated files): `/Kids` points at an object that doesn't exist. TS reads that reference as `null`, which is ISO 32000 semantics, skips it and returns 0 pages. Rust returns an error (-1).
- **TS succeeds where Rust fails:**
  - a corrupted `%PDF-` header byte: TS does not require the header, repair finds the objects
  - damaged trailer, `startxref` or xref-stream bytes: TS repair also scans `/Type /XRef` streams and ObjStm members

## Design notes

- **Object model.** `PdfObject` is a discriminated union on `kind`: `null | bool | int | real | string | name | array | dict | stream | ref`.
  - Names and dict keys are *binary strings* (one char per byte, no slash). This makes them usable as `Map` keys and keeps `#xx` bytes exact.
  - Dicts are `Map`s, so they keep insertion order.
  - Strings keep their bytes and remember whether they were hex.
- **Integers** are JS numbers. The lexer rejects integers outside ±2^53 with `Syntax` rather than losing precision silently. (This differs from Rust's `i64`.)
- **Number formatting.** `formatFixed(x, d)` reproduces Python's `f"{x:.{d}f}"`: it rounds the exact binary value half-to-even, using BigInt arithmetic on the IEEE bits.
  - `Number#toFixed` would be wrong on exact ties: `0.125.toFixed(2)` gives `"0.13"`, Python gives `"0.12"`.
  - It was cross-checked against CPython on 18,266 random values with 0 mismatches.
  - `formatReal` (the §1 rule) is `formatFixed(x, 6)` with trailing zeros trimmed and `-0` mapped to `0`.
- **Errors.** There is one error class, `PdfError`, with a `code` (`Syntax`, `UnexpectedEof`, `DepthLimit`, `Cycle`, `Xref`, `Filter`, `UnsupportedFilter`, `LimitExceeded`, `Invalid`) and an optional byte `offset`. Internal "try" paths catch only `PdfError` and rethrow anything else, so the robustness tests expose real bugs instead of hiding them.
- **Reader pipeline:**
  1. `startxref` → classic table or xref stream. The `/Prev` chain is followed with a visited set, and hybrid `/XRefStm` entries fill gaps.
  2. Every in-use offset entry is then validated: it must land on its own `n g obj`.
  3. Any failure, a missing `/Root`, or a `/Root` that isn't a dict triggers **repair**:
     - scan for the `obj` keyword and walk back over `num gen`
     - parse the candidates in file order, ignoring candidates that fall inside an object already parsed (e.g. `1 0 obj` text in stream data); later definitions win
     - take trailers from the last 32 `trailer` keywords and from `/Type /XRef` streams, preferring the newest with a resolvable `/Root`
     - add ObjStm members as type-2 entries
     - if no trailer is found, use any `/Type /Catalog`
- **Linear-time guarantees** against hostile inputs:
  - Each object parse is bounded by the next known object offset: the next xref offset, the next scan candidate, or the next ObjStm member offset. Stream data is not bounded.
  - `endstream` positions are indexed once per buffer.
  - The trailer scan is capped at 32 candidates.
  - Flate is fed in 1 KiB chunks, so a bomb overshoots the 64 MiB cap by at most about 1 MiB before `LimitExceeded`.
  - The page tree is walked with an explicit stack and a visited set.
  - Parser recursion is capped at 256.
- **Page tree.** `/MediaBox`, `/CropBox`, `/Rotate` and `/Resources` are inherited. Below the root, kids and attributes that are broken or missing are read as `null` and skipped. Reference cycles always throw `Cycle`.
- **Writer.** `PdfWriter` (`alloc`/`add`/`set`/`toBytes`) or `writePdf(objects, trailer, opts)`.
  - Objects are written in ascending number order. Classic mode writes a single subsection `0..Size`, and gaps become a linked free list.
  - `/Size` is computed. It is overwritten in place if the caller gave it, otherwise written first, which matches the fixtures.
  - Streams get `/Length` per the SPEC rule. `compressStreams` adds `/Filter /FlateDecode` to unfiltered streams.
  - `raw(bytes)` emits a pre-serialized body verbatim (see the first spec issue below).
- **Compact mode.** Every gen-0, non-stream, non-raw object goes into one ObjStm (`n off` pairs, then each object + `\n`, Flate). The ObjStm is numbered `max+1` and the xref stream `max+2`.
  - The xref stream has `/W [1 4 2] /Index [0 Size]`, is Flate-encoded without a predictor, and carries the trailer keys.
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
   | Hybrid `/XRefStm` precedence | table entries win |
6. **The `/Kids` policy differs between the twins.** A missing or broken kid gives 0 pages in TS and an error in Rust. See the differential section.
7. **The case 6 grep is blunt.** `node:` also matches object keys like `{ node: x }`; I had to rename a variable in the page walker. `fs` is matched as a whole word. A narrower pattern for import specifiers (`from "node:`) would avoid false positives. The compile-time guard (`types: []`, `lib: ES2022`) is the stronger check anyway.
8. **`expected.json` has no `revisions` definition for repaired files.** Its `revisions` is "count of `%%EOF`", which matches the `/Prev` chain length for every corpus file. They could diverge for files with updates appended without a new xref.
