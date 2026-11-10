# C7e — Automated web reader releases, apart from the packages

| Field | Value |
| --- | --- |
| ID | `C7e` |
| Status | `done` |
| Plan | [web reader §6 C7. Accessibility, offline and launch](../../plans/web-app.md#c7-accessibility-offline-and-launch) |
| Round / milestone | Round C (web reader) — after M2 Web reader v0.1 |
| Depends on | [C7c](./C7c-performance-gates-and-launch.md), [P6](./P6-daily-release.md) |
| ADRs | Amends 0011 (production from `web-v*` tags the release train cuts) |
| Branch | `ci/c7e-web-in-daily-release` |
| PR title | `ci(web): release the web reader automatically, once a day when it changed` |
| Size | M (300–600 lines incl. tests and the workflow files) |
| PR | [#389](https://github.com/laudebugs/laminapdf/pull/389) |

## Goal

The web reader releases itself. Once a day, after the full nightly passes on `main`, a train tags `web-v<next>` on the nightly's commit when what the reader ships changed since its last release, and that tag deploys it through C7c's release gates. The owner decided on 2026-11-10 that the reader keeps its own version, apart from the packages' daily release.

## Context

- `web-deploy.yml` already deploys production from a `web-v*` tag on a commit of `main` (C4d), behind C7c's release gates; until now the owner pushed each tag by hand.
- The package train (`release-train.yml`, P6) runs after each full nightly and uses the release bot, whose pushes start other workflows (the default token's don't).

## Scope

**In scope**

- `cargo xtask web next-release`: what ships changed since the newest `web-v*` tag (the reader's inputs, less docs, Markdown, `tools/` and the workflows), the next version (`package.json`'s when it is newer, else the last tag's patch + 1; behind another minor is refused), and the notes (one line per commit that changed what ships).
- `cargo xtask web check-tag`: the deploy refuses a tag whose version `package.json` doesn't allow.
- `tools/release/web-release-train.yml` (for the owner to install): the package train's nightly checks, then `next-release`, then an annotated tag holding the notes, pushed by the release bot. `WEB_RELEASE_TRAIN=off` stops it.
- A `web-deploy.yml` patch (on top of C7c's): `check-tag`, the build with the tag's version (`LAMINA_PDF_WEB_VERSION`, read by `vite.config.ts`), and the GitHub release from the tag's notes once the smoke test passes.

**Out of scope**

- Release notes beyond commit subjects, and any link to the package releases.

## Acceptance criteria

- `cargo test -p xtask` passes with the workflows installed: the version rules, what ships, and the train's and deploy's wiring.
- With the train installed, a nightly after a reader change produces `web-v<next>`, `app.laminapdf.com` shows it in Settings › About, and its GitHub release lists the changes; a nightly with no reader change tags nothing.

## Post-merge checks

- The owner installs `tools/release/web-release-train.yml` and applies the `web-deploy.yml` patch from the PR (after C7c's).
- The first nightly after the next reader change releases it, as above.

## As built

- `tools/xtask/src/web_release.rs`: `web next-release [--commit] [--since] [--notes]` and `web check-tag`.
  - **What ships:** the reader's inputs, the xtask WebAssembly build, `web.rs` and `web-deploy.yml`. Docs, Markdown and the other tools and workflows don't count.
  - **Version:** `apps/web/package.json`'s when newer than the newest `web-v*` tag, else that tag's patch + 1. A `package.json` behind another minor is refused. `ready=false` is reported before any version is worked out, so an odd tag can't fail a quiet night.
  - **Notes:** one `git log --first-parent -m --name-only`, a line per commit that changed what ships.
  - **check-tag:** refuses a tag off `package.json`'s major and minor, or below another tag.
- `.github/workflows/web-release-train.yml` (installed by the owner from `tools/release/`):
  - It uses the package train's nightly checks.
  - `--since` is the newest web-v tag with a GitHub release, so a tag whose deploy failed is made up by the next one. The lookup pages through every release, since the packages' daily releases are there too.
  - The release bot pushes an annotated tag holding the notes. `WEB_RELEASE_TRAIN=off` stops it.
- `web-deploy.yml` (owner's commit):
  - `check-tag` before the build.
  - `LAMINA_PDF_WEB_VERSION` from the tag, which `vite.config.ts` puts in Settings › About.
  - After the smoke test, a GitHub release from the tag's annotation, read through the API, or generated notes for a hand-pushed tag.
- Tests:
  - the version rules, `ships`, the argument parser;
  - the train's gating and bot tag, and that the installed copy matches the reviewed one;
  - the deploy's `check-tag`, versioned build and GitHub release.
- Reviews: high (10 findings) and low (1), all fixed; the fix commits list them.
