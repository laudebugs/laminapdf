# `apps/site`: the website at laminapdf.com

Lamina PDF's website ([todo W1b](../../docs/todos/ongoing/W1b-site-scaffold-and-deploys.md), [website plan §4](../../docs/plans/website.md#4-the-website)) is a static [Astro](https://astro.build/) site served from Cloudflare Workers static assets at **https://laminapdf.com**. It holds pages of text in the [Paper design language](../../docs/design/paper/design-language.md), and every build checks each page against the website's rules in [ADR 0011 §3](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#3-the-v01-privacy-guarantees-testable). The home page is a placeholder until [W1c](../../docs/todos/ready/W1c-home-page.md) writes it.

```sh
pnpm install                          # once per worktree
pnpm --filter @laminapdf/site dev        # http://localhost:4321 (no headers in dev)
pnpm --filter @laminapdf/site build      # dist/: what deploys; fails on a page that breaks the rules
pnpm --filter @laminapdf/site test       # the headers against ADR 0011 §4, the header's links
pnpm --filter @laminapdf/site typecheck
```

`cargo xtask check ts` runs the build, the tests and the typecheck, so CI fails on a page that breaks the rules.

## What every build checks

After the build, three steps from `@laminapdf/site-kit` (`astro.config.mjs`) run on `dist/`, in this order:

1. **The website's rules** (`siteChecks`). The build fails, naming each page and problem, when a page:
   - loads a file from another origin, or one the build didn't write;
   - links to a page of this site that the build didn't write;
   - has a `<style>` element or a `style` attribute, which `style-src 'self'` would block;
   - lacks a `<title>`, a description, or a canonical URL on `https://laminapdf.com`;
   - runs a script that imports a module from another origin, sets a cookie or sends a beacon.

   It checks the scripts and style sheets the build wrote in the same way, and fails if `_headers` sets a cookie. Astro bundles a `<script src="https://…">` into a module that imports the address, so the check of the build's scripts catches that one.
2. **The CSP's inline-script hashes** (`inlineScriptHashes`). The step replaces `'inline-script-hashes'` in `_headers` with the SHA-256 of each inline script the build wrote. When the build wrote none, as now, `script-src` is just `'self'`.
3. **`noindex` outside production** (`previewNoindex`). Every build adds `X-Robots-Tag: noindex` to `_headers` unless `LAMINA_PDF_ENV` is `production`. Local builds and PR previews are therefore never indexed, and only the production deploy sets the variable.

Fonts and images are files too: `assetsInlineLimit: 0` keeps Vite from inlining them as `data:` URLs, which `font-src 'self'` refuses.

## Headers

`public/_headers` applies [ADR 0011 §4](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#4-security-headers-appswebpublic_headers-for-)'s headers to every response, with the two differences the ADR gives the website:

- **`script-src 'self'` plus the inline scripts' hashes**, with no `'wasm-unsafe-eval'`, because the website runs no WebAssembly.
- **`connect-src 'self'`**, because the website fetches nothing remote.

`style-src` stays `'self'`. The build sets `build.inlineStylesheets: "never"`, so every style sheet is a file, and pages set no `style` attributes. `test/headers.test.ts` compares the `/*` block with the ADR's text. A change to the headers is an amendment to the ADR, so the test fails until both say the same. Astro's hashed files in `/_astro/` are cached for a year.

## Look and header

The pages use the tokens of `@laminapdf/tokens` and the self-hosted fonts of `@laminapdf/fonts` (Lamina Sans and Newsreader), with the layout's style sheet in `src/styles/site.css`. The theme follows the visitor's system, so the site needs no script. `src/layouts/Base.astro` writes each page's head and preloads Lamina Sans' Latin file, the face of the first paint.

The header is design language §7.39's: the wordmark, the links Download · Docs · Pricing · Performance · Privacy, and "Open the web app". A link to a page of this site appears once the page exists. `src/nav.ts` checks the files in `src/pages/`, so the todo that writes `/privacy/` adds its link by adding the page. "Open the web app" appears once `APP_LAUNCHED` in `src/nav.ts` is `true`, which [C7c](../../docs/todos/ready/C7c-performance-gates-and-launch.md) sets when the reader goes live. [W1d](../../docs/todos/ready/W1d-one-site-header.md) gives the docs site the same header.

## Deploying

`wrangler.toml` is the Worker `lamina-pdf-site`: static assets only, no script, served at `laminapdf.com` (a Workers custom domain on the apex), with `preview_urls` on for the per-PR previews on workers.dev. Unknown paths get `404.html` with a 404 status.

`.github/workflows/site.yml` deploys it, following the docs site's workflow ([`apps/docs/README.md`](../docs/README.md#deploying)):

- **`preview-build`** (pull requests that change the site or its inputs): builds the PR's site without secrets and uploads `dist/` as an artifact. The build marks it `noindex`.
- **`preview-deploy`** (pull requests): holds the secrets and never runs PR code. It reads `wrangler.toml` and the Wrangler version from the default branch and uploads the artifact as a preview version under the alias `pr-<n>`. Then it posts one comment with the URL and updates it on each push.
- **`production`** (pushes to `main`): builds with `LAMINA_PDF_ENV=production` and runs `wrangler deploy`.

Until the workflow is on `main`, a preview can be uploaded by hand with the deploy token, from `apps/site` after a build:

```sh
CLOUDFLARE_API_TOKEN=… CLOUDFLARE_ACCOUNT_ID=… pnpm exec wrangler versions upload --preview-alias pr-<n>
```

### What the owner needs to do

1. **Commit `.github/workflows/site.yml`.** W1b's pull request gives the file, because the session that wrote it can't push workflow files.
2. **Check the API token.** The docs site's token works if its **Zone → Workers Routes → Edit** permission covers the `laminapdf.com` zone, since the apex is a custom domain on the same zone. The secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` are shared with the docs workflow. Without them, the deploy steps skip with a notice.
3. **Run the first production deploy.** After this site is on `main`, run the **Website** workflow there once (Actions → Website → Run workflow). The first `wrangler deploy` creates the Worker, which previews need, and attaches `laminapdf.com`. It fails if the apex already has a DNS record: delete that record first, or attach the domain once in the dashboard (Worker → Settings → Domains & Routes).
4. **Redirect `www` to the apex.** Add a proxied DNS record for `www` (an `AAAA` record to `100::` is enough, since nothing answers there). Then add a redirect rule (Rules → Redirect Rules → the "Redirect from WWW to root" template): requests whose hostname is `www.laminapdf.com` go to `https://laminapdf.com` with the same path and query string, status 301.
5. **Preload HSTS, once every subdomain answers over HTTPS only.** A small PR adds `preload` to `Strict-Transport-Security` in `public/_headers`, as a third difference in ADR 0011 §4 and in `test/headers.test.ts`. After it deploys, submit `laminapdf.com` at <https://hstspreload.org/>.
