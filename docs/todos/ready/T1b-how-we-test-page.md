# T1b — A "How we test" page on the website, with the nightly's figures, the licence and where the code comes from

| Field | Value |
| --- | --- |
| ID | `T1b` |
| Status | `ready` |
| Plan | [trust and proof §T1](../../plans/trust-and-proof.md#t1-publish-the-proof) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T1a](./T1a-nightly-proof-figures.md) |
| ADRs | Follows [0011 §3](../../decisions/0011-cloudflare-topology-and-privacy.md#3-the-v01-privacy-guarantees-testable) (the website's rules: static data in the build, nothing from a third party, each claim tied to the rule or test behind it); states what [0001](../../decisions/0001-core-language-and-licence.md), [0004](../../decisions/0004-render-backend.md) (MuPDF prohibited) and [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (cargo-deny, the npm licence check) decide |
| Branch | `feat/t1b-how-we-test-page` |
| PR title | `feat(site): a How we test page with the nightly's figures, the licence and where the code comes from` |
| Size | M (≈ 300–450 lines: the snapshot command, the page and its components, the reader and tests, one workflow step; copy and design not counted) |
| PR | — |

## Goal

Publish what Lamina PDF's nightly measures, on a page at `laminapdf.com/how-we-test/`, with each number explained in plain English. The same page states the licence and where the code comes from. The figures come from [T1a](./T1a-nightly-proof-figures.md)'s record, through a committed snapshot that each production deploy refreshes, as `progress.json` is refreshed today.

On Hacker News, licences decide which PDF library people pick (about 150 comments), and warnings about AGPL in Ghostscript, MuPDF and iText recur in every year. In October 2026 a commenter asked of a new Rust engine: "Is this PDF engine vibe-coded and derived from reading the MuPDF source code?" ([50239092](https://news.ycombinator.com/item?id=50239092)). Another asked a library's author: "Do you have a comprehensive integration test suite that can validate the robustness of your implementation?" ([47214653](https://news.ycombinator.com/item?id=47214653)). Lamina PDF can answer both with evidence, and today it says neither ([report R1](../../research/hacker-news/pdf-users-on-hacker-news.md#r1-prove-correctness-in-public), [R9](../../research/hacker-news/pdf-users-on-hacker-news.md#r9-say-loudly-what-lamina-already-has)).

## Context

- [Report R1 and R9](../../research/hacker-news/pdf-users-on-hacker-news.md#r9-say-loudly-what-lamina-already-has); the [format and security note](../../research/hacker-news/notes/format-and-security.md) (AGPL traps, about 90 comments; its recommendation 7, the licence and its provenance up front) and the [extraction note](../../research/hacker-news/notes/extraction.md) (most parsers fail on part of a real corpus).
- [T1a](./T1a-nightly-proof-figures.md): the `kind: "proof"` record in `history.jsonl` on `bench-data`, its `Record` type in `tools/xtask/src/proof.rs`, and what each field counts.
- The pattern to copy, from [W1e](../done/W1e-progress-snapshot.md) and [W1c](../done/W1c-home-page.md):
  - `tools/xtask/src/progress.rs`: `run` writes `apps/site/src/data/progress.json`, reads from `origin/main` (`changes::at_revision`) and keeps the last snapshot's part it can't refresh, with a warning;
  - `apps/site/src/data/progress.ts`: the type and `readProgress`, which fails the build on a file it doesn't describe;
  - `apps/site/test/home.test.ts`: the page rendered from a fixture with Astro's container API;
  - `apps/site/src/data/copy.ts` (words the owner approved) and `src/data/sources.ts` (repository files read at build time, so nothing is typed twice).
- `.github/workflows/site.yml`: the production job runs `cargo xtask progress` before it builds, with `fetch-depth: 0`, which fetches every branch, `bench-data` included. `tools/xtask/src/workflows.rs`'s `site_production_should_refresh_the_roadmap_before_building` tests the order. The PR trigger's `paths` must list every file the site reads ([`apps/site/README.md`](../../../apps/site/README.md), "Deploying").
- The site's rules (`@laminapdf/site-kit`'s `siteChecks`: same-origin loads, no inline styles, a title, a description and a canonical URL, internal links that resolve); the header's links (`packages/site-kit/src/nav.mjs`: [design language §7.39](../../design/paper/design-language.md#739-site-and-product-pages-proposed)'s five) and `pageFiles`, the page list the header uses to show a link only once its page exists.
- What the licence section states, and where each fact lives: `license = "MIT OR Apache-2.0"` in the root `Cargo.toml`; `deny.toml`'s `[licenses] allow` list, its MPL-2.0 exception for UniFFI (the mobile bindings, ADR 0029) and its `[bans]` on `mupdf` and `mupdf-sys`; ADR 0004's "MuPDF is prohibited"; ADR 0019's checks; the docs page [What Lamina PDF writes and what it borrows](../../../apps/docs/src/content/docs/concepts/dependencies.mdx), which explains the same to developers.
- [Website plan §4](../../plans/website.md#4-the-website) (the page table) and [trust and proof §9](../../plans/trust-and-proof.md#9-open-questions), question 3: whether the proof lives on the website or in the docs.
- Pages this one links to rather than repeats: [U5a](./U5a-pricing-and-promises-page.md) (`/pricing/`, whose promises include "the code stays MIT OR Apache-2.0"), [U5b](./U5b-reader-benchmark.md) (`/performance/`, the reader benchmark), [C7c](./C7c-performance-gates-and-launch.md) (`/privacy/`) and [T1c](./T1c-publish-performance-numbers.md) (the docs site's benchmarks page).
- Design: [screens.md](../../design/paper/screens.md) and [account-cloud-product.md](../../design/paper/account-cloud-product.md#h12-website-home) (H12, the website's document sheet on the paper ground).

## Scope

**In scope**

- **`cargo xtask proof snapshot [--offline]`**, beside T1a's record in `tools/xtask/src/proof.rs` (a submodule if the file passes the 800-line budget). It reads `history.jsonl` from `origin/bench-data` and writes `apps/site/src/data/proof.json`, `version: 1`, in camelCase like `progress.json`:
  - `date`: the day the snapshot was written;
  - `night`: the newest `kind: "proof"` record with `full: true`, cut to the fields the page shows;
  - `nights`: over the last 30 full records, how many were recorded, how many passed (every job `success`), and the date of the first;
  - `fuzzHours`: the fuzzing time summed over every full record.

  With `--offline`, without the branch, or with no full record, it keeps the committed file and warns (`::warning::` under GitHub Actions). It also warns when the newest full record is more than three days old.
- **`apps/site/src/data/proof.ts`**: the `Proof` type and `readProof`, with `progress.ts`'s checks. A `null` section stays `null`.
- **The page**, `apps/site/src/pages/how-we-test.astro`, with its parts in `src/components/how-we-test/`. Each figure gets one or two sentences that say what was counted and what it means; for example, what "opened" covers (the file and its page tree read) and what it doesn't (every page drawn). The sections:
  - the files: the four projects' test suites (pdf.js, PDFBox, PDFium, qpdf) and the mutated copies, per source: opened, repaired on the way, refused, and refused while another reader opened them;
  - agreement with pdf.js, PDFium, qpdf and pypdf, and page text against pdf.js and pdftotext;
  - fuzzing: the targets, last night's executions, the hours since the first recorded night;
  - saves read back by qpdf, pdf.js and PDFium; the same bytes in each JavaScript runtime and in Rust;
  - veraPDF's verdict for each example and profile (PDF/A-2b, 2u, 3b, PDF/UA-1);
  - the render comparison of hayro with PDFium, and why Lamina PDF measures it;
  - what the figures leave out: the corpus is files chosen by four projects for being difficult, not a sample of everyday PDFs, and other limits the owner names;
  - the night's date, its commit (shortened, no link: the repository is private) and "N of the last 30 nights passed". A night where a check failed or a figure is `null` says so instead of showing a number.
- **The licence section**: MIT OR Apache-2.0; the allowed licences and the MPL-2.0 exception, read from `deny.toml` at build time through `src/data/sources.ts`; MuPDF banned, from `deny.toml`'s `[bans]`; what each check covers (cargo-deny and the npm licence check on every change check dependencies, not copied code); veraPDF used only as a separate program in tests.
- **Where the code comes from**: a statement the owner writes or approves. It says how the code is written: in Rust for Lamina PDF, from ISO 32000-2 and the other standards it cites. It names the part AI coding agents play, as the commit history shows, and says that every change is a reviewed pull request with tests. It links to the docs page on what Lamina PDF borrows.
- **Copy**: a `HOW_WE_TEST` export in `src/data/copy.ts` under the owner's approval comment, marked `[OWNER TO APPROVE]` until the owner approves it in the PR.
- **Links**: one line in the home page's "Where each part lives" (`Parts.astro`) and a link from the footer's licence line (`SiteFooter.astro`) to the licence section, both with owner-approved words. From the page: the docs site's dependencies page, T1c's benchmarks page once it exists there, and `/performance/`, `/pricing/` and `/privacy/` only once their page files exist (`pageFiles`).
- **`site.yml`**: production runs `cargo xtask proof snapshot` before it builds; the PR trigger's `paths` gain `deny.toml` and `Cargo.toml`, which the page now reads.
- **Design**: screen H13, "How we test", added to `screens.md` and `account-cloud-product.md`, built from H12's document sheet: light and dark, at phone and desktop widths.

**Out of scope** (and which todo covers it)

- The record itself → [T1a](./T1a-nightly-proof-figures.md).
- Extraction scores, generation timings and module sizes → [T1c](./T1c-publish-performance-numbers.md), on the docs site; this page links to them.
- The reader benchmark and the speed targets → [U5b](./U5b-reader-benchmark.md) and [U5a](./U5a-pricing-and-promises-page.md); the privacy promises → [C7c](./C7c-performance-gates-and-launch.md).
- A "How we test" link in the site header: §7.39 fixes five links, so adding one is a design decision (open question below).
- R9's Arabic, Devanagari and Chinese sample on the home page: not in this todo (see Risks).

## Tasks

- [ ] Claim: the first commit moves this file to `ongoing/`. Start once T1a's first full record is on `bench-data` (its post-merge check), so the first snapshot holds a real night.
- [ ] `cargo xtask proof snapshot`, its tests, and the first `proof.json`.
- [ ] `proof.ts` and its test.
- [ ] Design H13 from H12; add it to `screens.md` and `account-cloud-product.md`.
- [ ] Draft the copy with `/writing`; check each explanation against what the code counts (`live.rs`, `live/compare.rs`, `live/text.rs`, `render_diff.rs`, `harness.rs`, `roundtrip.rs`, `verapdf.rs`), and record the check in the PR.
- [ ] Ask the owner for the provenance statement, or for approval of a draft; check every claim in it against the repository (commit trailers, `deny.toml`, the ADRs) before asking.
- [ ] Build the page and its components; read `deny.toml` and the licence through `sources.ts`; the home page line and the footer link.
- [ ] `site.yml`'s production step and `paths`; the `workflows.rs` test.
- [ ] `/writing-review` on the copy; owner approval in the PR.

## Acceptance criteria

- [ ] The owner has approved the page's words, the provenance statement included, in the PR; `copy.ts` records the date.
- [ ] Every figure on the page comes from `proof.json`. `test/how-we-test.test.ts` renders the page from fixture snapshots and finds each figure, the night's date and the short commit. For a fixture with `render: null` and `render-diff` failed, it finds the sentence that says the figure is missing for that night, and no `0` in its place.
- [ ] The allowed licences on the page equal `deny.toml`'s `allow` list, and the banned crates its `[bans]` (a test over `sources.ts`).
- [ ] Every statement in the licence and provenance sections is checkable in the repository; the PR lists, per statement, the file or setting that shows it.
- [ ] `proof snapshot`'s tests: it takes the newest full proof record, skips other kinds and non-full records, counts passed nights over the last 30, sums the fuzzing hours, and keeps the committed file with `--offline`.
- [ ] `workflows.rs`: production runs `cargo xtask proof snapshot` before `pnpm --filter @laminapdf/site build`.
- [ ] The build passes the site's checks; axe finds nothing on the built page in light and dark at 375 and 1280 px in Chromium.

## Post-merge checks

- [ ] The production deploy serves `/how-we-test/` with the newest full night's figures, dated within a day of that nightly.
- [ ] A deploy run while `bench-data` can't be read keeps the committed snapshot and shows the warning in the job.

## Tests

`apps/site/test/proof.test.ts` (the committed file passes `readProof`; a wrong field fails, naming it), `apps/site/test/how-we-test.test.ts` (fixture snapshots: a night that passed, a night with a failed check, a `null` section), the `deny.toml` test; `proof.rs` snapshot tests over inline history lines; the `workflows.rs` test; the build's site checks and the axe run.

## Docs to update

- `apps/site/README.md`: the page, `proof.json` and how it is refreshed, the new PR trigger paths.
- [Website plan §4](../../plans/website.md#4-the-website): a `/how-we-test` row for T1b.
- [Git workflow §2](../../guidelines/git-workflow.md#pick-work-from-the-todo-board): a refresh of the proof snapshot is an exempt PR, like the progress snapshot's.
- `CLAUDE.md`: `cargo xtask proof snapshot` in the commands; `proof.json` and the page in the `apps/site` entry.
- [Trust and proof §9](../../plans/trust-and-proof.md#9-open-questions): record the answer to question 3.

## Risks and open questions

- **The provenance statement must be true.** Many commits carry `Co-Authored-By: Claude` trailers. A page that implies hand-written code would fail the very question it answers, so it describes the process as it is. Only the owner can approve it. Claims about origin stay to what the repository shows: a dependency check proves nothing about copied code, and the page says which check covers what.
- **Figures can embarrass** ([plan §8](../../plans/trust-and-proof.md#8-risks)). Publish them with their method; the owner approves what the page shows for a failed night.
- **Stale figures.** If the nightly stops recording, the page keeps showing the last night. The night's date sits beside the figures, and the snapshot command warns after three days.
- **Header link.** Whether "How we test" joins §7.39's five header links is the owner's call; until then the page is linked from the home page and the footer.
- **Where the proof lives** ([§9](../../plans/trust-and-proof.md#9-open-questions), question 3): this todo proposes the website, since the page speaks to people choosing a library. The owner decides with the copy; if the docs site wins, the same data and components move to `apps/docs`.
- **R9's script sample.** The report also suggests an Arabic, Devanagari and Chinese sample on the home page; the multilingual example's reference PDF exists on the docs site. That is a home page copy change for the owner, not part of this page.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
