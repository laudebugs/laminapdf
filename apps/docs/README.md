# `apps/docs`: the docs site

Lamina PDF's documentation site ([todo B7a](../../docs/todos/done/B7a-docs-site.md)): [Astro](https://astro.build/) with [Starlight](https://starlight.astro.build/), served from Cloudflare Workers static assets. It has quick starts per runtime, guides, recipes (one per example) and troubleshooting, the `@laminapdf/sdk` API reference (TypeDoc) and a page on the Rust API (rustdoc).

```sh
pnpm install                        # once per worktree
pnpm --filter @laminapdf/docs dev      # http://localhost:4321 (no CSP in dev)
pnpm --filter @laminapdf/docs build    # dist/: what deploys; fails on a broken internal link
pnpm --filter @laminapdf/docs typecheck
```

`cargo xtask check ts` runs the build (the fast lane's TypeScript job), so a broken link, a snippet that names a missing file, or TypeDoc failing on `@laminapdf/sdk` fails CI.

## Build settings

Two environment variables change what the build writes (`astro.config.mjs`):

| Variable | Default | Effect |
| --- | --- | --- |
| `LAMINA_PDF_ENV` | unset | `production` builds the released docs for `docs.laminapdf.com`: the header names the version, and search engines may index the pages. Every other build (`next.docs.laminapdf.com`, the previews, a local build) says on every page that it describes unreleased changes, links to the released docs, and sends `X-Robots-Tag: noindex` with every response. |
| `LAMINA_PDF_DOCS_TYPEDOC` | on | `0` leaves the TypeDoc reference out. `typecheck` uses it, so each lane runs TypeDoc once, in the build. |

## Look

The site uses Lamina PDF's [Paper design language](../../docs/design/paper/design-language.md), the look of the apps:
- **Ground and accent:** warm paper grounds and ink text, with one deep-teal accent.
- **Type:** Source Sans 3 for everything, titles included; weight and size carry the hierarchy ([quality plan §5](../../docs/plans/quality.md#5-the-font)).
- **Themes:** Starlight's light theme is Paper's light theme (§2.1), and its dark theme is Paper's proposed dark theme (§2.3).

The sidebar is an explicit list in `astro.config.mjs`, in the order of the [quality plan §3](../../docs/plans/quality.md#3-the-docs-as-a-story): a new page appears only once it is added there, and pages' `sidebar.order` frontmatter has no effect. A page that moves keeps its old URL through `redirects` in the same file (static redirect pages, since Workers static assets ignore `_redirects`).

`src/styles/paper.css` (Starlight's `customCss`) maps the design language's tokens (`--paper-*`) onto Starlight's `--sl-*` properties:
- **Colours:** asides use Paper's accent, Plum, `warning` and `danger`, and the sidebar marks the current page as a selected row.
- **Fonts:** self-hosted from `@laminapdf/fonts` (the CSP allows fonts from `'self'` only); their provenance and OFL licences are in [its README](../../packages/fonts/README.md).

When the design language's tokens change, change them here too.

## How the pages get their content

- **Snippets are the tested files.** `src/components/Snippet.astro` reads a file of the repository at build time (`<Snippet file="packages/sdk/examples/hello.ts" />`). It can show only the files its `import.meta.glob` lists: the TS examples and the Worker example in `packages/sdk/examples/`, the runtime runners in `packages/sdk/test/`, and the Rust examples in `crates/laminapdf/examples/`. All of them run in CI, so the code on the site is code that runs. Don't paste code into a page: add or extend an example and show that.
- **Examples show what they make.** `src/components/Example.astro` shows an example in one row of tabs: its code (`ts`, `rust`) in the language tabs, then a Result tab that shows a picture of every page and a link to the PDF, for an example that builds one (`result="hello"`); an Output tab shows the text one prints (`output="fixtures/extract/examples/invoice.md"`). Pages open on the code; the row remembers only the reader's language, never Result or Output. The Markdown twins write the code, a Result line with each PDF's link, and the Output. The pictures are `public/examples/preview/`, written by `cargo xtask docs-previews` from `fixtures/golden/examples/` (hayro, 144 dpi) with `manifest.json`; `cargo xtask check docs` fails when an example's PDF changes and its pictures don't. After changing an example, run `cargo xtask golden --update`, then `cargo xtask docs-previews`.
- **Six examples can be edited and run.** The TypeScript files in `runner/examples.ts` (hello, invoice, images and vectors, links and outlines, element flow, PDF/UA report) get an "Edit and run" button on their Result tab. Pressing it loads CodeMirror (`runner/editor.ts`); Run sends the code to the runner, a module worker at `/run/worker.js` (`runner/worker.ts`). The runner strips the types with Sucrase, runs the example against the docs' own `@laminapdf/sdk` with the inputs the SDK's tests pass (`runner/run.ts`), and draws the first 20 pages with `@laminapdf/render-hayro`. A run that takes more than 30 seconds is stopped, and the next run starts a new worker. `scripts/build-runner.mjs` builds it into `public/run/` (gitignored) before `astro dev` and `astro build`, but only when `packages/wasm/dist/` and the SDK and render packages are built; otherwise the pages show no button.
  Tests: `runner/run.test.mjs` checks the runner with a fake SDK, and `pnpm --filter @laminapdf/docs e2e` (Vitest and Playwright against `wrangler dev`, after a build; `cargo xtask pdf-runtimes <browser>` runs it) runs the six in a browser.
- **Diagrams** are inline SVG, coloured only by the `--paper-*` tokens (`src/styles/diagrams.css`), so each follows the light and dark themes and forced colours. The components in `src/components/diagrams/` draw them from a shared kit (`kit/`: box, arrow, lane, label, the arrowheads, the SVG wrapper and the `DrawingProps` every drawing takes). A page places one with `<Diagram name="open-edit-save" />`; `diagrams/registry.mjs` holds each diagram's caption and text description, which the page's Markdown twin shows instead of the picture. A diagram draws a wide layout and a narrow one for columns under 40rem. To add one: an entry in the registry, a drawing named after it (`open-edit-save` → `OpenEditSave.astro`), then `pnpm --filter @laminapdf/docs test` (no hard-coded colours, ids from the prefix, every name has a drawing) and `pnpm --filter @laminapdf/docs test:browser` (contrast in three themes and with forced colours, labels at 12 px or more). The rules for readers are in [docs writing](../../docs/guidelines/docs-writing.md#draw-a-diagram-where-a-picture-answers-the-question-faster).
- **The code table** (`src/components/CodeTable.astro`) reads `fixtures/codes.json`. Each group whose key matches the group pattern in `fixtures/codes.schema.json` appears in the file's order, with its title made from its key (`textWarnings` becomes "Text warnings"). A new group needs no change here.
- **The benchmarks page** (`/reference/benchmarks/`) reads `src/data/benchmarks.json` through `src/lib/benchmarks.ts`, which fails the build on a file it doesn't describe; `src/components/Benchmarks.astro` draws each section, and the page's Markdown twin gets the same tables from `sectionMarkdown`. `cargo xtask benchmarks` writes the file. To refresh it before a release, on an idle machine: `cargo xtask wasm`, `cargo xtask extract-bench` and `cargo xtask extract-bench --tables lattice-and-stream`, then `cargo xtask benchmarks`, and commit the file in its own PR.
- **The code pages** (`/reference/codes/<code>/`, `src/pages/reference/codes/[code].astro`) are the `docs/errors/<code>.md` pages, loaded by `plugins/error-pages.mjs`. A link to another code's page points to its page on the site. Other relative links (ADRs, todos) keep their text but lose the link, because the site doesn't have those files. The table links each code to its page. `codes.json`'s `docsUrl` (the repository pages) is unchanged.
- **The API reference** is generated by [starlight-typedoc](https://github.com/HiDeoo/starlight-typedoc) from `packages/sdk/src/index.ts` into `src/content/docs/reference/sdk/` (gitignored) on every build. TypeDoc 0.28 needs the TypeScript compiler API, which TypeScript 7 doesn't have, so this package uses TypeScript 6 from the `typedoc` catalog in `pnpm-workspace.yaml`. Without `packages/wasm/dist/`, the wasm-bindgen records show by name only (`skipErrorChecking`).
- **The example PDFs** are the goldens in `fixtures/golden/examples/`, copied into `dist/examples/pdf/` after the build (`plugins/integrations.mjs`).
- **The icons** are the website's: `favicon.svg`, `favicon-32.png` and `apple-touch-icon.png` from `apps/site/public/`, copied to the root of `dist/` after the build and linked from every page (`WEBSITE_ICONS` in `plugins/integrations.mjs`). To change the icon, change the website's files.
- **Links the validator can't see:** `starlight-links-validator` checks the Starlight pages. After the build, `plugins/integrations.mjs` checks every link on every built page to `/examples/pdf/…` and `/reference/codes/…` against the files the build wrote.

## The site header

The docs site shows the header the website shows, from [`@laminapdf/site-kit`](../../packages/site-kit/README.md#the-site-header): the wordmark links to https://laminapdf.com, and "Docs" is marked on every page. `src/components/Header.astro`, Starlight's `Header` override, keeps Starlight's search, social icons, theme switch and language switch beside the shared links. The menu button stays where Starlight draws it. A link to a website page appears once the page exists: `astro.config.mjs` reads `apps/site/src/pages/` at build time. Below Starlight's 50 rem breakpoint the header shows only search and the menu button. The menu lists the links instead, above the theme switch, from `src/components/MobileMenuFooter.astro` (Starlight's `MobileMenuFooter` override).

## Headers

`public/_headers` applies [ADR 0011 §4](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#4-security-headers-appswebpublic_headers-for-)'s headers to every response, with these differences for a static docs site:

- **`script-src` lists hashes.** After the build, `@laminapdf/site-kit` (the build steps both sites share) replaces `'inline-script-hashes'` with the SHA-256 of every inline script the build wrote (Starlight's theme and tabs, Expressive Code's copy button), so there is no `'unsafe-inline'` for scripts. `'wasm-unsafe-eval'` is for search (Pagefind). The build fails if the CSP line passes 1,900 characters, because Cloudflare's limit for a `_headers` line is 2,000.
- **`style-src` is `'self'` alone.** Expressive Code colours code tokens with style attributes, and Starlight writes a few more. After the build, `@laminapdf/site-kit`'s `inlineStylesToClasses` turns each distinct style into a class in one style sheet (about 40 rules), and rewrites Expressive Code's selector on the style attribute to match. Astro writes style sheets as files (`build.inlineStylesheets: "never"`). The build fails on an inline `<style>` element, or a selector on the style attribute it can't translate. `e2e/styles.e2e.test.ts` checks the pages in a browser for coloured code, working search and no CSP violation.
- **`connect-src` is `'self'` only.**

The example PDFs drop the CSP, so the browser's PDF viewer can open them. The example runner (`/run/*`) swaps the CSP for its own: the same `'self'`-only sources plus `'unsafe-eval'`, which it needs to run a reader's edit. A worker's CSP comes from its own script's response, so no page gets `'unsafe-eval'`. Every build but the released docs' adds `X-Robots-Tag: noindex` (`LAMINA_PDF_ENV`, above). `apps/docs/biome.json` turns off Biome's unused-import and unused-variable rules for `.astro` files, because Biome sees only their frontmatter and not the template that uses them. It also allows the restricted globals (Node's `process`, the DOM) in `astro.config.mjs` and `plugins/`, which run only in the Node build, in `runner/editor.ts`, which runs only in a page, and in the e2e tests.

## Versions

Every page names the version it documents: the fixed group's version (`packages/sdk/package.json`, ADR 0017's one lockstep version), read at build time (`plugins/release.mjs`). On `main` that version is always a published one: the pull request that bumps it merges only after its release is published. So the unreleased docs name the newest release too.

- **The header** shows the version on the released docs, linked to the changelog, and "Unreleased" on every other build, linked to the released docs.
- **The notice** (`src/components/Banner.astro`, Starlight's `Banner` override) says above every unreleased page that it describes changes that aren't released yet, and links to the released docs.
- **Pages write `{{version}}`** where they name the version: install lines (`npm install @laminapdf/sdk@{{version}}`, `cargo add laminapdf@={{version}}`) and docs.rs links. The build fills it in on the pages and in their Markdown twins. On an `.mdx` page it may stand anywhere, prose and links included, because a Vite plugin fills it in before MDX reads the page. The `.md` pages (the TypeDoc reference, the changelog) can't use it. The website copies the install page's two install lines and fills in the same placeholder (`@laminapdf/site-kit/version`).
- **The build fails** (`releaseChecks`) on:
  - a placeholder left unfilled;
  - an install command that names another version, the `alpha` tag aside, on any page but the changelog (a page of history);
  - a released page without the version in its header, or an unreleased page without the notice;
  - a `_headers` file that sends `X-Robots-Tag: noindex` on the released docs, or doesn't on any other build;
  - a `robots.txt` that blocks crawlers, that names no sitemap on the released docs, or that names one on any other build.

Every build lets crawlers in (`@laminapdf/site-kit/robots`), so they can read the `noindex` on unreleased docs; only the released docs name `sitemap-index.xml`.

## Deploying

`wrangler.toml` describes two Workers, both static assets with no script, and `.github/workflows/docs.yml` deploys them:

- **`lamina-pdf-docs`**, at **https://docs.laminapdf.com** (a Workers custom domain): the latest release's docs, with `preview_urls` on for the per-PR previews on workers.dev. It has no workers.dev address of its own, which would serve the released docs a second time without `noindex`; the website's Worker and the reader's preview Worker run with the same pair of settings.
- **`lamina-pdf-docs-next`** (`[env.next]`), at **https://next.docs.laminapdf.com**: `main`'s docs, unreleased and noindex.

The workflow runs for changes to the site or anything it's built from (`apps/docs/`, `packages/sdk/`, the Rust examples and their goldens, `codes.json` and its schema, `docs/errors/`, the workspace's `package.json`, lockfile, `pnpm-workspace.yaml`, `tsconfig.base.json` and `.nvmrc`):

- **`preview-build`** (pull requests): builds the PR's site with no secrets and uploads `dist/` as an artifact.
- **`preview-deploy`** (pull requests): holds the secrets and never checks out or runs PR code. It reads `wrangler.toml` and the Wrangler version from the default branch, installs that exact Wrangler with `npx` into a fresh npm cache, and uploads the artifact as a preview version (`--preview-alias pr-<n>`). Then it posts one comment with the URL and updates it on each push. It skips with a notice until the default branch has `apps/docs/wrangler.toml`.
- **`next`** (pushes to `main`, or a manual run on `main` without a tag): builds `main` as unreleased docs and deploys them in one job (`wrangler deploy --env next`), because `main` is trusted.
- **`production`** (a release tag): checks out the tag, checks that it names the docs' version, builds with `LAMINA_PDF_ENV=production` and deploys `docs.laminapdf.com`. The release lane calls it with the tag once the packages are published; a manual run with a tag (Actions → Docs site → Run workflow → `tag`) redeploys a release by hand.
- **Dry runs:** with the `dry-run` input, `next` and `production` build and check the site, then run `wrangler deploy --dry-run`, which deploys nothing and needs no secrets. A manual run can set it, and so do the release lane's own dry runs.
- **One deploy at a time:** production deploys share one concurrency group whatever the tag, and `next` has its own, so two deploys of one Worker never overlap.
- **Forks and missing secrets:** PRs from forks are skipped. Without the secrets, every deploy step skips with a notice and the jobs pass.

### What the owner needs to do

1. In Cloudflare, create an API token (My Profile → API Tokens → Create Token → Custom token) with:
   - **Account → Workers Scripts → Edit**: uploads versions and deploys the Worker, including its workers.dev route and preview URLs.
   - **Account → Account Settings → Read**: lets Wrangler read the account it deploys to.

   - **Zone → Workers Routes → Edit** on the `laminapdf.com` zone (Zone Resources → Include → Specific zone): attaches the custom domain `docs.laminapdf.com`. If the deploy still fails with a permission error on the domain, add **Zone → DNS → Edit** on the same zone (Cloudflare creates the `docs` record).

   Scope it to the one account (Account Resources → Include → that account). Optionally, set a TTL and restrict client IPs.
2. Add two repository secrets (Settings → Secrets and variables → Actions): `CLOUDFLARE_API_TOKEN` (the token) and `CLOUDFLARE_ACCOUNT_ID` (the account ID, shown on the account's Workers overview page).
3. After this site is on `main`, run the **Docs site** workflow there once (Actions → Docs site → Run workflow). The first `wrangler deploy` creates the Worker, and previews need it to exist. The account also needs a workers.dev subdomain, which the dashboard asks for on first use of Workers.
4. The custom domain: `laminapdf.com` is a zone on the same Cloudflare account, and `wrangler.toml` routes `docs.laminapdf.com` to the Worker (`custom_domain = true`). The next production deploy creates the DNS record and certificate; it fails if the token lacks the zone permission above or if a DNS record for `docs` already exists (delete it first, or attach the domain once in the dashboard: Worker → Settings → Domains & Routes).
5. `next.docs.laminapdf.com`: the first push to `main` after this setup creates the Worker `lamina-pdf-docs-next`, its DNS record and its certificate, with the same token. A Workers custom domain gets its own certificate, so the two-level name needs no advanced certificate ([Cloudflare's custom domains](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/)).
6. From then on, a push to `main` no longer changes `docs.laminapdf.com`. Until the release lane deploys it after each release, put the latest release's docs there by hand: run the **Docs site** workflow with `tag` set to that release's tag (for example `v0.2.0-alpha.2`).
