# U5b — Reader benchmark: lamina vs other PDF readers

| Field | Value |
| --- | --- |
| ID | `U5b` |
| Status | `ready` |
| Plan | [UI/UX §6 U5. Product promises](../../plans/ui-ux.md#u5-product-promises) |
| Round / milestone | Round U (UI/UX) — after M2 |
| Depends on | [C7c](../done/C7c-performance-gates-and-launch.md), [W1b](../done/W1b-site-scaffold-and-deploys.md), [D7c](./D7c-desktop-beta-release.md) |
| ADRs | Follows 0018 (test strategy) for reproducibility |
| Branch | `test/u5b-reader-benchmark` |
| PR title | `test(bench): add a reproducible benchmark against other pdf readers` |
| Size | M (scripts and docs; ~300 lines) |
| PR | — |

## Goal

Users quote SumatraPDF-class speed as the bar and complain about Acrobat's cold start, but no public benchmark compares readers. Measure lamina against pdf.js, the Chrome viewer, macOS Preview, SumatraPDF and Acrobat Reader on the same files and machines, and publish the method and raw numbers.

## Context

- [Hacker News research R1 and R6](../../research/hacker-news/pdf-users-on-hacker-news.md#r1-prove-correctness-in-public): speed is the most praised reader trait and slowness the second-largest complaint (470 comments together); [T1b](./T1b-how-we-test-page.md)'s "How we test" page should link the benchmark
- [T1b](./T1b-how-we-test-page.md) publishes the nightly's robustness figures at `/how-we-test/`, and [T1c](../done/T1c-publish-performance-numbers.md) the library's timings, extraction speed and module sizes on the docs site's benchmarks page: `/performance/` links to both rather than repeating them.
- **Design (Paper):** [H09 Benchmark page](../../design/paper/account-cloud-product.md#h09-benchmark-page). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7) — targets; [research gaps](../../research/product/pdf-app-product-and-ux.md#several-primary-sources-were-unreachable-and-the-gaps-matter) — no competitor benchmarks found.
- [Competitive teardown](../../research/product/notes/competitive-teardown.md) — Acrobat footprint and iPad drawing reports; Sumatra praise.
- [C3c](../done/C3c-viewer-perf-harness.md) — lamina's own harness; [D7c](./D7c-desktop-beta-release.md) — the signed, packaged desktop beta, so cold start and installer size are measured on a release build, not a dev build.

## Scope

**In scope**

- Reference files: 10-page text, 1,000-page textbook, vector-heavy architectural drawing, 500 MB scan, tagged multi-column paper (permissive licences; provenance recorded).
- Metrics: cold start to first page, warm open, scroll through 100 pages (dropped frames or long tasks), peak memory, installer/app size.
- Method per reader: scripted where possible (Playwright for browser viewers, OS automation for desktop apps), otherwise a screen recording with frame timing; three runs, median reported; machine specs and versions recorded.
- `bench/readers/` scripts, a results JSON, and the website's `/performance` page (`apps/site`, [website plan §4](../../plans/website.md#4-the-website); shared with U5a's performance targets if it exists).

**Out of scope** (and which todo covers it)

- CI gating on these numbers → lamina's own gates stay in [C7c](../done/C7c-performance-gates-and-launch.md) and D7c.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Choose machines (one mid-range Windows laptop, one Apple-silicon Mac) and versions; record them.
- [ ] Script each reader; fall back to recordings with a documented frame-timing method.
- [ ] Run, collect raw data, compute medians; publish with caveats.

## Acceptance criteria

- [ ] Every number comes with raw data, versions, machine and method; another person can rerun it from the README.
- [ ] Competitor licence terms allow benchmarking and publishing results (checked and noted per product).

## Post-merge checks

None.

## Tests

The scripts themselves; no CI gate.

## Docs to update

`bench/readers/README.md`; the website's `/performance` page; tick the benchmark item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Some EULAs restrict publishing benchmarks: check each and omit products that forbid it.
- Results age quickly: date every run and rerun before each major release.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
