# `apps/site`: the website at laminapdf.com

Lamina PDF's website is a static [Astro](https://astro.build/) site served from Cloudflare Workers static assets at **https://laminapdf.com** ([website plan §4](../../docs/plans/website.md#4-the-website), [todo W1b](../../docs/todos/done/W1b-site-scaffold-and-deploys.md)). It holds pages of text in the [Paper design language](../../docs/design/paper/design-language.md), and every build checks each page against the website's rules in [ADR 0011 §3](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#3-the-v01-privacy-guarantees-testable). For now the home page is the roadmap: what Lamina PDF is, a first PDF in code, the milestones, a dashboard of the todo board and the packages.

```sh
pnpm install                          # once per worktree
pnpm --filter @laminapdf/site dev        # http://localhost:4321 (no headers in dev)
pnpm --filter @laminapdf/site build      # dist/: what deploys; fails on a page that breaks the rules
pnpm --filter @laminapdf/site test       # the headers against ADR 0011 §4, the header's links, the home page from a fixture
pnpm --filter @laminapdf/site typecheck
pnpm --filter @laminapdf/site share-image  # rewrites public/og.png and the PNG favicons (needs Playwright's Chromium)
```

`cargo xtask check ts` runs the build, the tests and the typecheck, so CI fails on a page that breaks the rules.

## What every build checks

After each build, `astro.config.mjs` runs three steps from `@laminapdf/site-kit` on `dist/`, in this order:

1. **The website's rules** (`siteChecks`). The build fails, naming each page and problem, when a page:
   - loads a file the build didn't write, a file from another origin, or a file from its own origin by full address, which a preview's `'self'` refuses because the preview runs on another host;
   - links to a page of this site that the build didn't write (the file, the folder's `index.html` or `<path>.html`, the three forms the asset server answers), whether the link is relative, starts with `/` or names `https://laminapdf.com`;
   - has a `<style>` element or a `style` attribute, which `style-src 'self'` would block;
   - lacks a `<title>`, a description, or a canonical URL on `https://laminapdf.com`, unless it is marked `noindex`, as the 404 page is;
   - runs a script that imports a module from another origin, sets a cookie or sends a beacon.

   It checks the scripts and style sheets the build wrote in the same way, and fails if `_headers` sets a cookie. Astro bundles a `<script src="https://…">` into a module that imports the address, so the check of the build's scripts catches that one.
2. **The CSP's inline-script hashes** (`inlineScriptHashes`). The step replaces `'inline-script-hashes'` in `_headers` with the SHA-256 of each inline script the build wrote. When the build wrote none, as now, `script-src` is just `'self'`.
3. **`noindex` outside production** (`previewNoindex`). Every build adds `X-Robots-Tag: noindex` to `_headers` unless `LAMINA_PDF_ENV` is `production`. Only the production deploy sets the variable, so local builds and PR previews are never indexed.

## Headers

`public/_headers` applies [ADR 0011 §4](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#4-security-headers-appswebpublic_headers-for-)'s headers to every response, with the two differences the ADR gives the website:

- **`script-src 'self'` plus the inline scripts' hashes**, with no `'wasm-unsafe-eval'`, because the website runs no WebAssembly.
- **`connect-src 'self'`**, because the website fetches nothing remote.

`style-src` stays `'self'` and `font-src` stays `'self'`, so nothing inline loads. The build sets `build.inlineStylesheets: "never"`, which makes every style sheet a file, and `assetsInlineLimit: 0`, which keeps Vite from inlining fonts and images as `data:` URLs; pages set no `style` attributes.

`test/headers.test.ts` compares the `/*` block with the ADR's text. A change to the headers is an amendment to the ADR, so the test fails until both say the same. The `/_astro/*` block caches Astro's hashed files for a year.

## The home page

The home page ([screen H12](../../docs/design/paper/account-cloud-product.md#h12-website-home)) is one document sheet on the paper ground, the design the owner chose. Nothing on it is typed by hand twice:

- **Numbers and lists** come from `src/data/progress.json`, which `cargo xtask progress` writes from the todo board, the roadmap, `git log` and the open pull requests. Each production deploy runs the command before it builds, so the live site follows `main`. The committed file is what previews and local builds show, and refreshing it is an exempt pull request ([git workflow §2](../../docs/guidelines/git-workflow.md#pick-work-from-the-todo-board)). `src/data/progress.ts` checks the file at build time.
- **Code and packages** come from the files CI tests (`src/data/sources.ts`): the first PDF from `packages/sdk/examples/hello.ts` and `crates/laminapdf/examples/hello.rs`, the command-line examples that succeed from the CLI's README, and the install lines, the note on alpha releases and the package table from the docs site's install page.
- **Words** are in `src/data/copy.ts`, which the owner approved; a change to a claim goes back to the owner; the areas' plain names are in `src/data/rounds.ts`.

`test/home.test.ts` renders the page from a fixture snapshot with Astro's container API and finds each count, title and the date; `test/sources.test.ts` holds the code panel and the package table to their files. The code panel's tabs come from `src/scripts/tabs.ts`, a file because the CSP allows no inline script; without it, each pane shows under its own heading.

Every page carries Open Graph and X card tags with `public/og.png`. `robots.txt` lets search engines in only on the production build, and `sitemap.xml` lists every page but the 404 page.

## Look and header

The pages use the tokens of `@laminapdf/tokens` and the self-hosted fonts of `@laminapdf/fonts` (Lamina Sans and Newsreader), with the layout's style sheet in `src/styles/site.css`. The theme follows the visitor's system, so the site needs no script. `src/layouts/Base.astro` writes each page's head and preloads Lamina Sans' Latin file, the face of the first paint.

The header is the one both sites share, from [`@laminapdf/site-kit`](../../packages/site-kit/README.md#the-site-header): design language §7.39's wordmark, links and "Open the web app". A link to a page of this site appears once the page exists, on both sites. `astro.config.mjs` lists the files in `src/pages/` once per build, so the todo that writes `/privacy/` adds its link by adding the page. "Open the web app" appears once `APP_LAUNCHED` in the package's `src/nav.mjs` is `true`, which [C7c](../../docs/todos/ready/C7c-performance-gates-and-launch.md) sets when the reader goes live.

## Deploying

`wrangler.toml` is the Worker `lamina-pdf-site`: static assets only, no script, served at `laminapdf.com` (a Workers custom domain on the apex). Production has no workers.dev address, so search engines find one copy of the site, but `preview_urls` stays on for the per-PR previews on workers.dev. Unknown paths get `404.html` with a 404 status.

`.github/workflows/site.yml` deploys it, following the docs site's workflow ([`apps/docs/README.md`](../docs/README.md#deploying)):

- **`preview-build`** (pull requests that change the site or its inputs): builds the PR's site without secrets and uploads `dist/` as an artifact. The build marks it `noindex`.
- **`preview-deploy`** (pull requests): holds the secrets and never runs PR code. It reads `wrangler.toml` and the Wrangler version from the default branch and uploads the artifact as a preview version under the alias `pr-<n>`. Then it posts one comment with the URL and updates it on each push.
- **`production`** (every push to `main`, and every six hours): regenerates `src/data/progress.json` with `cargo xtask progress`, builds with `LAMINA_PDF_ENV=production` and runs `wrangler deploy`. The schedule catches pull requests opened since the last merge, so the todos they claim show as in progress.

Pushes to `main` deploy whatever they change, because any merge can move a todo. The pull request trigger's `paths` must list everything the site builds from, or a pull request that changes one of those files gets no preview:

- `apps/site/**`;
- the packages it imports: `packages/site-kit/**`, `packages/tokens/**` and `packages/fonts/**`;
- the files `src/data/sources.ts` reads: `packages/sdk/examples/hello.ts`, `packages/sdk/package.json`, `crates/laminapdf/examples/hello.rs`, `crates/laminapdf-cli/README.md` and `apps/docs/src/content/docs/install.mdx`.

Until the workflow is on `main`, the owner can upload a preview by hand: build the site, then run this from `apps/site` with the deploy token:

```sh
CLOUDFLARE_API_TOKEN=… CLOUDFLARE_ACCOUNT_ID=… pnpm exec wrangler versions upload --preview-alias pr-<n>
```

### What the owner needs to do

1. **Commit `.github/workflows/site.yml`.** W1b's pull request carries the file in its description, because the GitHub access that opened the pull request can't push workflow files.
2. **Check the API token.** The docs site's token works if its **Zone → Workers Routes → Edit** permission covers the `laminapdf.com` zone, since the apex is a custom domain on the same zone. The secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` are shared with the docs workflow. Without them, the deploy steps skip with a notice.
3. **Run the first production deploy.** After this site is on `main`, run the **Website** workflow there once (Actions → Website → Run workflow). The first `wrangler deploy` creates the Worker, which previews need, and attaches `laminapdf.com`. It fails if the apex already has a DNS record: delete that record first, or attach the domain once in the dashboard (Worker → Settings → Domains & Routes).
4. **Check the first preview.** Cloudflare's docs say `preview_urls = true` turns preview URLs on with workers.dev off, but show no example of the pair. If the next PR gets no preview comment, turn on Version URLs on the same Domains & Routes page.
5. **Redirect `www` to the apex.** Add a proxied DNS record for `www`: an `AAAA` record to `100::`, a placeholder address, is enough, because the redirect rule answers before any server would. Then add a redirect rule (Rules → Redirect Rules → the "Redirect from WWW to root" template): requests whose hostname is `www.laminapdf.com` go to `https://laminapdf.com` with the same path and query string, status 301.
6. **Preload HSTS, once every subdomain answers over HTTPS only.** A small PR adds `preload` to `Strict-Transport-Security` in `public/_headers`, as a third difference in ADR 0011 §4 and in `test/headers.test.ts`. After it deploys, submit `laminapdf.com` at <https://hstspreload.org/>.
