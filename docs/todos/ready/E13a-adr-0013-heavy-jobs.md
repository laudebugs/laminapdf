# E13a — ADR 0013: heavy jobs on Workflows + Containers; OCR and conversion engines and licences

| Field | Value |
| --- | --- |
| ID | `E13a` |
| Status | `ready` |
| Plan | [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [E12a](./E12a-redaction-feature-plan.md) |
| Blocks | [E13b](./E13b-ocr-and-conversion-feature-plan.md) |
| Dependency notes | [E12a](E12a-redaction-feature-plan.md), `C2` ([web-app §C2](../../plans/web-app.md#c2-render-backend-and-adrs): `lamina-render`, ADR 0011) |
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
- Engine matrix: Tesseract (Apache-2.0), PaddleOCR, ocrmypdf (MPL), LibreOffice (MPL), others; WASM OCR size
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

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
