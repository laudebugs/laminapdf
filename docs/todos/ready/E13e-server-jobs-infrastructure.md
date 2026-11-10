# E13e — Opt-in server jobs: Workflows + Containers with R2 lifecycle (OCR and Office conversion)

| Field | Value |
| --- | --- |
| ID | `E13e` |
| Status | `ready` |
| Plan | [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [C7c](../ongoing/C7c-performance-gates-and-launch.md), [E13c](./E13c-ocr-text-layer.md) |
| Dependency notes | [E13c](E13c-ocr-text-layer.md), `C7` ([web-app §C7](../../plans/web-app.md#c7-accessibility-offline-and-launch): web reader v0.1) |
| ADRs | Implements ADR 0013 and ADR 0011; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e13e-server-jobs-infrastructure` |
| PR title | `feat(jobs): opt-in server OCR and conversion jobs on Cloudflare` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Build the hosted job pipeline: an `apps/jobs` Worker accepting opt-in jobs (upload via presigned PUT to R2 with lifecycle expiry), a Workflow per job driving Containers (OCR engine image, LibreOffice image), status polling or events, result download, and deletion; lamina post-processes outputs in a Worker or container.

## Context

- **Design (Paper):** [X02 Convert to and from PDF](../../design/paper/convert-compare-produce.md#x02-convert-to-and-from-pdf). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- ADR 0013
- [cloudflare notes §3](../../research/architecture/notes/cloudflare.md#3-limits-for-server-side-pdf-work-in-a-worker-vs-containers) limits
- [full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 6

## Scope

**In scope**

- Job API (create, status, result, delete) with signed short-lived tokens
- Container images built and pinned in CI (licences recorded, SBOM). `containers/ocr` holds Tesseract + Leptonica + language data and a lamina binary that renders pages (`laminapdf-render`) and writes the text layer from Tesseract's hOCR (E13c `AddOcrLayer`); no ocrmypdf and no Ghostscript (AGPL), per ADR 0013's Tesseract-only pipeline
- Workflow with retries and step results in R2
- Retention: inputs and outputs expire (e.g. 24 h) and are deleted on completion download if the user chooses
- Client opt-in dialog per document

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.11` convert outlined or scanned text via OCR; `H.13` scanned → searchable PDF
- **Crates and packages:** `apps/jobs` (Worker + Workflow), `containers/ocr`, `containers/office`
- **`SaveConstraint`:** results open as new documents or apply `AddOcrLayer`
- **Web vs desktop:** web and desktop both offer opt-in server OCR; desktop prefers local
- **Server features:** Workflows, Containers, R2 (lifecycle), Queues
- **Capability-matrix rows to flip:** server path for A.11/H.13 (no new column; documented in the matrix notes)

## Tasks

- [ ] Worker API
- [ ] Workflow
- [ ] Images, with an SBOM per image and the licence check above
- [ ] Retention tests
- [ ] Opt-in UI

## Acceptance criteria

- [ ] e2e against a preview deployment: OCR a scan via the server, result downloaded, input deleted from R2 within the retention window
- [ ] No job runs without explicit opt-in (test)
- [ ] Container licence policy per [E13a](E13a-adr-0013-heavy-jobs.md) / ADR 0013 holds, checked by an SBOM (syft or equivalent, generated in CI for every image) that records every package's licence and fails the build when: any package is AGPL (by licence or by name: Ghostscript, MuPDF, ocrmypdf); or a lamina-built binary in the image links a GPL library (checked against the binary's dynamic dependencies and our Cargo/npm licence reports, which already deny GPL per ADR 0019). GPL packages from the Linux base image (shell, coreutils, …) and GPL engines invoked only as separate executables are allowed and listed in the SBOM
- [ ] A test image that adds an AGPL package, and one whose lamina binary links a GPL library, both fail the check

## Post-merge checks

- [ ] `apps/jobs` and the container images deployed to production by the deploy workflow on `main`; the R2 lifecycle rule that expires job inputs/outputs is active on the production bucket and an expired object is gone after the window (link the run and the bucket config)

## Tests

- Worker tests (Miniflare)
- Preview e2e
- SBOM licence scan

## Docs to update

- Privacy page update (ADR 0011)
- Ops runbook for the job service
- [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open product question: account required for server jobs? (abuse and cost) — if yes, add H2 as a dependency before starting
- Container cost and cold starts; set quotas

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
