# U6b — `@laminapdf/fonts`: self-hosted, subsetted Newsreader and Source Sans 3

| Field | Value |
| --- | --- |
| ID | `U6b` |
| Status | `ongoing` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6a](../done/U6a-paper-tokens.md) |
| Blocks | [C4b](../ready/C4b-web-app-shell.md), [W1b](../ready/W1b-site-scaffold-and-deploys.md), [D2a](../ready/D2a-adr-0008-and-desktop-scaffold.md), [U6f](../ready/U6f-gallery-and-visual-tests.md) |
| ADRs | Follows [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md#fonts-icc-profiles-and-test-corpora) (bundled fonts OFL-1.1 or Apache-2.0 only, listed with their licence in the notices), 0024 (typography tokens; from [U1a](../done/U1a-adr-0024-app-shell-ux.md)) |
| Branch | `feat/u6b-self-hosted-fonts` |
| PR title | `feat(fonts): self-host subsetted newsreader and source sans 3 woff2 files` |
| Size | S (≈ 150–200 lines of script, CSS and tests; the font binaries and licence texts don't count) |
| PR | — |

## Goal

Ship Paper's two UI typefaces from lamina's own origin: Newsreader (titles, outline, quotes) and Source Sans 3 (every control and message), as subsetted WOFF2 files in a `@laminapdf/fonts` package with their OFL-1.1 licences, `@font-face` rules with the right `font-display`, and a preload for the UI face. No page of lamina may fetch a font from Google Fonts or any other third-party host at runtime: the canvases link `fonts.googleapis.com` only because they are mockups, and principle 7 ("prove, don't claim") means the network indicator must read 0 on a local file. The desktop app bundles the same files, so it works offline from first launch.

## Context

- [Paper design plan §7 risks](../../plans/paper-design.md#7-risks) — "Google Fonts in the canvases vs the privacy promise: U6b self-hosts subsetted fonts (OFL)".
- Design language [§3 Typography](../../design/paper/design-language.md#3-typography): families, weights (400, 500, 600; Newsreader optical sizes 6–72 and italic for quotes), fallbacks (`'Segoe UI', system-ui, sans-serif`; `Georgia, serif`), the mockup-only `<link>` line, the [type scale](../../design/paper/design-language.md#type-scale), and the rule that Source Serif 4 is for page mockups only; [§7.25 page-appearance chips](../../design/paper/design-language.md#725-page-appearance-chips) draw "Aa" in Source Serif 4 (see open questions).
- [ADR 0019, fonts](../../decisions/0019-supply-chain-and-licence-hygiene.md#fonts-icc-profiles-and-test-corpora) and [npm licence check](../../decisions/0019-supply-chain-and-licence-hygiene.md#npm-licence-check): bundled fonts are OFL-1.1 or Apache-2.0, each listed with its licence in the third-party notices.
- [UI/UX plan §5.7 trust surfaces](../../plans/ui-ux.md#57-trust-surfaces-r9r13) and [U4a](../ready/U4a-network-indicator-and-privacy-proof.md) (network indicator and a CSP-backed privacy test): a font CDN request would be counted and would break the "0 network requests" claim.
- The docs site uses only Source Sans 3 ([Q1a](../done/Q1a-docs-font-source-sans.md), [quality plan §5](../../plans/quality.md#5-the-font)), so when it moves to this package it imports only that family. Its headings use weight 700, so ship Source Sans 3's variable `wght` file or add a 700 instance.
- [U6a](../done/U6a-paper-tokens.md): the font family tokens (`--paper-font-sans`, `--paper-font-serif`, `--paper-font-page`, `--paper-font-mono`, `--paper-font-signature`; `FONTS` in TypeScript) this package must match by name.
- [Code organization §9](../../guidelines/code-organization.md#9-generated-code) (commit outputs that need special tools, with a reproducible generator) and [§10 scripts](../../guidelines/code-organization.md#10-scripts).
- Upstream: Newsreader by Production Type (Google Fonts repository `productiontype/Newsreader`), Source Sans 3 by Adobe (`adobe-fonts/source-sans`); both SIL Open Font License 1.1. The OFL allows subsetting and redistribution; a subset is a Modified Version, so it must not use a Reserved Font Name if the licence declares one (check each `OFL.txt`; neither is expected to, but record what you found).

## Scope

**In scope**

- `packages/fonts` (`@laminapdf/fonts`, private): `files/*.woff2`, `fonts.css` (`@font-face` rules with relative `url()` so Vite, the Tauri frontend build and the gallery emit them as hashed assets), `LICENSES/Newsreader-OFL.txt`, `LICENSES/SourceSans3-OFL.txt`, `fonts.lock.json` (upstream repository, release tag or commit, SHA-256 of each source file and of each output), `README.md`, and a typed export of the preload URL(s) for apps.
- Faces: Source Sans 3 400, 500, 600 (upright; italic only if a screen uses it); Newsreader 400, 500, 600 upright with the optical-size axis kept if the variable file fits the budget (else static instances at the sizes the type scale uses), plus Newsreader italic 400 for quotes.
- Subsets: **Latin** and **Latin Extended** as two `unicode-range` files per face (the browser downloads Latin Extended only when the text needs it), including the punctuation the writing style uses (’ “ ” · … – — €, tabular figures and the `tnum` feature, which must survive subsetting). Other scripts (Greek, Cyrillic, CJK, Arabic, Hebrew, Indic …) fall back to the system font stack per glyph, as the CSS fallbacks already say; document this in the README and keep the fallbacks in the family tokens.
- Glyphs outside the fonts that the UI uses: shortcut-chip symbols (⌘ ⌥ ⇧ ⌫ ↵ ↑ ↓) and arrows. Decide per glyph: in the subset if the font has it, otherwise the system font (test that each one renders with a non-zero width in three engines).
- `font-display`: `swap` for Source Sans 3 (text must show at once), `swap` for Newsreader too unless the layout shift on titles is visible in the gallery; `size-adjust` / `ascent-override` fallback faces (`"Source Sans 3 Fallback"` over local `Segoe UI`/`Arial`) to limit layout shift.
- Preload: `<link rel="preload" as="font" type="font/woff2" crossorigin href="…">` for Source Sans 3 Latin 400 only (the face on first paint); exported as a constant; [C4b](../ready/C4b-web-app-shell.md) (`apps/web`) and [D2a](../ready/D2a-adr-0008-and-desktop-scaffold.md) (desktop) depend on this todo and load the fonts.
- The subsetting script `tools/fonts/subset.sh` (or an xtask subcommand if it grows past ~50 lines) using fontTools `pyftsubset` with Brotli for WOFF2, pinned in `tools/fonts/requirements.txt` (fontTools is MIT, brotli is MIT); run by hand when fonts change, never by tests.
- Caveat 500 (OFL) for the typed-signature style ([design language §3](../../design/paper/design-language.md#3-typography); G01): subset to Latin, shipped as a separate WOFF2 that loads only when the signature dialog opens, never preloaded.
- A guard that no product file references a third-party font host.
- A size budget, checked by a test.

**Out of scope** (and which todo covers it)

- Fonts used to *render or generate PDFs* (Standard 14 substitutes, embedding) → library rounds ([B4c](../done/B4c-standard-14-and-simple-fonts.md)); this package is UI chrome only.
- The CSP `font-src 'self'` header and the network indicator itself → [C4c](../ready/C4c-remote-files-encryption-privacy.md), [U4a](../ready/U4a-network-indicator-and-privacy-proof.md); this todo adds a test that the gallery makes no third-party request.
- The generated third-party notices file shipped in apps → at first release ([ADR 0019 SBOM and notices](../../decisions/0019-supply-chain-and-licence-hygiene.md#sbom-and-notices-at-first-release)); this todo commits the licence texts and a README table it can be generated from.
- Using the fonts in components → [U6d](../done/U6d-core-components.md); screenshot stability with them → [U6f](../ready/U6f-gallery-and-visual-tests.md).
- Desktop app wiring → [D2a](../ready/D2a-adr-0008-and-desktop-scaffold.md) imports `@laminapdf/fonts/fonts.css` like the web app.

## Tasks

- [x] Claim: the work PR's first commit moves this file to `ongoing/` (git workflow §2; the claim-PR wording predates it).
- [ ] Download the upstream font sources from their official repositories (not from Google Fonts' CSS API) into a gitignored scratch folder; record repository, tag/commit and SHA-256 in `fonts.lock.json`; copy each `OFL.txt` verbatim into `LICENSES/` and note any Reserved Font Name.
- [ ] Licence check per ADR 0019: OFL-1.1 is on the allow-list for bundled fonts; if the npm licence check reads `package.json` `license`, set `"license": "OFL-1.1"` for `@laminapdf/fonts` and make sure the allow-list accepts it for this package (or document the exception in the xtask's licence config).
- [ ] Write `tools/fonts/subset.sh` (`set -euo pipefail`, `shellcheck` clean) and `requirements.txt`: one `pyftsubset` call per face × range with `--flavor=woff2 --layout-features='kern,liga,calt,tnum,lnum,case,ss01…'` (keep the features the type scale needs; check which the fonts have), `--unicodes` for Latin and Latin Extended, `--no-hinting` unless hinting visibly helps on Windows at 12 px (compare in the gallery), and name-table licence entries kept.
- [ ] Measure variable vs static files for both families; pick the smaller set that meets the budget and record the numbers in the README.
- [ ] Write `fonts.css`: one `@font-face` per file with `font-family` names matching [U6a](../done/U6a-paper-tokens.md)'s tokens, `font-weight` ranges for variable files, `font-style`, `unicode-range`, `font-display`; plus metric-adjusted fallback faces.
- [ ] Export `FONT_PRELOADS` (typed array of `{ href, type }` resolved with `new URL("./files/….woff2", import.meta.url)`) from `src/index.ts` with TSDoc.
- [ ] Add a guard test (in this package, scanning the repository's `packages/` and `apps/` sources) that fails on `fonts.googleapis.com`, `fonts.gstatic.com`, `use.typekit.net` or any `@import url(http…)` for fonts; `docs/design/` (the canvases) is excluded.
- [ ] Add a size test: total of all WOFF2 files and the preloaded file each under budget (see Acceptance).
- [ ] Add a glyph-coverage test: every character used in the UI message catalogue (`@laminapdf/ui`, once it exists; until then the design language's sample strings) is in the Latin subsets, or in an explicit "system fallback" list.
- [ ] README: faces, ranges, sizes, how other scripts fall back, how to regenerate, licence table (font, version, licence, copyright line) for the future notices file.
- [ ] Switch the docs site to `@laminapdf/fonts` (it themes Starlight with an interim copy from [U6g](../done/U6g-docs-site-paper.md)) and delete `apps/docs/src/fonts/` and the `@font-face` rules in `apps/docs/src/styles/paper.css`.

## Acceptance criteria

- [ ] `@laminapdf/fonts` ships WOFF2 files for Source Sans 3 (400/500/600) and Newsreader (400/500/600 + italic 400) in Latin and Latin Extended subsets, with both OFL texts and `fonts.lock.json`; re-running `tools/fonts/subset.sh` reproduces byte-identical files (or the README explains why not and the lock records output hashes).
- [ ] **Size budget** (recommendation; adjust once measured, in this PR, with the owner's agreement): the preloaded Source Sans 3 Latin 400 file ≤ 30 KB; all Latin files together ≤ 200 KB; Latin Extended files are loaded only when used (verified in the browser test by checking `document.fonts` after rendering Latin-only text).
- [ ] A Playwright (or Vitest browser) test loads a page using the fonts (the gallery stub or a test page) and records every request: all font requests are same-origin and there are **zero** requests to any other host.
- [ ] The guard test fails if a product source file references a font CDN (shown once in the PR description).
- [ ] Text in Greek, Cyrillic and CJK on the test page renders with system fallback fonts (non-zero glyph widths, no tofu in a screenshot attached to the PR).
- [ ] `cargo xtask check` passes, including `npm-licenses` with the fonts package's licence.

## Post-merge checks

- [ ] When the desktop app exists ([D2a](../ready/D2a-adr-0008-and-desktop-scaffold.md)), confirm its bundle contains the WOFF2 files and that a cold start offline renders Source Sans 3 (note it in D2a's done todo).

## Tests

`packages/fonts/test/*.test.ts`: CDN guard, size budget, lock-file hashes match the files, glyph coverage. A browser test (Vitest browser mode or Playwright in [U6f](../ready/U6f-gallery-and-visual-tests.md)'s gallery once it exists; a minimal HTML page until then) for same-origin loading, `unicode-range` splitting and system fallback.

## Docs to update

- `packages/fonts/README.md` (new).
- [Design language §3](../../design/paper/design-language.md#3-typography): next to the mockup `<link>` line, say that the product uses `@laminapdf/fonts` and never this link.
- `CLAUDE.md`: layout (`packages/fonts`, `tools/fonts/`) and the regenerate command (`tools/fonts/subset.sh`, needs Python with the pinned fontTools).
- [Paper design plan §9](../../plans/paper-design.md#9-to-dos): nothing to tick yet (the combined U6 line is ticked by [U6f](../ready/U6f-gallery-and-visual-tests.md)).

## Risks and open questions

- **Source Serif 4 in the page-appearance chips** ([§7.25](../../design/paper/design-language.md#725-page-appearance-chips)): shipping a third family for one "Aa" is wasteful. Recommendation: draw the chip preview in Newsreader (or ship a two-glyph Source Serif 4 subset, < 3 KB). The owner decides as part of [U6a](../done/U6a-paper-tokens.md)'s sign-off; this PR implements the answer.
- **Variable Newsreader with the optical-size axis** may be too large for the budget; static instances at 15, 16, 20 and 40 px lose little. Measure before deciding.
- **Unhinted fonts at 12 px on Windows** can look soft; compare in the gallery on Windows ([X6](../ready/X6-windows-ci.md) runners or a manual check) before choosing `--no-hinting`.
- **OFL Reserved Font Names**: if a family declares one, the subset must be renamed; check the licence texts rather than assuming.
- **Licence allow-list**: the npm licence check may not know `OFL-1.1` for an npm package; extend the allow-list in its single source of truth rather than skipping the package.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
