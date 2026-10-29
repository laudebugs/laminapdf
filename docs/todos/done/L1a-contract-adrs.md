# L1a — Contract ADRs 0026 (errors), 0027 (coordinates and units), 0028 (façade rules, jobs, cancellation)

| Field | Value |
| --- | --- |
| ID | `L1a` |
| Status | `done` |
| Plan | [shareable-libraries §7 L1](../../plans/shareable-libraries.md#l1-contracts), [§4 principles](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031), [§6 ADRs](../../plans/shareable-libraries.md#6-decisions-to-record-adrs) |
| Round / milestone | Round L1 (shareable libraries) — before B5a/B5c |
| Depends on | None |
| Blocks | [B5a](../ready/B5a-lamina-build-core.md), [L1b](../ready/L1b-error-contract-codegen.md), [L2a](../ready/L2a-worker-client.md), [L3a](../ready/L3a-lamina-extract-reading-order.md), [L6a](../ready/L6a-lamina-ffi-spike.md) |
| ADRs | Writes **0026** (error and diagnostic contract), **0027** (coordinates, units and page indexing), **0028** (façade rules, jobs, cancellation and progress); extends [0003](../../decisions/0003-reader-tolerance-and-limits.md) §2 (codes), fills in [0002](../../decisions/0002-binding-strategy.md) §3 (façade), touches [0005](../../decisions/0005-document-model.md) §7 (threads); informs proposed 0009 (worker RPC, [C2a](../ready/C2a-render-adrs.md)) and 0031 (CLI, [L4a](../ready/L4a-lamina-cli.md)) |
| Branch | `docs/l1a-contract-adrs` |
| PR title | `docs(decisions): ADRs 0026 (errors), 0027 (coordinates and units), 0028 (façade rules, jobs, cancellation)` |
| Size | M (≈ 350–500 lines of docs) |
| PR | — |

## Goal

Write down the contract every lamina surface shares before the first public API exists: one error shape, one coordinate convention, and one set of façade rules with a job model for long work. B5a's builder and B5c's façade are still unwritten. Fixing these rules after `@lamina/pdf` ships would break the TypeScript API, and UniFFI could not bind a façade written only for wasm-bindgen ([report: design the façade to the stricter binding now](../../research/libraries/shareable-libraries.md#design-the-façade-to-the-stricter-binding-now)). This is a docs-only PR: three ADRs, the index and the plan tables.

## Context

- [Plan §4 principles 1–5](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031): the recommendations these ADRs turn into decisions.
- Errors: [report: errors and schemas as a single contract](../../research/libraries/shareable-libraries.md#errors-and-schemas-as-a-single-contract) (rustc-style JSON, `hint`, `docsUrl`), [report: strict types](../../research/libraries/shareable-libraries.md#strict-types-branded-units-discriminated-options-codes-generated-from-one-file) (`LaminaError` sketch), [LLM note (A) inferences](../../research/libraries/notes/llm-api-and-ai-extraction.md#a-what-makes-llmsagents-write-correct-code-against-a-library-api-and-what-lamina-should-do) (`didYouMean`, no reused names).
- Coordinates: [report: coordinates must be an explicit, typed field](../../research/libraries/shareable-libraries.md#coordinates-must-be-an-explicit-typed-field) (top-left points, CropBox, `/Rotate`, `/UserUnit`, `bboxNorm`).
- Façade and jobs: [report: design the façade](../../research/libraries/shareable-libraries.md#design-the-façade-to-the-stricter-binding-now), [report: cancellation and progress](../../research/libraries/shareable-libraries.md#cancellation-and-progress-belong-in-laminas-api-not-the-bindings), [rust-core-bridge note §2](../../research/libraries/notes/rust-core-bridge.md#2-one-façade-for-wasm-bindgen-and-uniffi-patterns-and-pitfalls) (pitfalls: lifetimes, generics, iterators, bytes) and [§5](../../research/libraries/notes/rust-core-bridge.md#5-concurrency-cancellation-progress).
- [ADR 0003 §2](../../decisions/0003-reader-tolerance-and-limits.md#2-warnings-stable-enumerated-codes): kebab-case codes, never renamed or reused; `fixtures/codes.json` is the single list ([§4](../../decisions/0003-reader-tolerance-and-limits.md#4-enforcement)).
- [ADR 0005 §7](../../decisions/0005-document-model.md#7-threads): `Document` is `Send` but `!Sync`. UniFFI objects must be `Send + Sync`, so 0028 must say where the lock lives.
- [Code organisation §5](../../guidelines/code-organization.md#5-error-handling-by-layer) and [TypeScript guideline: errors](../../guidelines/typescript.md#errors): today they name the TS class `PdfError`; the plan and report say `LaminaError`.
- [Portability rules](../../guidelines/code-organization.md#2-boundaries-and-dependency-direction): core crates have no clocks or threads, which shapes the job `Budget`.

## Scope

**In scope**

- `docs/decisions/0026-error-and-diagnostic-contract.md`:
  - The JSON shape every surface emits: `{ code, kind, message, hint?, didYouMean?, location?: { offset?, object?, page? }, docsUrl }`; warnings use the same shape (without `didYouMean`).
  - The `kind` taxonomy (recommendation: `input`, `unsupported`, `limit`, `security`, `usage`, `state`, `cancelled`, `internal`), written so ADR 0031 can map kinds to CLI exit codes.
  - Code rules: kebab-case as in ADR 0003 §2 (the report's `WRONG_PASSWORD`/`DISPOSED` spelling becomes `wrong-password`/`disposed`), never renamed or reused, retired codes reserved, one list in `fixtures/codes.json`.
  - **No document text in `message` or `hint`** (prompt-injection hygiene): only codes, numbers, offsets, object references, limit names and option names. Document text appears only in data fields.
  - `hint` rules: one actionable sentence per code, fixed text with only non-document parameters.
  - `docsUrl` rules: stable `…/errors/<code>` URLs; the domain is open (see Risks).
  - `explain` pages: one per code, covering what happened, why, how to fix and an example. Their source location is decided here and implemented by [L1b](../ready/L1b-error-contract-codegen.md).
  - Per-surface mapping: Rust `code()`/`kind()`/`hint()`/`docs_url()` plus `Serialize`; the TS class name (decide `LaminaError` vs `PdfError`; recommendation: `LaminaError`, with the guidelines updated); Swift/Kotlin error enums; CLI `--json` on stderr.
- `docs/decisions/0027-coordinates-units-and-page-indexing.md`:
  - High-level drawing and all extraction output use **points, top-left origin of the page's visible box, y down**.
  - Low-level APIs keep raw PDF user space in types named as such (`UserSpace…`).
  - Typed units: a Rust `Pt` newtype with `Mm`/`In` constructors and `RectPt` (or `Rect<Space>`); TS branded `Pt` with `pt()`/`mm()`.
  - Which box defines the frame (recommendation: CropBox) and how `/Rotate` and `/UserUnit` apply (recommendation: output in the displayed, rotated frame).
  - Every JSON output carries a `coordinates` block (`origin`, `unit`, `box`, `rotate`, `userUnit`), plus an optional `bboxNorm` in [0, 1].
  - Text placement: whether `y` names the baseline or the top of the line box.
  - Pages: a 0-based `index` in APIs and a 1-based `number` in output; CLI page ranges are 1-based, as in qpdf.
- `docs/decisions/0028-facade-rules-jobs-and-cancellation.md`:
  - UniFFI-compatible façade rules: `Send + Sync` handles behind `Arc`, `&self` methods only, owned handles (`PageRef { doc, index }`) instead of borrowed returns, no generics across the boundary, and a string-payload error enum.
  - Cursors (`next_batch(max)`) instead of iterators, and monomorphised constructors (`open_bytes`, `open_source`) instead of generic `ByteSource` parameters.
  - Where the lock lives: recommendation is a `Mutex` in the binding edge crate (`lamina-ffi` wraps the `Send` document), keeping every core crate, the `lamina` façade included, `Send + !Sync` with no `Mutex` as ADR 0005 §7 requires. The alternative is amending ADR 0005 §7 to a `Sync` core.
  - `Job<T>` with `step(Budget) -> Result<Step<T>, Error>`, `Step::Progress { done, total, phase }`, and progress throttling.
  - `Budget` in **work units** (operators, objects, bytes), not milliseconds, because the core has no clock. Edges translate a time slice into units, or the step loop checks a host clock between steps.
  - `CancelToken` (`Arc<AtomicBool>`, polled in long loops) and the `cancelled` code.
  - No threads in core. Async lives only at the edges: the TS worker client, Swift `async` and Kotlin `suspend`.
- Index rows in `docs/decisions/README.md`. Rows 0026–0028 (Round L1) in [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs). The `CLAUDE.md` list of accepted ADRs.
- The guideline error rows ([code organisation §5](../../guidelines/code-organization.md#5-error-handling-by-layer), [TypeScript: errors](../../guidelines/typescript.md#errors)) updated to point at ADR 0026, and the class renamed if the ADR renames it.

**Out of scope** (and which todo covers it)

- Generating codes, the TS union, the JSON Schema and `explain` text: [L1b](../ready/L1b-error-contract-codegen.md).
- Applying the rules in code: [B5a](../ready/B5a-lamina-build-core.md) (coordinates), [B5c](../ready/B5c-lamina-facade-and-examples.md) (façade), [L2a](../ready/L2a-worker-client.md) (async edge).
- Extraction schema `lamina.extract/1`: ADR 0030 in [L3a](../ready/L3a-lamina-extract-reading-order.md). CLI exit codes: ADR 0031 in [L4a](../ready/L4a-lamina-cli.md). UniFFI pin: ADR 0029 in [L6a](../ready/L6a-lamina-ffi-spike.md).

## Tasks

- [x] Read the plan §4 and §6, the report sections in Context, ADRs 0002, 0003 and 0005, and the two error guidelines
- [x] Confirm on the board and in `docs/decisions/` that 0026–0028 are still free (0020–0023 are reserved by editor and library todos)
- [x] Write ADR 0026 with the [ADR template](../../decisions/README.md) (Context, Decision, Consequences, Alternatives considered):
  - [x] Field table with a Rust, TS and JSON example
  - [x] The `kind` list
  - [x] Code and `hint` rules, and the no-document-text rule with one example of what is not allowed
  - [x] `docsUrl` pattern; where `explain` text lives
  - [x] Mapping per surface; the class name decision
  - [x] Alternatives: free-text errors, numeric codes, per-crate code lists, miette-style diagnostics
- [x] Write ADR 0027:
  - [x] The two coordinate spaces, with a diagram or table showing a rect in both
  - [x] Box, `/Rotate` and `/UserUnit` rules; the JSON `coordinates` block; `bboxNorm`
  - [x] Unit types in Rust and TS; text-baseline rule; page `index`/`number` rule
  - [x] Alternatives: bottom-left everywhere (pdf-lib), ratios (Textract), inches (Azure)
- [x] Write ADR 0028:
  - [x] The façade rule list, each with the UniFFI or wasm-bindgen constraint behind it
  - [x] The lock decision and how it relates to ADR 0005 §7 (amend 0005 in this PR if the decision changes it)
  - [x] `Job`/`Step`/`Budget`/`CancelToken` signatures (illustrative Rust); how TS, Swift, Kotlin and the CLI drive a job
  - [x] The `cancelled` code; progress throttling
  - [x] Alternatives: async core, threads in core, binding-level cancellation, Comlink-style proxies
- [x] Add the three rows to `docs/decisions/README.md` and to architecture §3; update `CLAUDE.md`'s accepted-ADR sentence
- [x] Update [code organisation §5](../../guidelines/code-organization.md#5-error-handling-by-layer) and [TypeScript: errors](../../guidelines/typescript.md#errors) to cite ADR 0026 (rename `PdfError` there if the ADR renames it)
- [x] Tick the L1 ADR to-do in [plan §9](../../plans/shareable-libraries.md#9-to-dos); set the plan's ADR table entries to link the files
- [x] `cargo xtask check docs`

## Acceptance criteria

- [x] Three ADR files exist with Status `Accepted` (or `Proposed` with the owner's reason recorded), dated, in template order
- [x] ADR 0026 defines every field of the error shape, the `kind` list, the code rules and the no-document-text rule. It names the TS class and where `explain` text lives
- [x] ADR 0027 gives an unambiguous answer for: origin, unit, reference box, rotation, `/UserUnit`, text baseline, and page index vs number
- [x] ADR 0028 lists the façade rules B5c's acceptance criteria refer to ([B5c](../ready/B5c-lamina-facade-and-examples.md)), defines `Budget` without a clock in core, and states how it relates to ADR 0005 §7
- [x] The guidelines no longer contradict ADR 0026 on the error class name or shape
- [x] `cargo xtask check docs` passes (links and board)

## Outcome and deviations

- All three ADRs are **Accepted**, dated 2026-10-29, in template order. The owner is asked in the PR to accept each one explicitly; if one is not accepted, its Status goes back to `Proposed` with the reason.
- **0026:** the class is `LaminaError`, and `PdfError` is not used. `explain` pages live in `docs/errors/<code>.md`. Every current code has an initial `kind` (§2). Deviations from the scope's shape: `location.page` became `location.pageIndex` (ADR 0027 §6 bans a bare `page` number), `location.id` (a `LaminaId`) was added, and a **`data`** object was added as the only place document-derived strings may appear. `docsUrl` uses the GitHub-rendered page as its base until the domain is chosen. Three existing variants put document text in their messages; L1b fixes them (recorded in [L1b](../ready/L1b-error-contract-codegen.md)).
- **0027:** page space is the effective CropBox as displayed, top-left, y down, with `/UserUnit` applied; user space is used only in `UserSpace…` types; drawing `y` is the baseline; APIs take an `index`, output carries `index` + `number`. Deviation: concrete `RectPt`/`UserSpaceRect` instead of a generic `Rect<Space>`, because of 0028 F3.
- **0028:** rules F1–F12; the lock lives in the binding edge (`Mutex` in `lamina-ffi`, `Rc<RefCell>` in `lamina-wasm`), and ADR 0005 §7 is unchanged (a cross-reference was added). Deviations: `PageRef` holds a `PageId` rather than an index and is built in the edge crates; jobs take the document on each `step` and check a document revision counter (new codes `job-finished`, `document-changed`). B5c is updated accordingly.
- Follow-ups recorded in [B5a](../ready/B5a-lamina-build-core.md) (unit types, `Rect` rename, `PageFrame`), [B5c](../ready/B5c-lamina-facade-and-examples.md) (revision counter, auto-trait test, job codes), [B6b](../ready/B6b-lamina-pdf-package.md) (ships `LaminaError` directly) and [L1b](../ready/L1b-error-contract-codegen.md) (decisions, document-text fixes, new codes).

## Post-merge checks

None.

## Tests

None (documentation only). `cargo xtask docs-links` checks the links.

## Docs to update

- [x] `docs/decisions/README.md`, [architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs), `CLAUDE.md`
- [x] [shareable-libraries §6 and §9](../../plans/shareable-libraries.md#9-to-dos)
- [x] [code organisation §5](../../guidelines/code-organization.md#5-error-handling-by-layer), [TypeScript: errors](../../guidelines/typescript.md#errors)
- [x] ADR 0005 §7, if 0028 amends it

## Risks and open questions

- **`docsUrl` domain** is not chosen yet ([architecture §8](../../plans/architecture.md#8-open-questions-for-the-team)). Recommendation: fix the path (`/errors/<code>`) now and generate the host from one constant, so the URL changes in one place.
- **Rotated pages:** output in the displayed frame matches viewers and extraction vendors, but content-stream editing needs the unrotated frame. The ADR must say which API sees which frame. The owner decides if the two cases conflict.
- **Lock placement:** an edge-crate `Mutex` serialises all calls on one document. That is fine for UniFFI's model, but it should be measured once B5c exists. Amending ADR 0005 §7 to a `Sync` core is the alternative.
- **`Budget::millis`** in the report's sketch needs a clock that core crates may not use. Recommendation: units in core, time at the edges.
- **Naming churn:** renaming `PdfError` to `LaminaError` touches [B6b](../ready/B6b-lamina-pdf-package.md)'s scope. If B6b merges first, L1b does the rename.
- Owner review: these are hard-to-reverse public contracts. Ask the owner to accept each ADR explicitly in the PR.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings) — run by the reviewer after this PR was opened, not by the implementing agent
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
