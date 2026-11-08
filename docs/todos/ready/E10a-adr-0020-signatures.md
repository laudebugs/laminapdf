# E10a — ADR 0020: signatures, crypto backends and key storage per platform

| Field | Value |
| --- | --- |
| ID | `E10a` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E9a](./E9a-form-authoring-feature-plan.md) |
| ADRs | Writes ADR 0020; builds on ADR 0007 §4 |
| Branch | `docs/e10a-adr-0020-signatures` |
| PR title | `docs(adr): 0020 signatures and crypto backends` |
| Size | M (200–600) |
| PR | — |

## Goal

Decide how lamina signs and validates: PAdES levels (B-B, B-T now; B-LT/B-LTA later), CMS construction (RustCrypto + `cms`/`der`/`x509-cert`), `Signer` trait (local key, WebCrypto, PKCS#12, OS keychains, PKCS#11, remote/external two-phase), placeholder sizing, RFC 3161 client split (core builds/parses, edge transports), revocation fetching at the edge, validation result model, trust stores per platform, and the TSA proxy Worker's contract (hash only).

## Context

- [04 §8 Digital signatures](../../research/pdf/04-interactive-security.md#8-digital-signatures-128)
- [04 §9 Signing via incremental update](../../research/pdf/04-interactive-security.md#9-signing-via-incremental-update-practical-constraints)
- [04 §13 Implications](../../research/pdf/04-interactive-security.md#13-implications-for-laminas-design) items 8–9
- [ADR 0007](../../decisions/0007-save-planner.md) §4 two-phase signing
- [editing notes §6](../../research/architecture/notes/editing-architecture.md#6-signatures-and-redaction-implications-for-the-architecture)
- [full-editor §9](../../plans/full-editor.md#9-open-questions) Q5 (eIDAS QES)

## Scope

**In scope**

- ADR 0020 file
- Crate plan: `laminapdf-sign` (core: CMS, PAdES, validation) and edge adapters (keychain/PKCS#11 in desktop, WebCrypto in web)
- Licence and audit status of crypto crates (RustCrypto: MIT/Apache)
- Trust anchors: OS stores on desktop, bundled (e.g. EUTL/AATL?) on web — decision
- TSA proxy contract (hash-only request, allow-listed TSAs, no logging of hashes)

**Out of scope** (and which todo covers it)

- Implementation (E10c onwards)

**Editor facts**

- **Crates and packages:** (decision) new `laminapdf-sign`
- **`SaveConstraint`:** no new save behaviour beyond the round's: signing makes the document `AppendOnly`; signing runs after any pending rewrite
- **Web vs desktop:** the ADR lists per-platform key storage
- **Server features:** TSA proxy Worker contract
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Survey pyHanko, PDFBox, DSS architectures
- [ ] Write the ADR
- [ ] Owner sign-off
- [ ] Index updates

## Acceptance criteria

- [ ] ADR accepted
- [ ] Open question on QES recorded with owner

## Post-merge checks

None.

## Tests

None (decision).

## Docs to update

- `docs/decisions/README.md`
- architecture §3
- `CLAUDE.md`
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Crypto crates must be audited or widely used; never hand-roll primitives (04 §13 item 7)

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
