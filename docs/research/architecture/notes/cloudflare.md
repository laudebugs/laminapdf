# Cloudflare for Lamina: web PDF app hosting, desktop distribution, and optional server-side processing (as of 2026-10-28)

All figures were taken from developers.cloudflare.com on 2026-10-28. Each page's own "last updated" date is given inline where it showed one. Limits change often, so recheck them before you commit to a design.

## 1. Hosting the SPA: Workers static assets vs Pages, routing, domains, previews, config, the `cf` CLI

### Takeaway
Use **Workers with static assets**, not Pages. The Pages docs now carry a banner saying "Start new projects with Workers." Pages has no deprecation date yet. For an SPA, set `assets.not_found_handling = "single-page-application"`, and use `run_worker_first` for the routes that need a Worker, such as `/api/*`. Static-asset requests are free and unlimited. The new `cf` CLI (beta since 2026-09-28) is the planned successor to Wrangler, but it is still in beta, so pin Wrangler for CI today.

### Cited Findings
- The Pages overview shows a banner: "Are you sure you want to use Pages? Workers supports most Pages use cases and offers a broader feature set. It is Cloudflare's primary platform for building applications. Start new projects with Workers." The page gives no sunset or deprecation date (page updated Aug 25, 2026). — [Cloudflare Pages docs](https://developers.cloudflare.com/pages/)
- The migration guide (updated Oct 12, 2026) says "Workers has a distinctly broader set of features." Pages Functions are already billed at Workers rates. The final migration step is deleting the Pages project. — [Migrate from Pages](https://developers.cloudflare.com/workers/static-assets/migration-guides/migrate-from-pages/)
- **Feature differences** (same guide):
  - **Preview URLs:** Workers needs `preview_urls: true`, a `previews` block, and preview builds enabled in Workers Builds. Previews are served at `<preview-name>.<your-domain>`. Pages lets you point any hostname at a branch.
  - **Custom domains:** Workers custom domains need the zone's nameservers on Cloudflare. Pages also allows a CNAME from outside Cloudflare.
  - **SPA detection:** Pages auto-detects SPA or 404 behavior. Workers needs `not_found_handling` set explicitly.
  - **`_headers` / `_redirects`:** supported in Workers static assets. — [Migrate from Pages](https://developers.cloudflare.com/workers/static-assets/migration-guides/migrate-from-pages/)
- **SPA config** (page updated Oct 12, 2026):
  - `assets.directory`
  - `not_found_handling: "single-page-application"`, which serves `/index.html` with a 200 for unmatched paths
  - `binding: "ASSETS"`
  - `run_worker_first: ["/api/*", "!/api/docs/*"]`, which needs Wrangler ≥4.20.0 or the Vite plugin ≥1.7.0
  - A pure SPA needs no `main` script at all. — [SPA routing](https://developers.cloudflare.com/workers/static-assets/routing/single-page-application/)
- With compat date ≥ 2025-04-01 (or the flag `assets_navigation_prefers_asset_serving`), browser navigations (`Sec-Fetch-Mode: navigate`) skip the Worker and get assets, which cuts billable invocations. One gotcha: navigating directly to `/api/x` returns HTML, so OAuth callbacks need a static callback page that forwards to `/api/...`. — [SPA routing](https://developers.cloudflare.com/workers/static-assets/routing/single-page-application/)
- **Static asset limits:**
  - 20,000 files per version (Free), 100,000 (Paid; needs Wrangler 4.34.0+)
  - 25 MiB per file on both plans (page updated Oct 12, 2026) — [Workers limits](https://developers.cloudflare.com/workers/platform/limits/)
- "Requests to static assets are free and unlimited." — [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/)
- **`cf` CLI** (beta 2026-09-28):
  - One CLI for the public Cloudflare API (2,900+ commands, mostly returning JSON) and for Workers (`cf init`, `cf dev`, `cf build`, `cf deploy`).
  - Uses a typed `cloudflare.config.ts`. `cf migrate` converts Wrangler config, and resource commands also work inside Wrangler projects.
  - `cf cli search` lets agents find commands.
  - Install with `npm i -g cf`, then `cf auth login`.
  - "Commands, configuration, and Build Output may change before the stable release." — [Changelog: Cloudflare CLI beta](https://developers.cloudflare.com/changelog/post/2026-09-28-cloudflare-cli-beta/)
- Secondary sources say that when the beta ends, Cloudflare will ship a final Wrangler major that points users to `cf`, and will maintain Wrangler for 18 months after that. They also say `cf` sends anonymous telemetry by default. — [Flavio Copes tutorial](https://flaviocopes.com/cloudflare-cf/); [omidsaffari.com guide](https://omidsaffari.com/it/blog/cloudflare-wrangler-cf-cli) (not verified against an official Cloudflare source)

### Inferences
- **Recommended setup:** one Worker named `lamina-pdf-web`. Its `assets.directory` is the Vite `dist/`, with SPA not-found handling. `run_worker_first: ["/api/*"]` turns on the optional server features. Previews per PR come from Workers Builds. The custom domain must be a zone on Cloudflare.
- Keep `wrangler.jsonc` for now. Try `cf migrate` in a branch, then switch after `cf` reaches GA. The 18-month Wrangler support window leaves plenty of runway.
- **PDF.js / WASM:** large WASM blobs or worker bundles served as *static assets* must each be ≤25 MiB.

### Gaps
- No official Pages end-of-life date was found. The only source for the 18-month Wrangler window is secondary.
- I did not verify whether Workers Builds preview URLs are created automatically for GitHub PRs, or whether they need extra setup beyond the `previews` block.

## 2. Storage and compute products: R2, D1, KV, Durable Objects, Queues, Workflows, Containers, Browser Run, Workers AI

### Takeaway
- **R2:** PDF blobs (egress is free)
- **D1:** users, document index, and share metadata
- **KV:** small, read-heavy config and flags only
- **SQLite-backed Durable Objects with WebSocket Hibernation:** one DO per document for live annotation sync
- **Queues and Workflows:** async, multi-step PDF jobs
- **Containers (GA 2026-04-13):** native tools such as qpdf, OCRmyPDF/Tesseract, LibreOffice, and Ghostscript
- **Browser Rendering (now "Browser Run"):** HTML→PDF
- **Workers AI:** `toMarkdown` and vision-model OCR, as an optional extra

### Cited Findings
**R2** (pricing updated Oct 26, 2026; limits updated Jun 8, 2026)
- **Standard storage:** $0.015/GB-month. Class A $4.50/M, Class B $0.36/M.
- **Infrequent Access storage:** $0.01/GB-month. Class A $9/M, Class B $0.90/M, plus $0.01/GB retrieval and a 30-day minimum.
- **Egress:** free.
- **Free tier:** 10 GB-month, 1M Class A, 10M Class B. — [R2 pricing](https://developers.cloudflare.com/r2/pricing/)
- **Object and upload sizes:** max object 5 TiB (4.995 TiB); single PUT up to 5 GiB; up to 10,000 multipart parts.
- **Write rate:** 1 write per second to the same key (429 above that).
- **Custom domains:** 100 per bucket.
- The page is internally inconsistent: one footnote says "4.995 GiB." — [R2 limits](https://developers.cloudflare.com/r2/platform/limits/)
- **Presigned URLs:**
  - GET, HEAD, PUT, and DELETE only. POST form uploads are not supported.
  - Expiry from 1 s up to 7 days.
  - Work **only on the S3 endpoint** `<ACCOUNT_ID>.r2.cloudflarestorage.com`, not on custom domains.
  - Browser use needs CORS rules on the bucket. — [R2 presigned URLs](https://developers.cloudflare.com/r2/api/s3/presigned-urls/)

**D1** (limits updated Oct 24, 2026)
- **Workers Paid:** 10 GB max per DB, 50,000 DBs, 1 TB per account, 1,000 queries per invocation, 30-day Time Travel.
- **Free:** 500 MB per DB, 10 DBs, 5 GB total, 50 queries per invocation, 7-day Time Travel.
- **All plans:** 2 MB max row or BLOB, 100 KB max SQL statement. — [D1 limits](https://developers.cloudflare.com/d1/platform/limits/)
- **Pricing on Paid:** 25B rows read included, then $0.001/M. 50M rows written included, then $1/M. 5 GB storage included, then $0.75/GB-month. — [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/)

**KV**
- **Paid:** 10M reads, then $0.50/M. 1M writes, then $5/M. 1 GB storage, then $0.50/GB-month.
- **Free:** 100k reads and 1k writes per day. — [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/)

**Durable Objects**
- **Plans:** available on both Free and Paid. Free supports **only SQLite-backed** DOs. — [DO pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/) (updated Oct 18, 2026)
- **Requests:** 1M per month included on Paid, then $0.15/M. Free gets 100k per day.
- **Duration:** 400k GB-s included on Paid, then $12.50 per M GB-s, billed on the 128 MB allocated per object.
- **WebSocket billing:** incoming messages are billed at **20:1**. Outgoing messages are free.
- **SQLite storage:** included amounts mirror D1, but stored data costs $0.20/GB-month. SQLite storage billing began in January 2026. — [DO pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/)
- **Hibernation:** objects idle under the Hibernation API accrue no duration. Plain `accept()` bills for the whole connection. Cloudflare's example: about $412.50 always-on vs about $12.50 with hibernation for 100 objects. — [DO pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/)
- **Limits:**
  - 10 GB SQLite per object on Paid
  - Unlimited objects
  - CPU per request: 30 s by default, up to 5 min via `limits.cpu_ms` (it resets per request or WebSocket message)
  - 32 MiB max received WebSocket message
  - About 1,000 req/s soft limit per object
  - The FAQ says 1 GB per object on Free, which conflicts with the main table. — [DO limits](https://developers.cloudflare.com/durable-objects/platform/limits/) (updated Jun 1, 2026)

**Queues**
- **Paid:** 1M ops per month, then $0.40/M. Retention is 4 days by default, configurable up to 14.
- **Free:** 10k ops per day, 24 h retention. — [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/)
- **Consumer duration:** 15 min max. — [Workers limits](https://developers.cloudflare.com/workers/platform/limits/)

**Workflows** (limits updated Sep 21, 2026)
- **Steps per Workflow:** Paid 10,000 by default, configurable up to 25,000. Free 1,024.
- **CPU per step:** Paid 30 s by default, up to 5 min. Free 10 ms. Wall-clock time per step is unlimited.
- **Step result size:** 1 MiB for non-stream results. JavaScript steps can return a `ReadableStream`, but Cloudflare suggests R2 for large artifacts.
- **Persisted state per instance:** 1 GB on Paid.
- **`step.sleep`:** up to 365 days.
- **Concurrency:** 50,000 running instances on Paid. Waiting instances don't count toward the limit.
- **Completed-instance retention:** 30 days on Paid, can be shortened. — [Workflows limits](https://developers.cloudflare.com/workflows/reference/limits/)

**Containers**
- **Status:** Containers and Sandboxes went **GA on 2026-04-13**, bringing higher concurrency, active-CPU pricing, Docker Hub images, and SSH. — [Changelog: Containers & Sandboxes GA](https://developers.cloudflare.com/changelog/post/2026-04-13-containers-sandbox-ga/)
- **Instance types** (limits updated Oct 21, 2026):

  | Type | vCPU | Memory | Disk |
  |---|---|---|---|
  | lite | 1/16 | 256 MiB | 2 GB |
  | basic | 1/4 | 1 GiB | 4 GB |
  | standard-1 | 1/2 | 4 GiB | 8 GB |
  | standard-2 | 1 | 6 GiB | 12 GB |
  | standard-3 | 2 | 8 GiB | 16 GB |
  | standard-4 | 4 | 12 GiB | 20 GB |

- **Custom types:** up to 4 vCPU and 12 GiB, with at least 3 GiB per vCPU.
- **Account defaults:** 1,500 vCPU and 6 TiB memory concurrent. Image size is capped at the instance disk size, with 50 GB of total image storage. — [Containers limits](https://developers.cloudflare.com/containers/platform-details/limits/)
- **Pricing:**
  - Billed per 10 ms while running. Memory and disk are billed on provisioned size; CPU only on active use.
  - **Workers Paid includes:** 25 GiB-h memory, 375 vCPU-min, and 200 GB-h disk per month.
  - **Overage:** $0.0000025 per GiB-s memory, $0.000020 per vCPU-s, $0.00000007 per GB-s disk.
  - **Egress:** $0.025/GB in NA/EU with 1 TB included.
  - Charges stop once the container sleeps. Each container is backed by a billed Durable Object. Not available on the Free plan. — [Containers pricing](https://developers.cloudflare.com/containers/pricing/)

**Browser Rendering, now "Browser Run"**
- The docs now say "Browser Run, formerly known as Browser Rendering" (no rename date given).
- **Quick Actions (REST):** Markdown, Screenshots, **PDFs**, Snapshots, Links, scrape, JSON, crawl.
- **Browser Sessions:** Puppeteer, Playwright (Cloudflare fork), CDP, and Stagehand. — [Browser Run overview](https://developers.cloudflare.com/browser-rendering/) (updated Aug 11, 2026)
- **Pricing:**
  - **Free:** 10 browser-minutes per day and 3 concurrent browsers.
  - **Paid:** 10 browser-hours per month included, then $0.09/h. 10 concurrent browsers averaged monthly, then $2 per extra browser. — [Browser Run pricing](https://developers.cloudflare.com/browser-rendering/pricing/) (updated Apr 21, 2026)

**Workers AI**
- `toMarkdown` converts "documents in multiple formats" and is free for most conversions. Image conversion uses billed models. The page names no OCR or layout model. — [Markdown conversion](https://developers.cloudflare.com/workers-ai/features/markdown-conversion/) (updated Apr 21, 2026)
- The Workers AI changelog lists vision capabilities covering document and PDF parsing, OCR (including multilingual), and handwriting recognition. No model was named in the snippet I saw. — [Workers AI changelog](https://developers.cloudflare.com/changelog/product/workers-ai/) (search snippet; not fully read)

### Inferences
- **Collaboration:** one SQLite-backed DO per document. Store annotation ops or a CRDT log (e.g., Yjs updates) in DO SQLite. Use WebSocket Hibernation so idle documents cost almost nothing. At 20:1, 1M incoming sync messages bill as only 50k requests.
- **Uploads:** browser → Worker mints a presigned PUT on the S3 endpoint (or uses multipart for files over 5 GiB, which is rare for PDFs) → R2. The Worker then writes a D1 row. Downloads can stream through a Worker or a custom domain on a public bucket. Presigned URLs can't use the custom domain.
- **Jobs:** a Workflow orchestrates each job (fetch from R2 → container step → write result to R2 → update D1/DO). Containers on `basic` or `standard-1` should be enough for qpdf and Ghostscript. OCR (Tesseract/OCRmyPDF) and LibreOffice conversion probably need `standard-1` to `standard-2` for 4–6 GiB RAM (inference, not benchmarked).

### Gaps
- I could not confirm the Browser Run `/pdf` endpoint's request timeout or rate limits, because the Limits page was not fetched.
- I found no named Workers AI OCR or layout model, and no confirmation that `toMarkdown` OCRs scanned PDFs.
- Container cold-start times are not documented on the pages I read.
- R2 multipart minimum part size was not stated on the limits page. My recollection is 5 MiB, the S3-compatible value, but I did not verify it.

## 3. Limits for server-side PDF work in a Worker vs Containers

### Takeaway
A Worker has **128 MB of memory**, a **64 MiB uncompressed script limit** (including WASM, on both Free and Paid, with no separate compressed limit per the current page), and **CPU of 10 ms on Free or 30 s by default up to 5 min on Paid**. That fits WASM-based PDF operations such as pdf-lib or Rust/WASM merge, split, rotate, metadata, form flattening, and small-file text extraction. It does not fit large files, OCR, rasterizing many pages, or LibreOffice. Those belong in Containers.

### Cited Findings
- **Script size:** 64 MiB uncompressed on both Free and Paid, with no compressed limit.
- **Startup:** 1 s.
- **Memory:** 128 MB per isolate.
- **CPU per HTTP request:** Free 10 ms. Paid 30 s by default, configurable up to 5 min (300,000 ms).
- **Cron CPU:** up to 15 min for intervals of 1 h or more.
- **Request body:** Free and Pro 100 MB, Business 200 MB, Enterprise up to 5 GB self-serve.
- **Subrequests:** Free 50 (1,000 to internal services). Paid 10,000 by default, up to 10M.
- **Duration:** no limit for HTTP requests. 15 min for Cron, DO alarms, and Queue consumers.
- **Workers per account:** 100 Free, 500 Paid. — [Workers limits](https://developers.cloudflare.com/workers/platform/limits/) (updated Oct 12, 2026)
- **Paid plan cost:** $5 per month minimum. Includes 10M requests (then $0.30/M) and 30M CPU-ms (then $0.02 per M CPU-ms). No egress charges. — [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/) (updated Oct 2, 2026)

### Inferences
- **Script-size change:** the 64 MiB uncompressed, no-compressed-limit figure is a change from earlier years, when the limits were 3 MB (Free) and 10 MB (Paid) compressed. My recollection of the old values is not re-verified. Either way, a WASM PDF engine (for example PDFium-WASM at roughly 5–10 MB, or MuPDF-WASM) now fits on both plans, subject to the 1 s startup limit.
- **Memory:** the 128 MB ceiling is the real constraint. PDFs over about 30–50 MB, or rendering many pages, risk OOM in a Worker, so route those to a Container. Bodies over 100 MB can't be POSTed to a Worker on Free or Pro, so upload straight to R2 with presigned URLs.
- **CPU:** on Free, 10 ms rules out any server-side PDF processing. Server features effectively need Workers Paid ($5 per month).

### Gaps
- Behavior when memory is exceeded, and whether WASM memory counts toward 128 MB, were not confirmed on the fetched page. Cloudflare has historically counted it.

## 4. Privacy-first architecture, auth, desktop distribution, cost, observability

### Takeaway
- **Privacy:** process PDFs client-side in WASM by default. Send data to R2 or Containers only on explicit opt-in. Set short retention with R2 lifecycle rules, and keep Workflows/Queues retention minimal.
- **Desktop distribution:** a Worker in front of an R2 bucket serves the Tauri dynamic-update JSON (return 204 when there's no update), per release channel, and logs download analytics.
- **Cost:** a small deployment runs for about $5 per month on Workers Paid, with egress free on R2 and Workers.

### Cited Findings
**Tauri updater format**
- **Static manifest:**
  - `version` (SemVer), optional `notes`, and optional `pub_date` (RFC 3339).
  - `platforms` keyed `OS-ARCH` (`linux|darwin|windows` - `x86_64|aarch64|i686|armv7`). Each entry needs a `url` and a `signature`, where `signature` is the *contents* of the `.sig` file.
  - The whole file is validated, so every platform entry must be complete. — [Tauri v2 updater plugin](https://v2.tauri.app/plugin/updater/)
- **Dynamic server:** return `204 No Content` when there's no update. Otherwise return `200` with `{version, url, signature, pub_date?, notes?}`.
- **Endpoint variables:** `{{current_version}}`, `{{target}}`, `{{arch}}`. Custom variables aren't supported, but you can supply a custom `{{target}}`.
- **Failover:** Tauri tries the next endpoint only on a non-2XX response.
- **Transport:** TLS is enforced in production. — [Tauri v2 updater plugin](https://v2.tauri.app/plugin/updater/)
- **Signing** can't be disabled. `pubkey` goes in `tauri.conf.json`. CI needs `TAURI_SIGNING_PRIVATE_KEY` (and the optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`); `.env` files are not supported. Losing the key breaks updates for every existing install.
- **Updater artifacts** with `createUpdaterArtifacts: true`:
  - macOS: `.app.tar.gz` plus `.sig`
  - Linux: AppImage plus `.sig`
  - Windows: NSIS `setup.exe` and MSI, each with `.sig`
  - `"v1Compatible"` will be removed in v3. — [Tauri v2 updater plugin](https://v2.tauri.app/plugin/updater/)
- R2 egress is free, so installer downloads cost only Class B operations ($0.36/M after 10M free per month). — [R2 pricing](https://developers.cloudflare.com/r2/pricing/)
- The 25 MiB per-file static-asset cap means installers must live in **R2**, not in Workers static assets. — [Workers limits](https://developers.cloudflare.com/workers/platform/limits/)

**Observability**
- **Workers Logs:** Free gets 200k events per day with 3-day retention. Paid gets 20M events per month, then $0.60/M, with 7-day retention.
- From **December 1, 2026**, Workers Logs moves to "Cloudflare Observability pricing." — [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/)
- Container logs are billed at the Workers Logs rate. — [Containers pricing](https://developers.cloudflare.com/containers/pricing/)

**Data retention controls**
- D1 Time Travel keeps 30 days on Paid and 7 on Free. — [D1 limits](https://developers.cloudflare.com/d1/platform/limits/)
- Workflows keeps completed instances for 30 days on Paid, and this can be shortened. — [Workflows limits](https://developers.cloudflare.com/workflows/reference/limits/)
- Queues keeps messages 4 days by default on Paid, configurable up to 14. — [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/)
- Container snapshots are kept for 30 days. — [Containers limits](https://developers.cloudflare.com/containers/platform-details/limits/)

### Inferences
- **Update endpoint:** `GET /update/{channel}/{{target}}/{{arch}}/{{current_version}}` on a Worker. The Worker reads `channels/{channel}/latest.json` from R2 (or KV/D1) and returns 204 or 200 with the matching platform's `url` (an R2 custom-domain download URL) and `signature`. Cache the manifest response briefly (around 60 s) with the Cache API.
- **Download analytics:** log each download to Workers Analytics Engine or D1, or simply rely on Workers Logs. Analytics Engine pricing was not verified here.
- **Release channels:** stable, beta, and nightly can be separate R2 prefixes. Promotion is just copying the manifest.
- **Privacy:** the default build has no `/api` routes and uploads nothing. When the user opts in, objects get an R2 lifecycle expiry (for example 24 h for processing temp files). R2 encrypts at rest by default, which I believe but did not re-verify in this session. For a strong guarantee, use client-side encryption before upload (end-to-end encryption) so the server only stores ciphertext. That would rule out server-side processing of those files.
- **Auth for an open-source app:** Cloudflare Access fits admin or staging only. It is a Zero Trust product with per-seat plans, so it doesn't suit end users. For end users, self-hostable libraries such as Better Auth or Auth.js run on Workers with D1 and support passkeys/WebAuthn, and forks can deploy them without a vendor account. Hosted options (Clerk, WorkOS) are simpler but add a vendor dependency and cost per monthly active user. **None of these auth claims were verified in this session.**
- **Rough cost at small to medium scale** (inference from the cited prices):
  - Workers Paid at $5 per month covers 10M requests, 30M CPU-ms, D1/DO allowances, 1M Queue ops, and Container allowances.
  - R2 at 100 GB is about $1.35 per month after the free 10 GB.
  - Downloads add nothing for egress.
  - The variable costs are Container CPU and memory, Browser Run hours ($0.09/h), and DO duration if you skip hibernation.

### Gaps
- The Cloudflare One/Access plan page returned 404, so free-tier seat counts and pricing are unverified.
- Better Auth, Auth.js, Clerk, and WorkOS compatibility with Workers/D1 and passkey support were not researched in this session.
- The new Cloudflare Observability pricing that applies from Dec 1, 2026 was not fetched.
- Tail Workers and Logpush specifics were not fetched.
- R2 encryption-at-rest wording and lifecycle rule limits were not re-verified.
- Workers Analytics Engine pricing and limits were not checked.
