# E10g — Signature validation: integrity, chain, revocation, signer inspection

| Field | Value |
| --- | --- |
| ID | `E10g` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E10d](./E10d-pades-signing-core.md) |
| Blocks | [E10h](./E10h-revision-diff-and-attacks.md), [E11e](./E11e-public-key-security.md) |
| ADRs | Implements ADR 0020; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10g-signature-validation` |
| PR title | `feat(sign): signature validation with structured results` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Validate each signature per the 04 §8.8 checklist: structural rules (ByteRange shape, hex-only Contents, exact gap, c+d at a revision end), digest and CMS signature, `signing-certificate-v2`, chain building to trust anchors at signing time, timestamps, revocation (OCSP/CRL from CMS/DSS or fetched at the edge where reachable); a structured result (intact/valid/trusted/coverage/modification level) and signer/certificate inspection.

## Context

- [04 §8.8 Attacks on signatures](../../research/pdf/04-interactive-security.md#88-attacks-on-signatures-ruhr-universität-bochum-pdf-insecurity-research)
- ADR 0020

## Scope

**In scope**

- Validator in `lamina-sign`
- Revocation fetcher interface (edge)
- Certificate viewer data API

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.04` verify signatures; `F.07` inspect signer identity and certificates
- **Crates and packages:** `lamina-sign`
- **`SaveConstraint`:** read-only
- **Web vs desktop:** desktop uses OS trust store; web uses the trust list decided in E10b
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `F.04` → `read` (core, pdf); `F.07` → `read` (core, pdf) (web and desktop columns flip in [E10j](E10j-signature-ui.md), with the signature panel)

## Tasks

- [ ] Structural checks
- [ ] CMS verify
- [ ] Chain and time
- [ ] Revocation
- [ ] Result model

## Acceptance criteria

- [ ] Validation verdicts match EU DSS on a fixture set of valid/invalid/expired/revoked signatures
- [ ] Any structural anomaly is a hard failure (USF tests)

## Post-merge checks

None.

## Tests

- Fixture set with expected verdicts

## Docs to update

- API docs: validation
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
