# E10e — Trusted timestamps (B-T, document timestamps) and the optional TSA proxy Worker

| Field | Value |
| --- | --- |
| ID | `E10e` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [C4a](../done/C4a-ui-chrome-and-platform.md), [E10d](./E10d-pades-signing-core.md) |
| Dependency notes | [E10d](E10d-pades-signing-core.md), `C4` ([web-app §C4](../../plans/web-app.md#c4-app-shell-on-cloudflare): `@laminapdf/ui`, `apps/web`) |
| ADRs | Implements ADR 0020; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10e-timestamps-and-tsa-proxy` |
| PR title | `feat(sign): RFC 3161 timestamps and TSA proxy` |
| Size | M (200–600) |
| PR | — |

## Goal

RFC 3161 timestamp tokens as CMS unsigned attributes (PAdES B-T) and document timestamps (`/DocTimeStamp`, `ETSI.RFC3161`); core builds requests and parses/validates tokens, the edge sends them; a Cloudflare Worker proxy for TSAs that lack CORS, receiving only the hash.

## Context

- **Design (Paper):** [G02 Sign with a certificate](../../design/paper/sign-protect-redact.md#g02-sign-with-a-certificate). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- ADR 0020
- [04 §8.4 SubFilters](../../research/pdf/04-interactive-security.md#84-subfilters)
- [04 §8.5 PAdES baseline levels](../../research/pdf/04-interactive-security.md#85-pades-baseline-levels-etsi-en-319-142-1-current-v121-2024-01): DSS and document timestamps are ignored for DocMDP at every `P`
- [full-editor: optional server features](../../plans/full-editor.md#optional-cloudflare-server-features)
- RFC 3161

## Scope

**In scope**

- TS request/response in `laminapdf-sign`
- Edge transport: desktop HTTP, web fetch (direct or proxy)
- `apps/tsa-proxy` Worker (allow-listed TSAs, no logging of hashes, rate limits)
- Settings: TSA URL, proxy opt-in

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.06` trusted timestamps
- **Crates and packages:** `laminapdf-sign`, `apps/tsa-proxy` (new Worker)
- **`Op` variants:** `Sign { timestamp: Some(..) }`, `AddDocumentTimestamp` (barrier)
- **`SaveConstraint`:** `Sign { timestamp }` follows E10d (an approval signature: allowed without certification and under DocMDP P2/P3, a violation under P1). **`AddDocumentTimestamp` and DSS (LTV) updates are not DocMDP changes at any level, including P1**: ISO 32000-2 §12.8.2.2.1 and the 2.0 errata (§12.8.5.3) say incremental updates that only add DSS data and/or document timestamps are ignored when evaluating DocMDP ([04 §8.5](../../research/pdf/04-interactive-security.md#85-pades-baseline-levels-etsi-en-319-142-1-current-v121-2024-01)). So the planner treats a revision holding only a `/DocTimeStamp` signature field + dictionary and `/DSS` additions as always allowed: `would_violate` is empty on P1, P2, P3 and FieldMDP-lock fixtures, and the save stays `AppendOnly`. Anything else in the same revision is classified on its own and may still violate. Signing and timestamping always run last in a save (redact → rewrite → sign, ADR 0007 §4); both are barrier ops (ADR 0006 §5). The same rule is what [E10h](E10h-revision-diff-and-attacks.md)'s classifier applies when validating existing files (its "LTV" class)
- **Web vs desktop:** desktop calls TSAs directly; web uses the proxy only if the user opts in (sends the hash only)
- **Server features:** TSA proxy Worker — hash only, no document bytes, no accounts required
- **Capability-matrix rows to flip:** `F.06` → `round-trip` (core, pdf) (web and desktop columns flip in [E10j](E10j-signature-ui.md), with the UI)

## Tasks

- [ ] Core: `TimeStampReq` builder and `TimeStampResp`/token parser and validator in `laminapdf-sign`; B-T as the `signature-time-stamp` unsigned attribute; `AddDocumentTimestamp` writing a `/Type /DocTimeStamp` signature (`/SubFilter /ETSI.RFC3161`) in its own incremental revision
- [ ] Planner: a `ModificationClass`-independent "LTV" exemption for revisions that only add a document timestamp and/or DSS data, applied at every DocMDP level (ISO 32000-2 §12.8.2.2.1)
- [ ] Edge: desktop HTTP client, web fetch (direct or via proxy)
- [ ] Worker (`apps/tsa-proxy`) with tests
- [ ] Settings: TSA URL, proxy opt-in

## Acceptance criteria

- [ ] B-T signatures validate in EU DSS
- [ ] `AddDocumentTimestamp` on DocMDP P1, P2 and P3 certified fixtures: `would_violate` is empty, the save is `AppendOnly` and incremental, and the certification signature still validates (EU DSS, pyHanko) with the timestamp revision reported as an allowed LTV update
- [ ] `Sign { timestamp }` reports class `Signature` and `requires_rewrite = false`; a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2): violation only on P1 for `Sign`, none anywhere for `AddDocumentTimestamp`
- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Worker tests assert no body other than a TimeStampReq is forwarded and nothing is logged

## Post-merge checks

- [ ] `apps/tsa-proxy` deployed to production by the deploy workflow on `main`; a timestamp request through the production proxy to each allow-listed TSA succeeds, and its logs show no request bodies or hashes

## Tests

- Worker unit tests (Vitest + Miniflare)
- Validation tests

## Docs to update

- Privacy page: TSA proxy
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
