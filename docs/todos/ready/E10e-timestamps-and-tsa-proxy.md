# E10e — Trusted timestamps (B-T, document timestamps) and the optional TSA proxy Worker

| Field | Value |
| --- | --- |
| ID | `E10e` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [C4a](./C4a-ui-chrome-and-platform.md), [E10d](./E10d-pades-signing-core.md) |
| Blocks | [E10j](./E10j-signature-ui.md) |
| Dependency notes | [E10d](E10d-pades-signing-core.md), `C4` ([web-app §C4](../../plans/web-app.md#c4-app-shell-on-cloudflare): `@lamina/ui`, `apps/web`) |
| ADRs | Implements ADR 0020; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10e-timestamps-and-tsa-proxy` |
| PR title | `feat(sign): RFC 3161 timestamps and TSA proxy` |
| Size | M (200–600) |
| PR | — |

## Goal

RFC 3161 timestamp tokens as CMS unsigned attributes (PAdES B-T) and document timestamps (`/DocTimeStamp`, `ETSI.RFC3161`); core builds requests and parses/validates tokens, the edge sends them; a Cloudflare Worker proxy for TSAs that lack CORS, receiving only the hash.

## Context

- ADR 0020
- [04 §8.4 SubFilters](../../research/pdf/04-interactive-security.md#84-subfilters)
- [full-editor: optional server features](../../plans/full-editor.md#optional-cloudflare-server-features)
- RFC 3161

## Scope

**In scope**

- TS request/response in `lamina-sign`
- Edge transport: desktop HTTP, web fetch (direct or proxy)
- `apps/tsa-proxy` Worker (allow-listed TSAs, no logging of hashes, rate limits)
- Settings: TSA URL, proxy opt-in

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.06` trusted timestamps
- **Crates and packages:** `lamina-sign`, `apps/tsa-proxy` (new Worker)
- **`Op` variants:** `Sign { timestamp: Some(..) }`, `AddDocumentTimestamp` (barrier)
- **`SaveConstraint`:** signing makes the document `AppendOnly`; signing always runs last in a save (redact → rewrite → sign, ADR 0007 §4); `Sign` is a barrier op (ADR 0006 §5)
- **Web vs desktop:** desktop calls TSAs directly; web uses the proxy only if the user opts in (sends the hash only)
- **Server features:** TSA proxy Worker — hash only, no document bytes, no accounts required
- **Capability-matrix rows to flip:** `F.06` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Core
- [ ] Edge
- [ ] Worker with tests
- [ ] Settings

## Acceptance criteria

- [ ] B-T signatures validate in EU DSS
- [ ] Worker tests assert no body other than a TimeStampReq is forwarded and nothing is logged

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

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
