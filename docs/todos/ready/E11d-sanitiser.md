# E11d — Sanitiser ops with a report

| Field | Value |
| --- | --- |
| ID | `E11d` |
| Status | `ready` |
| Plan | [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E3f](./E3f-attachments-and-associated-files.md), [E5b](./E5b-comment-threads-and-review.md), [E7c](./E7c-content-ir.md), [E11a](./E11a-security-and-sanitisation-feature-plan.md), [T2c](../done/T2c-reveal-command.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e11d-sanitiser` |
| PR title | `feat(ops): document sanitisation with a removal report` |
| Size | L (600–1,500) |
| PR | — |

## Goal

A sanitiser as a set of ops, each removing one category and reporting what it removed: hidden metadata (Info, XMP, `/PieceInfo`, thumbnails), comments, attachments (EmbeddedFiles, `/AF`, FileAttachment annots), scripts and active content (`/Names /JavaScript`, `/OpenAction` JS, `/AA`, Launch, SubmitForm, external refs, XFA), hidden layers and off-page content, and incremental history (via rewrite).

## Context

- **Design (Paper):** [G07 Sanitise](../../design/paper/sign-protect-redact.md#g07-sanitise). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §11 PDF as an attack surface](../../research/pdf/04-interactive-security.md#11-pdf-as-an-attack-surface)
- [04 §13](../../research/pdf/04-interactive-security.md#13-implications-for-laminas-design) item 11 (`sanitize()` CDR)
- [06 §3 group G](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms)

## Scope

**In scope**

- Ops: `StripMetadata`, `StripAnnotations`, `StripAttachments`, `StripActiveContent`, `StripHiddenContent` (OCG-off content needs the E7c IR; off-page via CropBox), `Sanitise { categories }`; `categories` are `laminapdf.reveal/1`'s names ([T2c](../done/T2c-reveal-command.md)): `metadata`, `annotations`, `attachments`, `active-content`, `hidden-content`, `history` (the full rewrite)
- XFA in `StripActiveContent`: XFA forms carry their own scripts (`<script>` in the template packet, FormCalc and JavaScript) that ignore the AcroForm action dictionaries, so the op removes `/AcroForm /XFA` entirely (the stream or the packet array and every packet stream it references) and clears `NeedsRendering` in the catalog; the AcroForm fields, if any, remain as the static fallback. A dynamic XFA document (no usable AcroForm) is reported as `XfaOnlyContentRemoved` so the user knows the form will not render as before
- `SanitiseReport` (JSON + UI), each removed item named by the object number `laminapdf reveal` lists
- Sanitise dialog with presets

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.08` remove hidden metadata and comments; `G.09` sanitise embedded content; `G.10` remove attachments and scripts; `G.13` remove hidden layers/information
- **Crates and packages:** `laminapdf-ops`, `laminapdf-content` (hidden content), `@laminapdf/ui`
- **`Op` variants:** the ops above (class `Document`, `requires_rewrite = true`)
- **`SaveConstraint`:** `RequiresRewrite`: a full rewrite, every earlier revision dropped. On signed files the constraint is `Conflict`: the UI names the signatures that will be invalidated and removed (ADR 0007 §3 override); order redact → rewrite → sign
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `G.08` → `round-trip` (core, pdf, web, desktop); `G.09` → `round-trip` (core, pdf, web, desktop); `G.10` → `round-trip` (core, pdf, web, desktop); `G.13` → `write` (core, pdf, web, desktop)

## Tasks

- [ ] Ops
- [ ] Report
- [ ] Dialog
- [ ] Fixtures with every active-content location, including XFA forms with `<script>` in the template packet

## Acceptance criteria

- [ ] After `Sanitise(all)`, `laminapdf reveal --fail-on metadata,annotations,attachments,active-content,hidden-content,history` exits 0 on every sanitiser fixture
- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug. Expected for the sanitiser ops (`requires_rewrite`): `Conflict` on every signed fixture
- [ ] After `Sanitise(all)` a scan finds no JS, Launch, EmbeddedFiles, `/AA` or `/OpenAction` actions anywhere, and no `/XFA` entry in `/AcroForm` and no orphaned XFA packet stream (template, datasets, config) left in the file (test over fixtures, including a static and a dynamic XFA form with scripts)
- [ ] Report lists every removed item

## Post-merge checks

None.

## Tests

- Active-content fixtures
- Scan tests

## Docs to update

- User help: sanitising
- [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
