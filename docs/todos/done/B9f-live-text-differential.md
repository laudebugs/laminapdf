# B9f — Live text differential in the nightly lane: lamina's page text vs pdf.js and pdftotext over the external corpus

| Field | Value |
| --- | --- |
| ID | `B9f` |
| Status | `done` |
| Plan | [pdf-library §7 B9](../../plans/pdf-library.md#b9-text-extraction) |
| Round / milestone | Round B (library) — M2 Web reader v0.1 (hardening) |
| Depends on | [B9c](./B9c-text-extraction-bindings-and-differential.md), [X3a](./X3a-nightly-lane.md) |
| ADRs | [0018](../../decisions/0018-test-strategy.md) tier 6 (differential); [0003](../../decisions/0003-reader-tolerance-and-limits.md) (exception classes carry their rule) |
| Branch | `test/b9f-live-text-differential` |
| PR title | `test(text): live text differential in the nightly lane` |
| Size | M (≈ 300–500 lines of Rust, JS and TOML) |
| PR | [#111](https://github.com/laudebugs/lamina/pull/111) |

## Goal

Run B9c's text comparison on the external corpus every night, not only on the fixtures. B9c split this part off: its PR-lane differential (`cargo xtask differential --text`, precomputed `fixtures/differential/text.json`) meets B9c's acceptance criteria; the live nightly run needs a lamina facts runner for text and exception classes in the live harness, which is its own piece of work.

## Context

- [B9c](./B9c-text-extraction-bindings-and-differential.md): `tools/differential/text.mjs` (pdf.js `getTextContent` and pdftotext per page), the comparison rule (whitespace and bidi controls removed; `crates/lamina/tests/text_differential.rs`), `fixtures/differential/text-exceptions.json` (ASCII85Decode content, pdftotext's widget text, pdftotext's visual-order RTL).
- [X3a](./X3a-nightly-lane.md): `cargo xtask differential --live` (`tools/xtask/src/live.rs`), the runner that kills overrunning readers and reruns their files one per process (`tools/xtask/src/runner.rs`), exception classes in `fixtures/differential/exceptions.toml`.

## Scope

**In scope**

- A `text` fact in the live differential: lamina's page text per file (an example or reader binary like the existing lamina facts runner, with the same per-file time limits), pdf.js's and pdftotext's from the `tools/differential/` runners (reuse `text.mjs`'s logic; pdftotext as a reader under the runner).
- `cargo xtask differential --live --text` (or `--facts text`), with `--source` and `--sample`, and the nightly workflow's differential job running it.
- The comparison rule of B9c, per page; a page agrees when lamina matches at least one tool. Agreement rates per source in the job summary.
- Exception classes in `exceptions.toml` with their rule (for example annotation text only pdftotext extracts; unsupported filters; visual-order RTL), and `[[known]]` entries with issues for lamina bugs found.
- Fails like the existing live differential: a lamina panic, crash or hang, or a disagreement with every tool that no exception excuses.

**Out of scope**

- Quads on the external corpus (B9c's quad test stays on fixtures: PDFium boxes need a 1:1 mapping, which the corpus doesn't promise).

## Tasks

- [x] Load `/rust-best-practices`; read `tools/xtask/src/live.rs` and B9c's `text_differential.rs`
- [x] lamina text facts runner; pdf.js and pdftotext text readers under `runner.rs`
- [x] `--text` in `differential --live`; summary table; exceptions classes
- [x] Nightly workflow: the differential job runs the text facts
- [x] Run it locally on a sample (`--sample 50`) and file issues for lamina bugs found

## Acceptance criteria

- [x] `cargo xtask differential --live --text --sample 50` runs locally with every disagreement excused or listed as known with an issue
- [x] The nightly differential job reports text agreement per source in its summary

## Post-merge checks

- [ ] The first scheduled nightly run with text is green (link it in a follow-up PR or the done todo)

## Tests

- xtask unit tests for the text fact's comparison and exception matching.

## Docs to update

- [x] `CLAUDE.md` (the live differential's facts), `tools/differential/README.md`

## Risks and open questions

- pdftotext's whitespace and annotation handling may need several classes; keep each one's rule in ADR 0003 terms.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
