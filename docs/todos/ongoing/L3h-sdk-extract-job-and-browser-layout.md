# L3h — Extraction jobs in `@laminapdf/sdk`, and a layout model in the browser

| Field | Value |
| --- | --- |
| ID | `L3h` |
| Status | `ongoing` |
| Plan | [shareable libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction) |
| Round / milestone | Round L3 (shareable libraries) — AI-ready extraction |
| Depends on | [L3c](../done/L3c-model-plugins.md) |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md) §6 (plug-ins, request/provide, the model allow-list), [0016](../../decisions/0016-package-design.md) (budgets), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (licences) |
| Branch | `feat/l3h-sdk-extract-job` |
| PR title | `feat(sdk): doc.extractJob() for host models, and Heron in the browser with onnxruntime-web` |
| Size | M (≈ 400–700 lines: the SDK job, a browser model package or example, tests) |
| PR | — |

## Goal

Let a web app use a layout model with Lamina PDF's extraction. L3c gave the WASM a stepped job (`WasmDocument.extractJob`) that asks the host for each page's regions; `@laminapdf/sdk` doesn't wrap it yet, and nothing runs a model in the browser. This todo adds `doc.extractJob(options)` to the SDK, with an async helper that renders each page, runs the host's model and hands back the result, and runs Docling's Heron there through onnxruntime-web, so a page the rules misread reads by its regions in the browser too.

## Context

- [L3c](../done/L3c-model-plugins.md): `WasmDocument.extractJob`, `step` with `request`, `provide({ kind, regions | words })`; the Rust `laminapdf-layout-onnx` crate's pre- and post-processing (640 × 640 bilinear resize, raw `uint8` planar RGB, scores ≥ 0.3, near-duplicate boxes dropped), which the browser version should match.
- [`tools/models/models.toml`](../../../tools/models/models.toml): Heron's pinned ONNX export (Apache-2.0 weights).
- [`@laminapdf/render-pdfium`](../../../packages/render-pdfium/README.md) and [`@laminapdf/render-hayro`](../../../packages/render-hayro/README.md): the page images.
- [AI extraction note §B](../../research/libraries/notes/llm-api-and-ai-extraction.md#b-ai-consumption-primitives-the-2026-landscape-algorithms-bboxprovenance-schema-tagged-pdf-and-rule-based-vs-model-based): onnxruntime-web.

## Scope

**In scope**

- `doc.extractJob(options)` in `@laminapdf/sdk` (`src/extract.ts`): a handle (freed with its document) with `step(budget)`, `request`, `provide(result)` and `cancel()`, typed from the binding's tsify types; and `await extractWith(doc, options, { layout, render })`, which loops the job, awaiting the host's model for each request.
- A browser layout model: onnxruntime-web (licence MIT, to verify for the version pinned) running Heron, with weights the app fetches and checks by SHA-256, never bundled; as an example or a small private package, decided in the PR.
- Budgets: the SDK's JS budget, and onnxruntime-web kept out of `@laminapdf/sdk` itself.

**Out of scope**

- OCR engines in the browser: [E13d](../ready/E13d-local-ocr.md) (implements `OcrEngine`, and in the browser answers OCR requests the same way).

## Acceptance criteria

- [ ] `doc.extractJob` with a fake model gives the same JSON as the Rust test `request_and_provide_should_give_what_the_plugin_path_gives` for the same regions.
- [ ] Heron in the browser finds the regions of `crates/laminapdf-layout-onnx/tests/reference/heron.json` within the same thresholds as the Rust test (Chromium; the weights fetched in CI only where the runner caches them, else the test is skipped with a notice).
- [ ] The SDK's JS stays within its budget.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
