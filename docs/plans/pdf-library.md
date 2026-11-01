# Plan: PDF generation library (`@laminapdf/sdk` and `lamina`)

- **Status:** Draft
- **Date:** 2026-10-28
- **Builds on:** [architecture plan](architecture.md) (Rounds A–B, ADRs 0001–0007, 0015–0019; this plan adds [ADR 0023](../decisions/0023-content-stream-reading.md)) and the [architecture research report](../research/architecture/app-and-package-architecture.md)
- **Research:** [03 Text and fonts](../research/pdf/03-text-and-fonts.md) (§2 text operators, §4–5 encodings and ToUnicode, §6 subsetting, §7 shaping, §9 tagging, §11.2 requirements), [04 §7 encryption](../research/pdf/04-interactive-security.md#7-encryption-76), [05 §8](../research/pdf/05-creating-pdfs-from-code.md#8-implications-for-lamina), [06 §9–10](../research/pdf/06-feature-inventory.md#9-suggested-phasing), [07 spike](../research/pdf/07-spike-rust-and-typescript.md)
- **Conventions:** [code organisation](../guidelines/code-organization.md), [Rust](../guidelines/rust.md), [TypeScript](../guidelines/typescript.md), [git workflow](../guidelines/git-workflow.md)
- **Roadmap:** [roadmap.md](roadmap.md)

## 1. Goal

Ship an importable, permissively licensed library that **generates correct PDFs from code** in TypeScript (`@laminapdf/sdk` on npm) and Rust (`lamina` on crates.io). Both are thin façades over the same Rust core. A generated file comes back as a `Document`, so the same object can later be opened, edited and saved with the API the apps use ("generate first, read and edit later", [architecture §2](architecture.md#2-the-architecture-in-one-picture)).

## 2. Scope and non-goals

**In scope (this plan, Rounds B1–B9)**

- Pages with any size, page boxes and rotation.
- Text with embedded, subset fonts (TrueType/OpenType/CFF, variable fonts instantiated at embed time), shaping, bidi, line breaking, ToUnicode, real space glyphs.
- Images: JPEG pass-through, PNG (with alpha as `/SMask`), raw RGBA from the caller.
- Vector graphics: paths, strokes, fills, dashes, transforms, clipping, opacity, DeviceRGB/DeviceGray/DeviceCMYK and ICC-based colour.
- Links (URI and internal destinations), named destinations, outlines (bookmarks), page labels.
- Metadata: Info dictionary and XMP, document language, viewer preferences.
- Tagged PDF, PDF/UA-1 and PDF/A-2 (b/u) as **goals** reached in Round B8, with conformance profiles that turn violations into build errors.
- The read path that the builder shares: `Document::open` (including files encrypted with the standard security handler, given the password), typed read access, `transact`, `save` (incremental or full).
- Text extraction (B9): positioned glyphs with Unicode and quads, read from content streams, for the readers' text layer, search and copy.
- Runtimes: browser, Node, Deno, Bun, Cloudflare Workers (WASM); native Rust on macOS, Windows, Linux (and iOS/Android compile checks).
- Docs site, examples, publishing to npm and crates.io.

**Non-goals (for now)**

- A flowing layout engine (paragraph boxes across pages, tables, floats). The builder positions text runs and offers single-box line breaking. Layout is a later plan; "Typst as layout backend" is one option ([05 §8.3](../research/pdf/05-creating-pdfs-from-code.md#83-build-versus-wrap-which-existing-work-to-stand-on)).
- HTML/CSS → PDF. Out of scope for the library; see [full-editor §E13](full-editor.md#e13-ocr-and-conversion).
- Rendering in the base package. Rendering ships as the optional `@laminapdf/render-pdfium` package in Round C ([web-app.md](web-app.md)).
- Python, Swift/Kotlin and napi bindings. UniFFI is "later" per ADR 0002; the [shareable libraries plan](shareable-libraries.md) brings forward the façade rules and a `lamina-ffi` spike, and adds the CLI, AI-ready extraction and agent docs.
- Editing operations beyond the proving ops in B3. They arrive with the [editor rounds](full-editor.md) and are exposed in the library at the same time.

## 3. Users and use cases

| User | Use case | Must work on |
| --- | --- | --- |
| Web developer | Invoice/receipt/ticket generated in the browser, no server | Browser, bundlers (Vite, webpack, esbuild) |
| Edge/backend developer | Generate a report in a Cloudflare Worker or a Node/Deno/Bun service | workerd (128 MB isolate), Node ≥ 22, Deno, Bun |
| Rust developer | Native PDF output from a CLI, server or desktop app | `lamina` crate, no C toolchain by default |
| Public sector / regulated orgs | Accessible (PDF/UA) or archival (PDF/A) output by default | Both, with validator-clean output |
| lamina apps | Images → PDF, blank pages, stamps, headers/footers in the editor | Same builder over an open `Document` |

## 4. Requirements

### Functional

1. **Builder API** `Builder → page → surface → finish() → Document` in both languages (ADR 0016 fixes names).
2. **Fonts.** Caller supplies font bytes; the core never reads the file system ([guidelines §2](../guidelines/code-organization.md#2-boundaries-and-dependency-direction)). Default embedding is Type 0 + Identity-H with deterministic subset tags. Standard 14 fonts by name with bundled AFM metrics, rejected under PDF/A and PDF/UA profiles. `OS/2.fsType` respected, with an explicit override ([03 §6.3](../research/pdf/03-text-and-fonts.md#63-licensing-os2fstype)).
3. **Text.** Shaping (HarfRust/rustybuzz), bidi (UAX #9), line breaking (UAX #14) inside a box, OpenType feature control, missing-glyph diagnostics (never silent tofu), automatic ToUnicode and `ActualText` ([03 §11.2](../research/pdf/03-text-and-fonts.md#112-concrete-requirements)).
4. **Images, vectors, colour, links, outlines, metadata** as listed in §2.
5. **Determinism.** Same input → byte-identical output (fixed IDs/dates when supplied), so golden tests work.
6. **Conformance profiles** (B8): `PdfUa1`, `PdfA2b`, `PdfA2u`, `PdfA3b` (associated files). Missing alt text, missing `Lang` or title, content that is neither tagged nor marked as an artifact, a link without a `Link` structure element and `/Contents`, an unembedded font or an unmapped glyph is an error at `finish()`.
7. **Read/edit path.** `Document::open(source, OpenOptions)` (`Limits`, strict mode and an optional password; [ADR 0003](../decisions/0003-reader-tolerance-and-limits.md)) (standard security handler, read only: RC4 and AES-128 for R2–R4, AES-256 for R5/R6, [04 §7.2](../research/pdf/04-interactive-security.md#72-standard-security-handler-revisions)), typed read views (pages, outline, metadata), `transact`, undo, `save({ mode: "auto" | "incremental" | "full" })` with `SaveConstraint` honoured (ADRs 0005–0007).
8. **Text extraction** (B9): per page, glyph runs with Unicode text (ToUnicode first, then encoding and glyph-name fallbacks, [03 §4–5](../research/pdf/03-text-and-fonts.md#4-encodings-965-in-pdf-20--966-in-iso-32000-1-975-annex-d)), font size, writing direction and a quad per glyph in page space; `ActualText` and MCIDs where present.
9. **Errors and warnings.** One error type per crate and one `LaminaError` class with matching string codes in TS ([ADR 0026](../decisions/0026-error-and-diagnostic-contract.md)) ([guidelines §5](../guidelines/code-organization.md#5-error-handling-by-layer)). Tolerant reads report warnings.

### Non-functional

| Requirement | Target (WASM sizes: [ADR 0016](../decisions/0016-package-design.md) §6) |
| --- | --- |
| `@laminapdf/sdk` WASM, gzipped | Per module ([ADR 0016](../decisions/0016-package-design.md) §6, owner 2026-10-30): base ≤ 380 KB (measured 368 KB; the 350 KB of 2026-10-29 revised after measurement), text ≤ 960 KB (measured 937 KB); extract, render and sign get theirs when they exist (spike core: 74 KB; krilla-fit builder stack: 377 KB) |
| JS glue + TS API, min+gz | ≤ 20 KB |
| Rust `lamina` default features | No C dependencies; builds for `wasm32-unknown-unknown`, iOS and Android targets |
| Hello-world generation | ≤ 5 ms after `init()` on a 2023 laptop; a 100-page text report ≤ 500 ms (B5d: 251–422 ms in Node 24 for the B6c report, 658–958 ms before, measured under a load average near 190 on 8 cores; natively 39 ms, 78 ms before; `cargo xtask pdf-runtimes` records it nightly) |
| Memory in a Worker | 100-page report with two embedded fonts under 64 MB peak |
| Output validity | qpdf `--check` clean, opens in pdf.js and PDFium, veraPDF-clean under each profile (B8) |
| Licence | MIT OR Apache-2.0; dependency licences gated by cargo-deny and the npm licence check (ADR 0019) |

## 5. Design summary

The architecture is in [architecture.md](architecture.md); this section lists only what the library adds.

- **Crates** (Round B): `lamina-model`, `lamina-ops`, `lamina-save`, `lamina-text`, `lamina-build`, `lamina-content` (B9, [ADR 0023](../decisions/0023-content-stream-reading.md)), the `lamina` façade and `lamina-wasm`. Decryption lives in `lamina-syntax` (crypt filters) and is wired into `Document::open` in B2. Packages: `@laminapdf/sdk` (TS API, ESM-only) over `@laminapdf/wasm` (build output). Layout per [code organisation §1](../guidelines/code-organization.md#1-top-level-layout).
- **Generation strategy** depends on the `krilla-fit` spike ([architecture §4](architecture.md#4-spikes-to-close-open-questions)): either depend on krilla and adapt its output into `lamina-cos`, or build a krilla-like builder directly over `lamina-cos`, reusing `subsetter`, `skrifa` and HarfRust. The [spike result](../../spikes/krilla-fit/README.md) recommends the second (recorded in [ADR 0005 §6](../decisions/0005-document-model.md); accepted when the spike's PR merges).
- **Split WASM modules** (ADR 0016): the base module (COS, model, save, build) and a text module (shaping, subsetting) may be separate so pure-structure users don't pay for HarfRust. Decided by measurement in B6.
- **TypeScript is synchronous after `init()`**. `init()` takes a URL, `Response`, bytes or a precompiled `WebAssembly.Module` (required on Workers). Export conditions: `browser`, `workerd`, `node`, `deno`, `default`; checked with publint and `attw --pack`.
- **Lockstep versions** for the core crates, `@laminapdf/wasm`, `@laminapdf/sdk` and the render packages; OIDC trusted publishing (ADR 0017).

### API sketch (illustrative, not final)

TypeScript:

```ts
import { init, Builder, Document, Font } from "@laminapdf/sdk";

await init(); // in a Worker: await init({ module: wasmModule })

const inter = Font.fromBytes(await (await fetch("/fonts/Inter.ttf")).arrayBuffer());
// PDF/UA-1 needs a title (shown via DisplayDocTitle) and a language; the profile checks both.
const b = Builder.create({ profile: "pdf-ua-1", lang: "en-GB", title: "Invoice 42" });

const page = b.page({ size: "A4" });
const s = page.surface();
s.text("Invoice 42", { font: inter, size: 24, at: [56, 780], tag: "H1" });
s.textBox(longParagraph, { font: inter, size: 11, box: [56, 600, 483, 160], tag: "P" });
s.image(logoPng, { at: [440, 760], width: 100, alt: "Acme logo" });  // tagged Figure with /Alt
s.path(p => p.moveTo(56, 590).lineTo(539, 590))
  .stroke({ color: "#999", width: 0.5 })
  .artifact("Layout");                    // decorative rule: marked as an artifact, not tagged
s.text("Pay online", {
  font: inter, size: 11, at: [56, 563], tag: "P",
  // Link structure element holding the text and the annotation (OBJR); /Contents describes the target
  link: { uri: "https://example.com/pay", contents: "Pay invoice 42 online" },
});
page.finish();

b.outline([{ title: "Invoice", page: 0 }]);
const doc = b.finish();                 // a Document, not bytes
const bytes: Uint8Array = doc.save();   // full save of a new file

// later: read and edit with the same type
const again = Document.open(bytes);
again.transact(tx => tx.setInfo({ subject: "Paid" }));
const updated = again.save({ mode: "incremental" });
```

Rust:

```rust
use lamina::{Builder, BuildOptions, Document, Font, Info, OpenOptions, PageSize, Profile, SaveOptions, Tag};

let inter = Font::from_bytes(inter_bytes)?;
let mut b = Builder::new(BuildOptions {
    profile: Profile::PdfUa1,
    lang: "en-GB".into(),
    title: Some("Invoice 42".into()), // PDF/UA-1: required, shown via DisplayDocTitle
    ..Default::default()
});
let mut page = b.page(PageSize::A4);
let mut s = page.surface();
s.text("Invoice 42", &inter, 24.0, (56.0, 780.0)).tag(Tag::H1);
page.finish();
let mut doc: Document = b.finish()?;

let mut out = Vec::new();
doc.save(&mut out, SaveOptions::default())?;              // any WriteSink

let mut doc = Document::open(out, OpenOptions::default())?;  // owns the bytes (ADR 0005)
doc.transact(|tx| tx.set_info(Info { subject: Some("Paid".into()), ..Default::default() }))?;
```

### Platform matrix

| Platform | How it loads | Tested in CI by |
| --- | --- | --- |
| Browser (Chromium, Firefox, WebKit) | ESM + `.wasm` asset, bundler or CDN | Vitest browser mode + Playwright |
| Node 22/24/26 (LTS lines) | `node` condition, `.wasm` via `fs`-free `fetch(file URL)` or bytes | Vitest |
| Deno, Bun | `deno`/`default` conditions | smoke tests per release |
| Cloudflare Workers | `workerd` condition, precompiled `WebAssembly.Module` import | `@cloudflare/vitest-pool-workers` |
| Native Rust | `lamina` crate | nextest on macOS, Windows, Linux; check-only for iOS/Android |

Node support follows the LTS lines current at each release. On 2026-10-28, Node 22 and 24 are in maintenance LTS and Node 26 enters active LTS; Node 20 reached end of life on 2026-04-30 ([Node.js release schedule](https://github.com/nodejs/release#release-schedule)).

## 6. Docs, examples and publishing

- **Docs site** (recommendation): Starlight (MIT) in `apps/docs`, deployed to Workers static assets at `docs.laminapdf.com`, with TypeDoc output for `@laminapdf/sdk`; Rust API docs on docs.rs. Every public item documented (`missing_docs` denied).
- **Examples**: `examples/` in the façade crate (`cargo run --example invoice`) and a TS examples folder run as tests. Minimum set: hello world, invoice, multilingual (Latin + Arabic + Devanagari + CJK), images and vectors, links and outlines, Worker endpoint, PDF/UA report, PDF/A archive, "generate then edit".
- **Publishing (deferred, decided 2026-10-28)**: crates and npm packages stay local for now. Consumers use workspace/path dependencies, git dependencies or `pnpm pack` tarballs. When the owner decides to publish: reserve names, first manual crates.io publish then trusted-only, npm via OIDC with provenance, GitHub artifact attestations for `.wasm`, `0.x` pre-releases with a written stability policy ([architecture: Publishing (later)](architecture.md#publishing-later)).

## 7. Rounds

Architecture Round B is delivered as rounds B1–B9, each split into todos (one PR each). B9 was added after review; it is numbered last to keep existing references stable, but it runs after B6 and in parallel with B7–B8, and it must merge before web reader round C5 ([roadmap](roadmap.md#sequence-and-dependencies)). Each PR is mergeable on its own, keeps CI green and follows the [~400-line guidance](../guidelines/git-workflow.md#5-pull-requests) where code allows. ADRs land in the PR that depends on them. Prerequisite: architecture Round A (`lamina-syntax`, `lamina-cos`, workspace, CI fast lane). Each round is split into todos (one PR each) in [`docs/todos/`](../todos/README.md); the links under each round below point to them, and their **Depends on** fields fix the order.

### B1. Decisions and spike

**Todos:** [B1a](../todos/done/B1a-round-b-adrs.md) (ADRs, done), [B1b](../todos/done/B1b-krilla-fit-spike.md) (spike, done).

**Deliverable:** ADRs 0003 (tolerance and `Limits`), 0005 (document model), 0006 (op log), 0007 (save planner), 0018 (test strategy); `spikes/krilla-fit/` with a written result and a generation-strategy decision.
**Accepted when:** ADRs reviewed and marked Accepted or Proposed with named open points; spike README answers "can krilla write into an existing object graph?" with measurements.

### B2. Document model

**Todos:** [B2a](../todos/done/B2a-limits-owned-document.md) (`Limits`, warning codes, strict mode, owned `Document`; done in [#13](https://github.com/laudebugs/lamina/pull/13)), [B2b](../todos/done/B2b-standard-security-decryption.md) (decryption; done in [#22](https://github.com/laudebugs/lamina/pull/22)), [B2d](../todos/done/B2d-windowed-reading.md) (windowed reading; done in [#29](https://github.com/laudebugs/lamina/pull/29)), [B2c](../todos/done/B2c-lamina-model-corpus-fuzz.md) (`lamina-model`, IDs, corpus facts, pypdf differential, fuzz targets); B2b and B2c both follow B2a and can run in parallel.

**Deliverable:** `lamina-model` (typed `Page`, `Outline`, `Info`, `NameTree` views over lazy COS; `LaminaId`s); `Document::open(source, OpenOptions)` with `Limits`, strict mode and an optional password; standard security handler decryption in `lamina-syntax` (read only: R2–R4 with RC4 and AES-128 crypt filters, R5/R6 AES-256, `/Identity` and per-stream crypt filters, `/EncryptMetadata`; [04 §7](../research/pdf/04-interactive-security.md#7-encryption-76)); the golden and corpus harness and the first `fuzz/` targets. Writing encrypted files is [full-editor E11](full-editor.md#e11-security-and-sanitisation).
**Accepted when:** every file in `fixtures/corpus` opens or fails with a coded error; encrypted fixtures for every standard revision (R2, R3, R4 RC4, R4 AES-128, R5, R6; user and owner passwords; empty user password) open and match the unencrypted original's page count, outline and strings; a wrong password fails with a coded error; fuzz targets run 10 min in CI without a panic; page count and outline match pypdf's on the corpus.

### B3. Ops and save

**Todos:** [B3a](../todos/done/B3a-overlay-and-ops.md) (overlay, `lamina-ops`, undo; done in [#31](https://github.com/laudebugs/lamina/pull/31)), [B3b](../todos/done/B3b-lamina-save-writers.md) (`lamina-save` writers, `WriteSink`, rebase; done in [#35](https://github.com/laudebugs/lamina/pull/35)), [B3c](../todos/done/B3c-save-planner-and-roundtrip.md) (`SavePlanner`, `SaveConstraint`, round-trip; done in [#50](https://github.com/laudebugs/lamina/pull/50)).

**Deliverable:** `lamina-ops` (`Op`, `transact`, inverses, undo/redo) and `lamina-save` (`SavePlanner`, `SaveConstraint`, incremental and full writers over `WriteSink`). Proving ops: `SetInfo`, `RotatePage`.
**Accepted when:** open → op → save → reopen round-trips in lamina, pdf.js, PDFium and qpdf; signed fixtures report `AppendOnly` and refuse a full rewrite; undo restores byte-identical output.

### B4. Fonts and text

**Todos:** [B4a](../todos/done/B4a-font-loading-and-subsetting.md) (font loading, subsetting, Type 0 + ToUnicode), [B4b](../todos/done/B4b-shaping-bidi-line-breaking.md) (HarfRust, ICU4X bidi and line breaking), [B4c](../todos/done/B4c-standard-14-and-simple-fonts.md) (standard 14, encodings).

**Deliverable:** `lamina-text`: font loading (skrifa), shaping (HarfRust), bidi and line breaking (ICU4X), subsetting (`subsetter`), ToUnicode/`/W` writers, standard 14 metrics, missing-glyph diagnostics.
**Accepted when:** shaping goldens for Latin, Arabic, Devanagari, Hebrew and CJK match HarfBuzz output; extracted text from generated files (pdf.js, pypdf, pdftotext) equals the input string, including ligatures.

### B5. Builder and the `lamina` crate

**Todos:** [B5a](../todos/done/B5a-lamina-build-core.md) (builder, vectors, colour, images), [B5b](../todos/done/B5b-builder-text-and-document-features.md) (text, links, outlines, labels, metadata), [B5c](../todos/done/B5c-lamina-facade-and-examples.md) (`lamina` façade, examples, byte and render goldens), [B5d](../todos/done/B5d-text-report-generation-performance.md) (a 100-page text report within 500 ms: shape plans cached per font, a line holding a whole item reuses its shaping, fewer copies in `pdf_run`; the B6c report went from 658–958 ms to 251–422 ms in Node 24 on a heavily loaded machine, and is benchmarked nightly by `lamina`'s `report` bench).

**Deliverable:** `lamina-build` (pages, text, text boxes, images, paths, colour, links, named destinations, outlines, page labels, Info + XMP); `finish()` returns `Document`; the `lamina` façade crate with Rust examples.
**Accepted when:** every example produces byte-identical goldens; qpdf `--check` clean; golden renders match in PDFium and hayro within thresholds.

### B6. WASM and `@laminapdf/sdk`

**Todos:** [B6a](../todos/done/B6a-lamina-wasm-and-adr-0016.md) (`lamina-wasm`, split base/text modules, ADR 0016, size budgets), [B6b](../todos/done/B6b-lamina-pdf-package.md) (`@laminapdf/sdk`), [B6c](../todos/done/B6c-runtime-matrix.md) (runtime matrix).

**Deliverable:** `lamina-wasm` (wasm-bindgen + tsify), `@laminapdf/wasm`, `@laminapdf/sdk` with `init()` and export conditions; ADR 0016; size budget checks; the runtime matrix in CI.
**Accepted when:** the same example suite passes in Chromium, Firefox, WebKit, Node 22/24/26, Deno, Bun and workerd; size budgets enforced (CI fails above budget); publint and attw clean.

### B7. Docs, examples and first release

**Todos:** [B7a](../todos/done/B7a-docs-site.md) (docs site; done), [B7c](../todos/done/B7c-docs-custom-domain.md) (`docs.laminapdf.com`; done), [B7b](../todos/done/B7b-versioning-and-0-1-0-alpha.md) (Changesets, ADR 0017, local `0.1.0-alpha.0`; done); publishing later: [P1](../todos/ready/P1-reserve-names-and-trusted-publishing.md), [P2](../todos/ready/P2-first-registry-prerelease.md).

**Deliverable:** `apps/docs` site; examples; Changesets with Cargo version sync; ADR 0017 (versioning; publishing part deferred); `0.1.0-alpha.0` versioned and tagged, **not published**.
**Accepted when:** docs deploy on a preview URL per PR; a fresh project outside the repo installs the packed tarballs (`pnpm pack`) and a git/path dependency on the `lamina` crate, and runs the quick start in each runtime.

### B8. Tagged PDF, PDF/UA and PDF/A

**Todos:** [B8a](../todos/done/B8a-structure-tree.md) (structure tree), [B8b](../todos/done/B8b-conformance-profiles-and-verapdf.md) (profiles, veraPDF, `0.2.0-alpha`).

**Deliverable:** structure tree builder (`tag()` on content, artifacts, alt text, MCID and ParentTree bookkeeping), conformance profiles, XMP identification, output intents, embedded sRGB ICC; veraPDF in the nightly lane (as an external tool, never linked); release `0.2.0-alpha`.
**Accepted when:** generated examples pass veraPDF for PDF/UA-1 and PDF/A-2b/2u/3b; a missing alt text or `Lang` fails `finish()` with a coded error; the Matterhorn checks that machines can test pass.

### B9. Text extraction

**Todos:** [B9a](../todos/done/B9a-lamina-content-and-adr-0023.md) (ADR 0023, `lamina-content`), [B9b](../todos/done/B9b-text-extraction-core.md) (fonts, CMaps and Unicode per glyph in `lamina-text`), [B9d](../todos/done/B9d-text-runs-quads-and-page-text.md) (quads, runs, `ActualText`, MCIDs, the façade's `page.text()`), [B9e](../todos/done/B9e-separators-at-backward-jumps.md) (a separator where the pen jumps back on a line, found by U2b), [B9c](../todos/done/B9c-text-extraction-bindings-and-differential.md) (`page.text()` in WASM/TS, differential and quad tests; must merge before web C5), [B9f](../todos/done/B9f-live-text-differential.md) (the text differential in the nightly lane).

**Deliverable:** ADR 0023 (content-stream reading: adds `lamina-content` to the crate list); `crates/lamina-content` as a **read-only** content-stream interpreter (tokenizer, graphics and text state, `q`/`Q`, `cm`, text operators, Form XObject recursion with depth limits, marked content); text extraction in `lamina-text` on top of it: font dictionaries read (simple and Type 0, `/Widths` and `/W`, embedded font metrics via skrifa), encodings and ToUnicode CMap parsing, glyph quads, `ActualText` and MCIDs; exposed in `lamina`, `lamina-wasm` and `@laminapdf/sdk` (`page.text()`). [Full-editor](full-editor.md#6-design-summary) E2 adds a content append API to this crate and E7 extends it for editing (ADR 0022).
**Accepted when:** extracted text for fixtures in five scripts (Latin, Arabic, Devanagari, Hebrew, CJK) matches expected text and pdf.js/pdftotext output on the corpus; glyph quads lie within tolerance of PDFium's character boxes; encrypted fixtures extract the same text as their originals; extraction fuzz target runs 10 min without a panic.

### After B9

Editing operations (merge, split, annotations, forms, signing…) are added to the library in the same PR that adds them to the core, round by round in [full-editor.md](full-editor.md). The [capability matrix](full-editor.md#3-tracking-the-capability-matrix) records which ones the library exposes.

## 8. Risks

| Risk | Mitigation |
| --- | --- |
| krilla cannot target our object graph, forcing a parallel builder | Confirmed by the [B1 spike](../../spikes/krilla-fit/README.md): own builder over `lamina-cos`, reusing `pdf-writer`/`subsetter`/`skrifa`/HarfRust and porting krilla's approaches with attribution |
| WASM size grows past budget (shaping, ICU4X data) | Split text module; ICU4X data slicing; budget gate in CI |
| Font licensing (fsType, bundled fonts) | Respect fsType; bundle no fonts in the core package; OFL fonts only in examples |
| PDF/UA and PDF/A rules not testable automatically (Matterhorn has human checks) | Profile errors cover machine-checkable rules; docs state the rest |
| API churn before 1.0 annoys early users | `0.x` with a published stability policy; cargo-semver-checks from 0.2 |
| Workers memory (128 MB) with large CJK fonts | Subset early, stream output through `WriteSink`; document limits |

## 9. Open questions

1. ~~Generation strategy: depend on krilla or build over `lamina-cos`?~~ **Answered by the [`krilla-fit` spike](../../spikes/krilla-fit/README.md)** (recorded in [ADR 0005 §6](../decisions/0005-document-model.md); accepted when the spike's PR merges): krilla cannot write into an existing object graph (it only returns finished bytes). Recommendation: build `lamina-build` over `lamina-cos` and reuse `pdf-writer` (`Content`), `subsetter`, `skrifa` and HarfRust; use krilla as a reference and a test oracle, not as a core dependency.
2. Do we ship an optional `@laminapdf/fonts` package (Noto subsets) for zero-config text, or always require font bytes?
3. Do we offer a JSX/declarative layer on top of the builder, and when?
4. Is a layout engine (flowing text, tables, pagination) a library goal, and is Typst embedding the route?
5. Which binding is next after TS: Python (largest PDF audience) or UniFFI (mobile)?
6. Public stability of the low-level COS API: exposed in 0.x or kept internal?

## 10. To-dos

Each round's todos (one per PR, with dependencies) are linked from [§7](#7-rounds) and on the [todo board](../todos/README.md); this checklist is the summary.

### B1. Decisions and spike

- [x] Write ADR 0003: tolerance policy, warning codes, `Limits` defaults with numbers
- [x] Write ADR 0005: lazy COS overlay, typed views, `LaminaId` rules
- [x] Write ADR 0006: `Op` enum shape, transactions, undo scope, barrier ops
- [x] Write ADR 0007: `SavePlanner`, `SaveConstraint`, save ordering
- [x] Write ADR 0018: goldens, corpus, differential and fuzz tiers
- [x] Run the `krilla-fit` spike: generate a 3-page document into a `lamina-cos` graph and measure size and time ([result](../../spikes/krilla-fit/README.md))
- [x] Record the generation-strategy decision in the spike README and link it from ADR 0005

### B2. Document model

- [x] Create `crates/lamina-model` with `Page`, `Outline`, `Info`, `NameTree` views (B2c; also `NumberTree`, `PageLabels`)
- [x] Implement `Document::open(source, OpenOptions)` over an owned `ByteSource` ([ADR 0005](../decisions/0005-document-model.md)), with `OpenOptions` holding `Limits`, strict mode and an optional password ([ADR 0003](../decisions/0003-reader-tolerance-and-limits.md)) (B2a; the password is used for decryption since B2b; windowed reading of non-slice sources since B2d)
- [x] Stable warning codes and strict mode; `fixtures/codes.json` checked against the Rust enums ([ADR 0003](../decisions/0003-reader-tolerance-and-limits.md) §2, §4) (B2a)
- [x] Move the robustness tests' 30 s wall-clock budget to a nextest timeout ([ADR 0018](../decisions/0018-test-strategy.md)) (B2a)
- [x] Windowed reading for sources without `as_slice` (64 KiB windows, chunked repair scan, `DataPending` with only the needed windows; [ADR 0005](../decisions/0005-document-model.md) §2) ([B2d](../todos/done/B2d-windowed-reading.md))
- [x] Standard security handler decryption in `lamina-syntax`: R2–R4 (RC4, AES-128), R5/R6 (AES-256), crypt filters, `/EncryptMetadata` (B2b, [#22](https://github.com/laudebugs/lamina/pull/22))
- [x] Encrypted fixtures for every revision (user, owner and empty passwords); wrong-password error code (B2b, [#22](https://github.com/laudebugs/lamina/pull/22))
- [x] Assign `LaminaId`s to pages and outline items; test stability across reopen (B2c; also across an incremental save, and path IDs for direct pages)
- [x] Add `fixtures/corpus/expected.json` facts (page count, outline titles) and a corpus test (B2c; written by `tools/corpus-facts`, exceptions in `exceptions.toml`)
- [x] Add `fuzz/` targets for open and page-tree walk; run 10 min in CI (B2c: `lex`, `parse_object`, `flate_decode`, `open`, `page_tree`; scheduled in `nightly.yml` since [X3a](../todos/done/X3a-nightly-lane.md))
- [x] Differential test: page count and outline vs pypdf (B2c; corpus plus fixed-seed mutants, precomputed by `cargo xtask differential --update`)

### B3. Ops and save

- [x] Create `crates/lamina-ops`: `Op`, `transact`, inverse ops, undo/redo stack (B3a; inverses are computed at undo time from an undo record derived from the COS journal, plus savepoints, coalescing and the `OpLog` envelope)
- [x] Create `crates/lamina-save`: `SavePlanner`, incremental and full writers, `WriteSink` (B3b: the crate, both writers over the overlay, `WriteSink`, `/ID` and rebase; B3c: `SavePlanner`, `Document::save`)
- [x] Implement `SaveConstraint` detection (signatures → `AppendOnly`) (B3c: signature fields, `/Perms` DocMDP and UR3, transforms, FieldMDP locks, DocTimeStamp, a repaired file's objects; rewrite reasons; violations; the ADR 0007 §3 mode table and the explicit override)
- [x] Implement proving ops `SetInfo` and `RotatePage` with undo tests (B3a: ops, undo and property tests; B3c: byte-identical output after undo)
- [x] Round-trip test: reopen saved output in lamina, pdf.js, PDFium and qpdf `--check` (B3c: `cargo xtask roundtrip`, full lane)

### B4. Fonts and text

- [x] Create `crates/lamina-text`: `Font::from_bytes`, TTC index, variable-font instancing ([B4a](../todos/done/B4a-font-loading-and-subsetting.md))
- [x] Integrate HarfRust shaping behind a `Shaper` trait; add per-run OpenType features ([B4b](../todos/done/B4b-shaping-bidi-line-breaking.md))
- [x] Add ICU4X bidi and line breaking; greedy breaking inside a box ([B4b](../todos/done/B4b-shaping-bidi-line-breaking.md))
- [x] Integrate `subsetter`; deterministic subset tags; `/W` and chunked ToUnicode ([B4a](../todos/done/B4a-font-loading-and-subsetting.md))
- [x] Bundle standard 14 AFM metrics ([B4c](../todos/done/B4c-standard-14-and-simple-fonts.md)); reject them under PDF/A and PDF/UA profiles ([B8b](../todos/done/B8b-conformance-profiles-and-verapdf.md))
- [x] Emit missing-glyph diagnostics with codes ([B4a](../todos/done/B4a-font-loading-and-subsetting.md): `missing-glyph`)
- [x] Shaping goldens for five scripts ([B4b](../todos/done/B4b-shaping-bidi-line-breaking.md))
- [x] Extraction tests with pdf.js, pypdf, pdftotext that return the input in every script ([B4b](../todos/done/B4b-shaping-bidi-line-breaking.md): pdf.js and pypdf facts, exact for Latin and CJK; [B4d](../todos/done/B4d-tounicode-per-text-cids.md): one CID per glyph and text and pdftotext facts; Arabic, Devanagari (pypdf) and the mixed line now exact, the remaining lines listed as extractor bugs with upstream links)

### B5. Builder and the `lamina` crate

- [x] Create `crates/lamina-build`: `Builder`, `page()`, `surface()`, `finish() -> Document` ([B5a](../todos/done/B5a-lamina-build-core.md))
- [x] Surface ops: text, text box ([B5b](../todos/done/B5b-builder-text-and-document-features.md)); image (JPEG, PNG + SMask, raw RGBA), path, clip, opacity, colour spaces done in [B5a](../todos/done/B5a-lamina-build-core.md)
- [x] Document ops: links, named destinations, outlines, page labels, Info + XMP, viewer preferences ([B5b](../todos/done/B5b-builder-text-and-document-features.md))
- [x] Create the `lamina` façade crate; `lib.rs` docs open with the invoice example ([B5c](../todos/done/B5c-lamina-facade-and-examples.md))
- [x] Add Rust examples (hello, invoice, multilingual, vectors, links-and-outlines, generate-then-edit) ([B5c](../todos/done/B5c-lamina-facade-and-examples.md))
- [x] Byte-identical goldens and render goldens (PDFium and hayro) for every example ([B5c](../todos/done/B5c-lamina-facade-and-examples.md): 72 dpi goldens per engine, hayro vs PDFium at 150 dpi)
- [x] A 100-page text report within 500 ms after `init()`, benchmarked nightly ([B5d](../todos/done/B5d-text-report-generation-performance.md): `crates/lamina/benches/report.rs` in `cargo xtask bench`, `packages/sdk/bench/reports.mjs` in Node)

### B6. WASM and `@laminapdf/sdk`

- [x] Create `crates/lamina-wasm` (wasm-bindgen + tsify) and the `cargo xtask wasm` build with `wasm-opt` ([B6a](../todos/done/B6a-lamina-wasm-and-adr-0016.md); `-O1`, which gzips smaller than `-Oz`, ADR 0016 §5)
- [x] Create `packages/sdk` with `init()`, `Builder`, `Document`, `Font`, `LaminaError` codes matching Rust ([B6b](../todos/done/B6b-lamina-pdf-package.md))
- [x] Add export conditions and validate with publint and `attw --pack` ([B6b](../todos/done/B6b-lamina-pdf-package.md))
- [x] Write ADR 0016 with measured sizes; decide split WASM modules ([B6a](../todos/done/B6a-lamina-wasm-and-adr-0016.md), [ADR 0016](../decisions/0016-package-design.md): base + text)
- [x] Add size-budget checks to CI (WASM gz, JS min+gz) ([B6a](../todos/done/B6a-lamina-wasm-and-adr-0016.md): `cargo xtask size`, the fast lane's `wasm` job; the TS API's share with B6b)
- [x] Run the TS example suite in Chromium, Firefox, WebKit, Node 22/24/26, Deno, Bun, workerd ([B6c](../todos/done/B6c-runtime-matrix.md): `cargo xtask pdf-runtimes`; every runtime in the fast lane since 2026-11-01, all nightly)

### B7. Docs, examples and first release

- [x] Create `apps/docs` (Starlight) with quick start per runtime, guides and TypeDoc API ([B7a](../todos/done/B7a-docs-site.md))
- [x] Deploy docs to Workers static assets with per-PR preview URLs ([B7a](../todos/done/B7a-docs-site.md): `.github/workflows/docs.yml`; it deploys once the owner adds the Cloudflare secrets, `apps/docs/README.md`)
- [x] Serve the docs at `docs.laminapdf.com` and fill in the domain everywhere ([B7c](../todos/done/B7c-docs-custom-domain.md))
- [x] Add Changesets and the Cargo version-sync script; write ADR 0017 ([B7b](../todos/done/B7b-versioning-and-0-1-0-alpha.md): `cargo xtask version sync [--check]`, [ADR 0017](../decisions/0017-versioning-and-release.md) versioning part)
- [x] Tag `0.1.0-alpha.0` locally (not published); verify install from `pnpm pack` tarballs and a git/path crate dependency in a clean project in each runtime ([B7b](../todos/done/B7b-versioning-and-0-1-0-alpha.md): versioned at `0.1.0-alpha.0` and `cargo xtask verify-install`, run on macOS; the `v0.1.0-alpha.0` tag and its release-lane run are the owner's post-merge step)
- [x] Write the stability policy and changelog (published with the first registry release) ([B7b](../todos/done/B7b-versioning-and-0-1-0-alpha.md): drafts [`docs/stability-policy.md`](../stability-policy.md) and [`CHANGELOG.md`](../../CHANGELOG.md), generated by Changesets; published in [P2](../todos/ready/P2-first-registry-prerelease.md))
- [ ] Later, when the owner decides: publish to npm and crates.io ([architecture: Publishing (later)](architecture.md#publishing-later))

### B8. Tagged PDF, PDF/UA and PDF/A

- [x] Add structure-tree building: `tag()`, artifacts, alt text, MCIDs, ParentTree, RoleMap ([B8a](../todos/done/B8a-structure-tree.md))
- [x] Add profiles `PdfUa1`, `PdfA2b`, `PdfA2u`, `PdfA3b` with build-time errors ([B8b](../todos/done/B8b-conformance-profiles-and-verapdf.md); associated files for PDF/A-3b)
- [x] Write XMP identification schemas and output intents with an embedded sRGB profile ([B8b](../todos/done/B8b-conformance-profiles-and-verapdf.md): the 480-byte CC0 sRGB v4 profile)
- [x] Add veraPDF (nightly) over all generated examples; fail on any violation ([B8b](../todos/done/B8b-conformance-profiles-and-verapdf.md): the pinned Java CLI, not Docker; `cargo xtask verapdf`)
- [x] Add PDF/UA and PDF/A examples and docs pages ([B8b](../todos/done/B8b-conformance-profiles-and-verapdf.md))
- [ ] Tag `0.2.0-alpha` (local, unpublished): the version is `0.2.0-alpha.0` since [B8b](../todos/done/B8b-conformance-profiles-and-verapdf.md); the owner tags `v0.2.0-alpha.0` after it merges

### B9. Text extraction

- [x] Write ADR 0023 (content-stream reading; `lamina-content` crate; limits on operators and XObject depth) ([B9a](../todos/done/B9a-lamina-content-and-adr-0023.md))
- [x] Create `crates/lamina-content`: tokenizer and read-only interpreter with graphics and text state (B9a; Form XObjects, marked content, the `content` fuzz target, operator counts checked against pdf.js)
- [x] Font dictionary reading: simple and Type 0 fonts, `/Widths`, `/W`, embedded metrics ([B9b](../todos/done/B9b-text-extraction-core.md))
- [x] Encodings, glyph-name fallbacks and ToUnicode CMap parsing ([B9b](../todos/done/B9b-text-extraction-core.md))
- [x] Glyph runs with Unicode, quads, direction, `ActualText`, MCIDs; `page.text()` in Rust ([B9d](../todos/done/B9d-text-runs-quads-and-page-text.md))
- [x] Expose `page.text()` in WASM and TS ([B9c](../todos/done/B9c-text-extraction-bindings-and-differential.md): a third module, `extract`)
- [x] Extraction tests in five scripts vs pdf.js and pdftotext; quad tests vs PDFium; fuzz target ([B9c](../todos/done/B9c-text-extraction-bindings-and-differential.md); the nightly run on the external corpus is [B9f](../todos/done/B9f-live-text-differential.md))
