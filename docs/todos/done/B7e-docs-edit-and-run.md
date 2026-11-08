# B7e — Docs: edit and run the TypeScript examples that teach by changing

| Field | Value |
| --- | --- |
| ID | `B7e` |
| Status | `done` |
| Plan | [Docs examples §2](../../plans/docs-examples.md#edit-and-run-b7e) |
| Round / milestone | Round B (library) — docs |
| Depends on | [B7d](./B7d-docs-example-results.md) |
| ADRs | Follows 0011 (same-site loads; one page with looser headers, recorded in this todo's PR), 0009 (the viewer), 0004 (PDFium in the browser) |
| Branch | `feat/b7e-docs-edit-and-run` |
| PR title | `feat(docs): edit and run the TypeScript examples on their pages` |
| Size | L (400–800 changed lines of code) |
| PR | — |

## Goal

Let a reader change one of six TypeScript examples on its page and see the new PDF at once, with nothing to install and nothing sent anywhere. An "Edit and run" button on the Result tab loads an editor and a sandboxed runner from the docs site itself, runs the docs' own `@laminapdf/sdk`, and draws the result with `@laminapdf/viewer`.

## Context

- [Docs examples plan](../../plans/docs-examples.md) §1 (the six examples: hello, invoice, images and vectors, links and outlines, element flow, PDF/UA report) and §2 (the design).
- [Research](../../research/docs-site/live-examples.md): MDN's live samples and Svelte's and Vue's playgrounds, the pattern this follows; why not StackBlitz or Sandpack.
- `apps/docs/public/_headers` and [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) §4: the headers every page keeps.
- `packages/viewer/README.md`: `ViewerClient` and `createViewer`.

## Scope

**In scope**

- The runner page (`/run/`), its headers, and the sandboxed iframe that hosts it.
- The editor (CodeMirror 6), type stripping (Sucrase), the run, errors, the viewer, Download and Reset.
- The six examples' inputs (fonts, images) served from the docs site.

**Out of scope**

- Rust examples, any example outside the six, a free-form playground, saved or shared code.

## Tasks

- [x] Check CodeMirror 6's and Sucrase's licences (`cargo xtask npm-licenses`) and add them to `apps/docs`.
- [x] `apps/docs/public/run/` (or a built page): loads `@laminapdf/sdk` with the text module, takes the code and inputs by `postMessage`, strips the types, runs the example's default export, and posts back the bytes or the `LaminaPdfError`. Its `_headers` entry adds `'unsafe-eval'` to `script-src` for that path alone.
- [x] The Result tab's "Edit and run" button: on first press, loads the editor, the iframe (`sandbox="allow-scripts"`, no `allow-same-origin`) and the viewer; Run, Reset, Download; errors with their code and hint.
- [x] Copy the SDK's and PDFium's WebAssembly into the docs build (`apps/docs` build step), and the six examples' input files.
- [x] Tests: the runner page with a fake SDK (messages in and out, an error); a browser test that runs `hello` with a changed string and checks the PDF's text.

## Acceptance criteria

- [x] Each of the six examples runs unchanged, and its bytes equal the golden PDF.
- [x] Changing a value and pressing Run shows the new PDF; a mistake shows the error's code and hint, not a stack trace.
- [x] A page with the button loads nothing extra until it is pressed; the button then loads only from the docs site.
- [x] Every page but the runner keeps ADR 0011's headers exactly; the runner's iframe can't read the docs site's storage.

## Post-merge checks

- [x] On the deployed docs, the six examples run in Chromium, Firefox and Safari.

## Tests

Node tests for the runner's message handling; a Playwright test in the docs' browser tests for one example end to end.

## Docs to update

`apps/docs/README.md`, the plan's to-do, ADR 0011 (an implementation note on the runner page's headers), CLAUDE.md's docs site line.

## Risks and open questions

- `'unsafe-eval'` on one page: the sandbox without `allow-same-origin` gives it an opaque origin, so its code can't reach the docs' storage or cookies. Ask the owner to accept the note in ADR 0011.
- First-press weight, about 3.5 MB: show a progress line while it loads.
- Examples that read files (fonts, images) must take them from the docs site; the runner passes them in as bytes, as the tests do.

## As built

- **A worker, not an iframe:** the runner is a module worker at `/run/worker.js` with its own CSP in `_headers` (`'unsafe-eval'` for `/run/*` alone). A worker's CSP comes from its own script's response, so every page keeps ADR 0011's headers, and the code it runs is the docs' own example as the reader edited it in their own tab. ADR 0011 has an implementation note.
- **hayro, not the viewer and PDFium:** the owner asked for our own libraries. `@laminapdf/render-hayro` draws each page in the worker, the same renderer as the Result tab's pictures, into canvases on the page; the first run loads about 3 MB gzipped (the SDK's text module and hayro).
- **Golden bytes:** the runner passes the SDK test suite's fixed options (`exampleOptions`, from `packages/sdk/test/suite/hello-options.ts`), so each example run unchanged gives its golden's bytes in Chromium, Firefox and WebKit.
- **Download as a data: URL:** Chromium checks a blob: download against the page's CSP, which lists no blob:, and reports a violation though the file arrives.
- **Buttons with icons:** Run, Reset and Download PDF carry `@laminapdf/icons`' play, reset and download icons, inlined at build time from the icons' SVG files (the package exports only a built `dist/`).
- **Built only with the WebAssembly:** `scripts/build-runner.mjs` builds the runner before `astro dev` and `astro build` when `packages/wasm/dist/` and the SDK and render packages are built; otherwise the pages show no button. The docs workflow builds only the site, so the deployed docs show the button once `.github/workflows/docs.yml` runs `cargo xtask web docs-runner` (the WebAssembly, the SDK and the hayro packages) before the docs build; the PR gives that change for the owner to commit, since agents can't push workflows.
- **One e2e server:** `@laminapdf/test-utils/wrangler-dev` is `apps/web`'s e2e server and HTTPS helper, now shared by both sites' suites. `cargo xtask pdf-runtimes <browser>` builds the docs and runs theirs after the reader's.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
