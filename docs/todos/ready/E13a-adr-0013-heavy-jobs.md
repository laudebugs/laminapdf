# E13a — ADR 0013: heavy jobs on Workflows + Containers; OCR and conversion engines and licences

| Field | Value |
| --- | --- |
| ID | `E13a` |
| Status | `ready` |
| Plan | [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [C2b](../done/C2b-lamina-render-crate.md), [E12a](./E12a-redaction-feature-plan.md) |
| Blocks | [E13b](./E13b-ocr-and-conversion-feature-plan.md) |
| Dependency notes | [E12a](E12a-redaction-feature-plan.md), `C2` ([web-app §C2](../../plans/web-app.md#c2-render-backend-and-adrs): `laminapdf-render`, ADR 0011) |
| ADRs | Writes ADR 0013; amends/uses ADR 0011 |
| Branch | `docs/e13a-adr-0013-heavy-jobs` |
| PR title | `docs(adr): 0013 heavy jobs, OCR and conversion engines` |
| Size | M (200–600) |
| PR | — |

## Goal

Decide how lamina runs heavy, opt-in server jobs (OCR, Office ↔ PDF, PDF → Word/Excel, HTML → PDF, batch) on Cloudflare: Workflows orchestrating Containers, Queues, R2 for blobs with lifecycle expiry, Browser Run for HTML; the job API and privacy rules; and which engines (client WASM OCR, desktop local OCR, server OCR, LibreOffice for Office, conversion engines) pass the licence rules (no AGPL in shipped artefacts; copyleft only behind a process boundary).

## Context

- [cloudflare notes §2](../../research/architecture/notes/cloudflare.md#2-storage-and-compute-products-r2-d1-kv-durable-objects-queues-workflows-containers-browser-run-workers-ai) and [§3](../../research/architecture/notes/cloudflare.md#3-limits-for-server-side-pdf-work-in-a-worker-vs-containers): products and limits
- [report: Cloudflare section](../../research/architecture/app-and-package-architecture.md#cloudflare-hosts-everything-for-about-5-a-month-with-heavy-jobs-in-containers)
- [full-editor: optional server features](../../plans/full-editor.md#optional-cloudflare-server-features)
- [full-editor §9](../../plans/full-editor.md#9-open-questions) Q2, Q3
- [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md)

## Scope

**In scope**

- ADR 0013 file: topology diagram, job lifecycle, retention, limits (Worker 128 MB vs Containers), cost model
- Engine matrix: Tesseract (Apache-2.0, with Leptonica BSD-2), PaddleOCR (Apache-2.0), LibreOffice (MPL-2.0), others; WASM OCR size. ocrmypdf (MPL-2.0) is listed as excluded: it requires Ghostscript (AGPL-3.0) at run time, and container images we run as a service count as shipped artefacts under ADR 0001/0019
- Licence rule for container images (the ADR records it): no AGPL component anywhere (in our binaries or in any container image, since images we run as a service count as shipped artefacts under ADR 0001/0019); GPL/LGPL code only as separate executables or system libraries of the base image that our code reaches across a process boundary (invoked as a program, never linked into a lamina binary), recorded in the SBOM; our own binaries link no GPL library. "No GPL in the image" is not achievable, because Linux base images ship GPL tools (bash, coreutils, glibc is LGPL), so the rule is about AGPL and about linking
- OCR pipeline decision (recommended, no AGPL): Tesseract only, no ocrmypdf and no Ghostscript. lamina renders each page to an image (`laminapdf-render`), Tesseract produces hOCR, and lamina writes the invisible text layer into the original PDF (`AddOcrLayer`, E13c), so the PDF is never re-encoded by a third-party tool. Using ocrmypdf instead needs an explicit owner decision recorded in this ADR (an exception to ADR 0019 with the AGPL obligations for a network service spelled out); it is not the default
- Whether jobs require an account (links to H2) — decision or open question

**Out of scope** (and which todo covers it)

- Implementation (E13c onwards, E14e, E16g)

**Editor facts**

- **Crates and packages:** (decision) `apps/jobs` Worker + container images
- **`SaveConstraint`:** no new save behaviour beyond the round's: OCR layer and converted outputs follow normal rules (incremental / new document)
- **Web vs desktop:** identical on web and desktop
- **Server features:** the whole topology
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Engine survey with licences and sizes
- [ ] Write the ADR
- [ ] Owner sign-off
- [ ] Index updates

## Acceptance criteria

- [ ] ADR accepted with an engine per job type and retention numbers
- [ ] No engine in the ADR, or in any image it describes, is AGPL (Ghostscript, MuPDF, ocrmypdf's Ghostscript dependency included); any GPL engine is used only as a separate executable behind a process boundary, never linked into a lamina binary; the OCR pipeline is the Tesseract-only one above unless the owner signs off a recorded exception

## Post-merge checks

None.

## Tests

None (decision).

## Docs to update

- `docs/decisions/README.md`
- architecture §3
- `CLAUDE.md`
- full-editor §9 Q2/Q3
- [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open product question: do server jobs need an account (abuse, rate limits, cost)? If yes, E13e must also depend on H2

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
