# Plan: shareable libraries (TypeScript, Rust, mobile, CLI, agents)

- **Status:** Draft
- **Date:** 2026-10-29
- **Research:** [shareable libraries report](../research/libraries/shareable-libraries.md) and its [six notes](../research/libraries/README.md)
- **Builds on:** [PDF library plan](pdf-library.md) (Rounds B1–B9), [architecture plan](architecture.md), ADRs [0001](../decisions/0001-core-language-and-licence.md), [0002](../decisions/0002-binding-strategy.md), [0003](../decisions/0003-reader-tolerance-and-limits.md), [0005](../decisions/0005-document-model.md)–[0007](../decisions/0007-save-planner.md)
- **Proposes:** ADRs 0026–0031 (§6); 0026–0028 accepted by the owner on 2026-10-29
- **Roadmap:** [roadmap.md](roadmap.md)

## 1. Goal

Make lamina's libraries good for three kinds of user at once: frontend and mobile engineers who need smooth UI, backend and command-line users who need composable tools, and LLMs and AI agents that write code against the API and read its output. The [PDF library plan](pdf-library.md) delivers the Rust core, the `lamina` crate and `@lamina/pdf`. This plan adds what that plan leaves out:

1. **One contract** across every surface: errors, coordinates and units, jobs and cancellation, versioned JSON schemas.
2. **TypeScript hardening:** a worker client, disposal and memory bounds, range loading.
3. **AI-ready extraction:** reading order, tables, Markdown and JSON with bounding boxes for citations.
4. **A CLI** (`lamina`) for humans and agents, with a thin MCP mode.
5. **Docs for agents:** `llms.txt`, docs shipped in the packages.
6. **Mobile:** a `lamina-ffi` spike (UniFFI), then a mobile SDK plan.

The research found the opening is real: in October 2026 no permissively licensed library combines tolerant parsing, editing, incremental save, signing and rendering in one engine across web, Node, iOS and Android, and no permissive CLI covers text extraction and rendering ([report §1](../research/libraries/shareable-libraries.md#developers-leave-pdf-tools-over-stalls-malformed-files-and-licences)).

## 2. Scope and non-goals

**In scope:** the six areas above, as rounds L1–L6 (§7).

**Non-goals (for now)**

- Building the mobile SDKs themselves. L6 ends with a spike and a plan; the SDK rounds come from that plan.
- Replacing ADR 0002's bindings. wasm-bindgen + tsify stays for the web; UniFFI stays the mobile path. The third-party `uniffi-bindgen-react-native` `wasm2` path is re-evaluated with the Component Model in 2027 ([report: open questions](../research/libraries/shareable-libraries.md#open-questions-where-research-presses-on-accepted-decisions)).
- Shipping ML model weights in core crates. Layout and OCR models are optional plug-ins (L3c, [E13](full-editor.md#e13-ocr-and-conversion)).
- napi-rs, Python and React Native/Flutter bindings (deferred by ADR 0002).

## 3. Users

| User | What they need from lamina | Served by |
| --- | --- | --- |
| Frontend engineer | No main-thread jank, small bundles, typed API, no leaks | L2 worker client, L1 types, B6 size budgets |
| Mobile engineer | Drop-in SwiftUI/Compose views, bounded memory, stylus ink, idiomatic async | L6 spike and plan |
| Backend / edge developer | Same code in Node, Deno, Bun, Workers; streaming; deterministic output | B6c runtime matrix, L2c range loading |
| Shell user, CI pipeline | Pipes, `--json`, predictable exit codes, batch | L4 CLI |
| LLM writing code | Guessable names, explicit units, errors with hints, current docs | L1 contract, L5 agent docs |
| AI agent reading PDFs | Markdown/JSON with reading order, tables, bounding boxes, hidden-text flags | L3 extraction, L4 CLI, L4c MCP |

## 4. Principles (recommendations, recorded in ADRs 0026–0031)

1. **Design the façade to UniFFI's rules now** ([report §2](../research/libraries/shareable-libraries.md#design-the-façade-to-the-stricter-binding-now)): handles that a binding can make `Send + Sync` (UniFFI needs this). The `lamina` façade is a core crate, and ADR 0005 §7 allows no `Mutex` in core, so [ADR 0028](../decisions/0028-facade-rules-jobs-and-cancellation.md) §2 puts the lock in the binding edge crate (`lamina-ffi` wraps the `Send` document in a `Mutex`; ADR 0005 unchanged) rather than making the core `Sync`. Also: `&self`-friendly methods, owned handles instead of borrowed returns, no generics across the boundary, a string-payload error enum, an explicit `CancelToken`, resumable `Job::step`. UniFFI forbids `&mut self`; fixing this after `@lamina/pdf` ships would break the TypeScript API.
2. **One error shape everywhere:** `{ code, kind, message, hint, location, docsUrl }`, kebab-case codes as today (ADR 0003 §2), generated from [`fixtures/codes.json`](../../fixtures/codes.json) into Rust, TypeScript and JSON Schema. Messages never quote document text (prompt-injection hygiene).
3. **Explicit coordinates:** high-level and extraction APIs use **top-left origin, points**, with typed units (`Pt`, `Mm`) and an explicit `coordinates` block in every JSON output; low-level APIs keep raw PDF user space, named as such. Pages: 0-based `index` plus 1-based `number` in output.
4. **The core stays synchronous.** Long work is a `Job` the caller steps with a budget counted in work units (core crates have no clock; time limits live at the edges); cancellation and progress are lamina API, not binding features (UniFFI has no built-in cancellation). Async lives at the edges: the TS worker client, Swift `async`, Kotlin `suspend`.
5. **Versioned schemas and deterministic output:** every JSON output carries a schema id (`lamina.extract/1`, `lamina.info/1`); new keys may appear within a version, consumers ignore unknown keys (qpdf's contract).
6. **Separate binaries for heavy features:** wasm code cannot be tree-shaken, so render, extract and sign are separate `.wasm` modules with their own budgets; the 350 KB gz budget covers the base module only.
7. **Rules in core, models as plug-ins:** extraction is rule-based and dependency-light; layout and OCR models implement traits in optional edge crates with licence-clean weights.

## 5. Design summary

The [report](../research/libraries/shareable-libraries.md) has side-by-side Rust, TypeScript, CLI, Swift and Kotlin sketches for each area. All snippets are illustrative, not final.

```rust
// lamina façade (B5c, ADR 0028): Send + !Sync; lamina-ffi wraps it in a Mutex, lamina-wasm in Rc<RefCell<…>>
let doc = lamina::Document::open_bytes(bytes, OpenOptions::default())?;
let mut job = doc.extract_job(&ExtractOptions { format: Format::Markdown, ..Default::default() })?;
let cancel = job.cancel_token();                       // clone it to another thread or the UI to cancel
let out = loop {
    // the document is passed to every step; budget in work units, since core crates have no clock (ADR 0028 §3–§4)
    match job.step(&doc, Budget::units(10_000))? {
        Step::Done(out) => break out,
        Step::Progress { done, total, phase } => report(done, total, phase),
        _ => {}                                         // Step is #[non_exhaustive]
    }
};
// or run to completion: doc.extract(&options, &cancel)?
```

```ts
// @lamina/pdf/worker (L2a): the async twin of the sync API, in a Web Worker
import { connect } from "@lamina/pdf/worker";
const pdf = await connect({ signal });
await using doc = await pdf.open(bytes, { transfer: true }); // zero-copy; the default copies. Symbol.asyncDispose
const md = await doc.extract({ format: "markdown", pages: "1-5" });
```

```sh
# lamina CLI (L4): pipe-friendly, JSON for agents, exit codes 0 ok / 1 failure / 2 usage / 3 warnings / 4 needs --force / 5 password
cat invoice.pdf | lamina extract - --format md > invoice.md
lamina info report.pdf --json | jq '.pages'
lamina pages merge a.pdf b.pdf:1-z:odd -o out.pdf
lamina mcp --root ./docs                               # stdio MCP server over the same commands; read-only unless --allow-write
```

## 6. Decisions to record (ADRs)

Numbers 0020–0023 are reserved by editor and library todos, and 0024–0025 by the [UI/UX plan](ui-ux.md) (app shell UX, mobile shell); this plan proposes 0026–0031.

| # | ADR | Written in | Relationship |
| --- | --- | --- | --- |
| [0026](../decisions/0026-error-and-diagnostic-contract.md) (Accepted, owner 2026-10-29) | Error and diagnostic contract: JSON shape, `hint`/`docsUrl` rules, no document text in messages, `explain` pages | [L1a](../todos/done/L1a-contract-adrs.md) | Extends ADR 0003's codes |
| [0027](../decisions/0027-coordinates-units-and-page-indexing.md) (Accepted, owner 2026-10-29) | Coordinates, units and page indexing in public APIs | [L1a](../todos/done/L1a-contract-adrs.md) | New; constrains B5 builder and B9 extraction |
| [0028](../decisions/0028-facade-rules-jobs-and-cancellation.md) (Accepted, owner 2026-10-29) | Façade rules, jobs, cancellation and progress across bindings | [L1a](../todos/done/L1a-contract-adrs.md) | Fills in ADR 0002 §3; informs proposed ADR 0009 |
| 0029 | Mobile bindings and packaging: UniFFI pin, idiomatic Swift/Kotlin layer, SPM only, AAR layout | [L6a](../todos/ready/L6a-lamina-ffi-spike.md) | Fills in ADR 0002 §5; does not change it |
| 0030 | Extraction output (`lamina.extract/1`) and model plug-ins | [L3a](../todos/ready/L3a-lamina-extract-reading-order.md) | Follows ADR 0001 and proposed ADR 0023 |
| 0031 | CLI and MCP contract: grammar, exit codes, JSON/NDJSON versioning, MCP security defaults | [L4a](../todos/ready/L4a-lamina-cli.md) | New; reuses ADR 0026 errors |

## 7. Rounds

Each round is split into todos (one PR each) on the [todo board](../todos/README.md); their **Depends on** fields fix the order. L1 should land before the B5 builder so the contract shapes the first public API.

### L1. Contracts

**Todos:** [L1a](../todos/done/L1a-contract-adrs.md) (ADRs 0026, 0027, 0028), [L1c](../todos/done/L1c-error-contract-core.md) (kinds, hints and tagged locations in the Rust core, before B5c), [L1b](../todos/ready/L1b-error-contract-codegen.md) (error codegen, `explain`).

**Deliverable:** three ADRs; codes and error types generated from `fixtures/codes.json` into Rust, TypeScript and JSON Schema; `explain` text per code.
**Accepted when:** B5a's builder uses ADR 0027's coordinates; B5c's façade meets ADR 0028's rules; a TS test narrows the error class's `code` exhaustively (the class is `LaminaError`, [ADR 0026](../decisions/0026-error-and-diagnostic-contract.md) §7); a change to `codes.json` without regenerating fails CI.

### L2. TypeScript hardening

**Todos:** [L2a](../todos/ready/L2a-worker-client.md) (`@lamina/pdf/worker`), [L2b](../todos/ready/L2b-disposal-and-memory.md) (disposal and memory bounds), [L2c](../todos/ready/L2c-range-byte-source.md) (range `ByteSource` over `Error::DataPending`).

**Deliverable:** a worker client with the same method names as the sync API, version handshake, `AbortSignal` and transfer lists, which `@lamina/viewer` (C3a) consumes; `Symbol.dispose` on every handle, a capped wasm memory and an abort-and-reinstantiate wrapper; range reading for large and remote files.
**Accepted when:** a 50 MB file opens in a worker with no long task over 50 ms on the main thread; opening and disposing a document in a loop keeps wasm memory at its post-warm-up size (± one 64 KiB page; linear memory never shrinks, so the high-water mark is the baseline); opening a remote file fetches only the ranges it needs.

### L3. AI-ready extraction

**Todos:** [L3a](../todos/ready/L3a-lamina-extract-reading-order.md) (ADR 0030, `lamina-extract` blocks and reading order), [L3b](../todos/ready/L3b-extract-tables-and-serialisers.md) (tables, Markdown/JSON/chunks, bindings), [L3c](../todos/ready/L3c-model-plugins.md) (model plug-in traits).

**Deliverable:** `lamina-extract` (core): reading order from the structure tree, else XY-Cut++; headings, lists, header/footer removal, de-hyphenation, hidden-text flags; lattice tables from ruling lines; Markdown, JSON and chunk output in the `lamina.extract/1` schema with page, bbox and structure path per block; `LayoutModel` and `OcrEngine` traits.
**Accepted when:** reading-order and table scores are measured on a public benchmark subset and recorded; every block carries a bbox in top-left points; hidden text never appears in Markdown and is flagged in JSON.

### L4. CLI and MCP

**Todos:** [L4a](../todos/ready/L4a-lamina-cli.md) (ADR 0031, `lamina-cli` core commands), [L4b](../todos/ready/L4b-cli-batch-render-schema.md) (batch, render, extract, schema, completions), [L4c](../todos/ready/L4c-mcp-mode.md) (`lamina mcp`), [L4d](../todos/ready/L4d-cli-distribution.md) (distribution).

**Deliverable:** the `lamina` binary (`crates/lamina-cli`, layer `app`): `info`, `check`, `pages`, `extract`, `render`, `schema`, `explain`; qpdf-style page ranges; stdin/stdout `-`; `--json` and NDJSON for batch; exit codes 0–5; `--jobs`; completions and man pages; an MCP stdio mode with `--root`, read-only by default (`--allow-write` to opt in); release binaries, Homebrew and an npm wrapper.
**Accepted when:** every command has `--json` output validated against its schema in tests; exit codes are tested per case; the MCP mode passes the MCP inspector's conformance checks and refuses paths outside `--root`.

### L5. Docs for agents

**Todos:** [L5a](../todos/ready/L5a-agent-docs.md).

**Deliverable:** `llms.txt` and `llms-full.txt` on the docs site, a `.md` twin of every page, docs shipped inside the npm package and the crate, an `AGENTS.md` snippet for users' repositories, a machine-readable rename log.
**Accepted when:** the docs build produces the files; the npm tarball contains version-matched docs; an agent eval (a fixed task list run against the examples) is recorded as a baseline.

### L6. Mobile

**Todos:** [L6a](../todos/ready/L6a-lamina-ffi-spike.md) (`lamina-ffi` spike, ADR 0029), [L6b](../todos/ready/L6b-mobile-sdk-plan.md) (mobile SDK plan).

**Deliverable:** a spike that binds the B5c façade with UniFFI, builds an XCFramework and AARs, runs open/extract/save from Swift and Kotlin tests, and reports per-ABI sizes; then a mobile plan (`LaminaView` for SwiftUI, `LaminaViewer` for Compose; tiles, memory trimming, stylus to `/Ink`, accessibility, SPM and Maven Central) with its own rounds and todos.
**Accepted when:** the spike README answers "does the façade bind without changes?" with measurements; the plan is reviewed and its todos are on the board.

## 8. Changes to existing todos

This plan adjusts these todos (edited in the same PR):

- [B5a](../todos/ongoing/B5a-lamina-build-core.md) depends on L1a: the builder's coordinates follow ADR 0027.
- [B5c](../todos/ready/B5c-lamina-facade-and-examples.md): acceptance criteria add ADR 0028's façade rules.
- [B6a](../todos/ready/B6a-lamina-wasm-and-adr-0016.md): ADR 0016 states per-module budgets (base, text, extract, render, sign) and adds twiggy diffs.
- [B6b](../todos/ready/B6b-lamina-pdf-package.md): per-condition wasm loaders and the `init({ module })` escape hatch.
- [C3a](../todos/ready/C3a-viewer-worker-rpc.md) depends on L2a: the viewer reuses `@lamina/pdf/worker` instead of a second protocol.
- [C2a](../todos/ready/C2a-render-adrs.md): ADR 0004 picks the render backend per surface (desktop, web, edge, mobile) using hayro's measured wasm size.
- [L3a](../todos/ready/L3a-lamina-extract-reading-order.md) depends on [U2b](../todos/ready/U2b-reading-order-ground-truth.md) (UI/UX plan): reading order is measured on its ground-truth corpus, and the reader's copy and reader mode reuse `lamina-extract`'s ordering.
- [L6a](../todos/ready/L6a-lamina-ffi-spike.md) feeds [U3b](../todos/ready/U3b-mobile-shell-spike.md) (`mobile-shell` spike, ADR 0025); [L6b](../todos/ready/L6b-mobile-sdk-plan.md) waits for U3b. U3b decides lamina's own mobile apps; L6 is the SDK for other developers, which needs UniFFI either way.
- [E10d](../todos/ready/E10d-pades-signing-core.md): the `Signer` is two-phase (`prepare` returns the digest, `finish` takes the signature) so WebCrypto, the Secure Enclave and HSMs fit a synchronous core.

## 9. To-dos

### L1. Contracts

- [x] Write ADRs 0026 (errors), 0027 (coordinates and units), 0028 (façade rules, jobs, cancellation)
- [x] Kinds, hints and tagged locations for every code in the Rust core (`codes.json` v2), before B5c
- [ ] Generate Rust, TS and JSON Schema error codes from `fixtures/codes.json`; add `explain` text

### L2. TypeScript hardening

- [ ] `@lamina/pdf/worker`: typed RPC, version handshake, `AbortSignal`, transfer lists
- [ ] `Symbol.dispose` on every handle, capped wasm memory, abort-and-reinstantiate wrapper
- [ ] Range `ByteSource` reporting missing ranges through `Error::DataPending` (ADR 0005 §2), with edge adapters (HTTP range, JS callback, native positioned reads)

### L3. AI-ready extraction

- [ ] Write ADR 0030; create `lamina-extract` with blocks, reading order, headings, lists, header/footer removal
- [ ] Lattice tables; Markdown, JSON and chunk serialisers; expose in Rust, WASM and TS
- [ ] `LayoutModel` and `OcrEngine` traits; optional model edge crate

### L4. CLI and MCP

- [ ] Write ADR 0031; create `crates/lamina-cli` with `info`, `check`, `pages`, `extract` (text), `explain`
- [ ] Batch and NDJSON, `--jobs`, `render`, Markdown/JSON extraction, `schema`, completions, man pages
- [ ] `lamina mcp` (rmcp, stdio), `--root`, read-only by default (`--allow-write`), honest tool annotations
- [ ] Release binaries (cargo-dist), Homebrew tap, npm wrapper with per-platform packages

### L5. Docs for agents

- [ ] `llms.txt`, `.md` twins, docs in packages, `AGENTS.md` snippet, rename log, agent eval baseline

### L6. Mobile

- [ ] `lamina-ffi` spike right after B5c with UniFFI `=0.32.x`, XCFramework and AARs, size report; ADR 0029
- [ ] Mobile SDK plan with rounds and todos

## 10. Risks

| Risk | Mitigation |
| --- | --- |
| Façade rules slow down B5c | Rules are written first (L1a) and are small; the L6a spike checks them early |
| Extraction quality without models disappoints on tables and scans | Publish measured scores; plug-in traits (L3c) for models; say plainly what rules cannot do |
| MCP spec churn (2025-11-25 → 2026-07-28 changed a lot) | MCP is a thin adapter over CLI commands; pin the rmcp version; CLI stays the primary agent interface |
| Prompt injection through document text | Document text only in data fields, never in `message`/`hint`; hidden text flagged; MCP `--root` required and read-only by default |
| UniFFI version churn and missing cancellation | Pin per ADR 0029; lamina's own `CancelToken` (ADR 0028) |
| Mobile platform viewers improve (androidx.pdf) | Compete on fidelity, editing and signature-safe saves, not on viewing alone |

## 11. Open questions

Decided (project owner, 2026-10-29), recorded in the [roadmap](roadmap.md#decisions-made):

- **UniFFI timing:** the L6a spike starts right after B5c, in parallel with B6 (ADR 0002 §5's "UniFFI later" still holds for the SDK itself).
- **UniFFI pin:** `=0.32.x` (zero-copy byte arguments, foreign traits); React Native via ubrn (0.31 only today) is revisited when scheduled. ADR 0029 (L6a) records it.
- **Size budget:** ADR 0016 (B6a) sets a budget per wasm module (base, text, extract, render, sign); 350 KB gz applies to the base module.

Still open:

1. **Render default per surface** (proposed ADR 0004): PDFium (~2.15 MB gz in wasm) for desktop; hayro may suit edge and mobile if C1a's fidelity holds.
2. **Facts to verify before the dependent todos start:** androidx.pdf release state (1.0.0-beta01 vs alpha19), Safari `Symbol.dispose`, whether Workers can compile wasm from bytes, Marker's and Docling's licences, Kotlin cancellation in UniFFI 0.32 ([report: items to verify](../research/libraries/shareable-libraries.md#items-to-verify-before-planning)).
