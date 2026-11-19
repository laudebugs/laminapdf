# Commands in detail

Every `cargo xtask` task and the other commands, with their flags. The root `CLAUDE.md` lists the everyday ones with shorter notes; this file is the reference.

The tools each task needs, and how to run a single crate's tests, are in the root [`CLAUDE.md`](../../CLAUDE.md#commands).

```sh
pnpm install                 # once per worktree: Biome, TypeScript (needed by the TS checks)
cargo xtask check            # everything a PR must pass: `check docs` + `check rust` + `check ts`
cargo xtask check docs       # docs-links + docs-prose + docs-snippets + todos --check + portable paths + design screens (every screen
                             #   in docs/design/paper/screens.md has its area heading and design file, canvas.json matches the files;
                             #   "Not yet drawn" IDs reported) + capability matrix static checks
                             #   + renames (docs/renames.json against laminapdf.renames/1, an entry per #[deprecated]/@deprecated item,
                             #   every new name in the sources) + package metadata (every published package, the CLI's npm wrapper
                             #   included, has a description; no ADR number, todo ID or "is private" in it or its README's
                             #   first paragraph) + sdk doc comments (no ADR number or todo ID in a /** */ comment of
                             #   packages/sdk/src, generated files included: TypeDoc's API reference) + gen codes --check + gen boundary --check + version sync --check
                             #   (fast; no network, no build, no node_modules)
cargo xtask check docs-prose [--all-strict] [--warnings] [--json]  # reader-facing text (docs site, docs/errors, package
                             #   READMEs) vs docs/guidelines/docs-writing.md (word lists: tools/xtask/docs-prose.toml); errors on
                             #   files changed since the merge base with main (drafts included), counted warnings elsewhere; `check docs`
                             #   runs it with --all-strict
cargo xtask check docs-snippets [--all-strict] [--warnings] [--json]  # the examples and Snippet's glob vs
                             #   docs/guidelines/examples.md (headers, shown on a page or in examples/UNUSED.md, TS/Rust pairs in
                             #   tools/xtask/docs-snippets.toml, public imports, no test scaffolding outside its
                             #   [[test-example]] list, ≤ 100 lines); strict on changed examples, warnings elsewhere (`check docs` runs it with --all-strict); skips examples/differential/ (a tool)
cargo xtask biome-examples [--all-strict]  # Biome with --error-on-warnings on the changed TS examples (every one with --all-strict,
                             #   as `check ts` runs it)
cargo xtask clippy-examples [--all-strict] [--warnings] [--json]  # clippy on Lamina PDF's examples with
                             #   tools/xtask/clippy-examples/clippy.toml (functions ≤ 40 lines) and let_underscore_must_use;
                             #   strict on changed examples, warnings elsewhere (`check rust` runs it with --all-strict)
cargo xtask check rust       # fmt --check, maintainability, portability, clippy native + wasm32 (-D warnings), clippy-examples,
                             #   tests, rustdoc, check since, cargo-deny
cargo xtask check maintainability  # docs/guidelines/maintainability.md's Rust rules, no build: ≤ 800 non-test lines per module
                             #   and no new size-lint #[expect] (tools/xtask/maintainability-baseline.toml), no pub type name in
                             #   two crates and no façade name re-exported twice (maintainability-duplicates.toml); both only shrink;
                             #   no `matches!` or `Err(…)` or-pattern on LimitExceeded or DataPending outside laminapdf_cos::error;
                             #   no copied test helper (`fn fixtures_dir`, `fn classic`, `struct Rng`) or cross-crate `#[path]` outside
                             #   laminapdf-test-fixtures (baseline `test-helpers`); no PDF lexer or PNG row filter outside
                             #   laminapdf-syntax (`one-lexer`); no todo ID with a letter suffix (read from docs/todos/'s file
                             #   names) in crates/*/src or packages/*/src, TypeScript included (`todo-id`)
cargo xtask check since      # every public item of a published crate ends its docs with `Added in X.Y.Z.` (rustdoc's JSON
                             #   with RUSTC_BOOTSTRAP=1, into target/since/; tools/xtask/src/since.rs); part of `check rust` (in CI on the Linux x64 leg only). @laminapdf/sdk's
                             #   exports carry `@since X.Y.Z`, which `check ts`'s apps/docs/scripts/check-api-docs.mjs checks
cargo xtask check ts         # the platform boundary (ADR 0010 rule 1: no `@tauri-apps/*`, `isTauri`, Tauri globals or user agent in
                             #   packages/*/src, comments aside), biome ci (packages/sdk/biome.json's examples override reports errors; every example is linted
                             #   again with --error-on-warnings), typecheck and tests of packages/* and apps/*, the docs site's build
                             #   (apps/docs: TypeDoc, snippets, code table; fails on a broken internal link), the website's build
                             #   (apps/site: fails on a page that breaks the website's rules), npm licences
cargo xtask fmt              # format Rust and TS/JSON (fmt --check only verifies)
cargo xtask lint             # clippy (native all-targets + wasm32 core crates) and biome ci
cargo xtask test             # cargo nextest (or cargo test) + doc tests, then TS tests
cargo xtask portability      # core crates: no fs/net/process/thread/clocks, no edge deps, forbid(unsafe),
                             #   cargo check for wasm32-unknown-unknown, aarch64-apple-ios, aarch64-linux-android
cargo xtask wasm             # build the wasm32 crates with the `wasm-release` profile, then @laminapdf/wasm's three modules
                             #   (linked with a 2 GiB maximum memory, wasm-bindgen with --experimental-reset-state-function,
                             #   wasm-opt, binaryen fetched into target/tools/) into packages/wasm/dist/, each checked for the
                             #   maximum; the base module with a 64 MiB maximum into target/wasm/small-memory/;
                             #   their Node tests (wasm-bindgen-test, test/smoke.test.ts), then @laminapdf/sdk over them
                             #   (tsc, typecheck, Vitest in Node (`node` and `small-memory` projects) and Vite, publint, attw --pack)
cargo xtask size             # raw/gzip/brotli of each module, min+gzip glue and @laminapdf/sdk's JS; fails above packages/wasm/size-budgets.json
                             #   (ADR 0016 §6), writes target/size-report.json, twiggy diff on > 5 % growth
cargo xtask pdf-runtimes [node|chromium|firefox|webkit|deno|bun|workerd]… [--suites <name,…>] [--label <name>]  # @laminapdf/sdk's example suite per runtime (default
                             #   all; after `wasm`): byte-identical to the goldens; hello, invoice, invoice-from-json and report timings and WebAssembly memory
                             #   vs pdf-library §4 in the job summary (recorded, not asserted); `node` is the one on PATH;
                             #   Deno, Bun, wrangler and Playwright are dev deps (`playwright install` runs first); a browser
                             #   also runs the packages' browser tests and the reader's (apps/web) typecheck, build and e2e suite;
                             #   --suites runs only those named (examples, a package's, web, docs, site; `cargo xtask changes`
                             #   lists a PR's), every one by default; a suite still running after 10 minutes (an e2e suite: 20)
                             #   is stopped with what it started and run once more, and a pass then is reported flaky;
                             #   --history <dir> --runner <label> --record appends them to history.jsonl (nightly);
                             #   --label <name> writes measurements-<name>.json instead of measurements.json
cargo xtask viewer-perf [chromium|firefox|webkit]… [--quick] [--history <dir> --runner <label> [--record] [--dry-run]]
                             # after `wasm`: @laminapdf/viewer's perf harness (packages/viewer/perf/): first page warm and
                             #   cold (Fast 4G, Chromium), 1,000 pages scrolled twice, memory, fling, zoom vs web-app §4,
                             #   in target/viewer-perf/ and the job summary (reported, never asserted); --history flags
                             #   figures >10% worse (bench's rule, `viewer/<browser>/<figure>` keys), --record appends,
                             #   --dry-run prints the issue; --quick: 100 pages, never recorded
cargo xtask viewer-perf --gate  # the reader's release gate (C7c): three full Chromium runs, each figure's median against
                             #   web-app §4 + 25 % (packages/viewer/perf/gate.ts), a miss fails unless perf/waivers.json waives it
cargo xtask web size         # after `web build`: apps/web/dist/index.html's startup scripts, gzipped, against apps/web/size-budget.json
                             #   (`pdf-runtimes` runs it after its reader build, so it gates PRs); a per-script table
cargo xtask web next-release [--commit <rev>] [--since <tag>] [--notes <file>]  # is <rev> a new web reader release? what ships
                             #   changed since <tag> (the last that deployed; default the newest web-v* tag): not docs, Markdown,
                             #   or tools/ and workflows other than the wasm build, web.rs and web-deploy.yml; prints and sets ready,
                             #   tag (web-v<next>: apps/web/package.json's version when newer, else the newest tag's patch + 1) and
                             #   previous; the notes are those commits' subjects
cargo xtask web check-tag <tag>  # fail unless <tag> is web-v<version> on apps/web/package.json's major and minor, below no other
                             #   web-v* tag (production-checks runs it)
cargo xtask gallery [--test|--update]  # the Paper gallery (apps/gallery/README.md): serve it (Vite); --test runs its Playwright
                             #   tests in the Playwright image pinned in apps/gallery/package.json (Docker: the gallery's files
                             #   streamed in, the host's node_modules untouched; results in target/gallery/results/); --update
                             #   rewrites apps/gallery/e2e/__screenshots__/ (the only way to change them) and lists what changed
cargo xtask gen codes [--check|--md]  # validate fixtures/codes.json against codes.schema.json, then write the error contract:
                             #   ErrorCode/WarningCode (crates/laminapdf-syntax/src/generated/codes.rs; the façade re-exports
                             #   them), the façade's explain pages (crates/laminapdf/src/generated/explain.rs), @laminapdf/sdk's
                             #   unions, KINDS/HINTS and explain text (packages/sdk/src/generated/), the laminapdf.error/1
                             #   schema (packages/sdk/schemas/, crates/laminapdf/schemas/), @laminapdf/render's code tables
                             #   (packages/render/src/generated/) and docs/errors/README.md's tables;
                             #   --check fails naming each stale file; --md prints the tables (for the docs site)
cargo xtask gen boundary [--check]  # the TypeScript side of the lists that cross the WASM boundary, read from the Rust source
                             #   (no build): @laminapdf/sdk's builder enums (packages/sdk/src/generated/builder-enums.ts, from
                             #   laminapdf-wasm's name tables in src/build_dto.rs and TagKind::as_str) and @laminapdf/render-hayro's
                             #   HAYRO_VERSION (the workspace's hayro pin), and the extract JSON's types (packages/sdk/src/generated/extract.ts,
                             #   from crates/laminapdf-extract/schemas/extract.schema.json), and the flow document's types (packages/sdk/src/generated/document.ts,
                             #   from crates/laminapdf-build/schemas/document.schema.json); --check fails naming each stale file
cargo xtask deny             # cargo-deny (licences, advisories, bans, sources) + npm licence check
cargo xtask npm-licenses     # npm licence check only
cargo xtask docs-links       # every relative link and #anchor in tracked *.md resolves (GitHub slugs); --json for tooling
cargo xtask docs-links --external [--cache <file>]  # nightly: every http(s) URL in tracked *.md answers (links, bare URLs, HTML
                             #   href/src; curl, one retry, per-host rate limit, allowlist .config/link-check.toml; 403/429 reported as
                             #   refused; a host given up after 5 refusals or 3 URLs with no answer or a server error; missing
                             #   pages/sites reported (missing.md, with reasons), never a failure; report in target/nightly/links/);
                             #   --cache: URLs that answered OK in the last 4 to 7 days report `cached`, not fetched (docs_links/cache.rs)
cargo xtask todos            # check the todo graph, point each Depends on link at its todo's current folder, print "Can start now"
                             #   and the ongoing todos, and write the whole board (with what each todo blocks) to
                             #   target/todos/board.md; the board and Blocks fields are never committed, since a derived list
                             #   would change in every PR that moves a todo and make each merge conflict with the other open PRs
cargo xtask todos --check    # change nothing; fail on an inconsistent graph, a Depends on link to an old folder, a Blocks row,
                             #   or a docs/todos/README.md that links to a todo file (a stale board brought back by a merge)
cargo xtask indexnow snapshot <sitemap> --out <file>  # W3e: before a production deploy, the live sitemap's pages and their
                             #   <lastmod> (a sitemap index followed; a URL fetched with curl, or a file)
cargo xtask indexnow submit --before <snapshot|sitemap> --after <sitemap> --key-file <file> [--dry-run]
                             # after it: the pages added or whose <lastmod> moved, posted to api.indexnow.org with the key
                             #   both sites serve as public/<key>.txt; every failure only warns (site.yml's and docs.yml's
                             #   production jobs only, tests in tools/xtask/src/workflows/indexnow.rs; fixtures/indexnow/)
cargo xtask progress [--offline]  # write apps/site/src/data/progress.json, the website's roadmap and dashboard, from main
                             #   (origin/main) whatever the checkout: the board's counts per section and "Can start now" (todos.rs's
                             #   parser), the roadmap's milestones and their state, merged PRs from `git log` (titles without
                             #   numbers) and the todos open PRs claim, which move from next up to in progress (gh api; --offline
                             #   or a gh failure keeps the last list); the website's production deploy runs it before every
                             #   build, so the committed file only feeds previews and local builds; refreshing it is an exempt PR
cargo xtask capabilities     # run the tests tagged `/// capability: <ID> <surface> <status>`, regenerate docs/capabilities/README.md and
                             #   matrix.json (the reader's capability page); count `[cap J.01 web round-trip]` titles in apps/web/e2e,
                             #   which the reader's e2e suite runs, and `[cap J.01 desktop round-trip]` titles in
                             #   apps/desktop/e2e/specs, which the full lane's desktop-e2e jobs run
cargo xtask capabilities --check  # full lane: fail if that page differs from what the passing tagged tests prove
cargo xtask golden --update  # regenerate every golden in fixtures/golden/ (and fixtures/reading-order/) from its source, print the list
                             #   (and fixtures/shaping/ with hb-shape when HarfBuzz is installed)
cargo xtask examples         # run the Lamina PDF examples into target/examples-out/, byte-compare with
                             #   fixtures/golden/examples/, qpdf --check each (needs qpdf)
cargo xtask render-golden [--update]  # every page of fixtures/golden/examples/ by PDFium (Python with
                             #   tools/render/requirements.txt) and hayro at 72 dpi vs fixtures/render-golden/,
                             #   and hayro vs PDFium at 150 dpi; then laminapdf-render's PDFium and hayro adapters on
                             #   fixtures/corpus/ at 96 dpi vs fixtures/golden/render/ (laminapdf-render-pdfium's ignored tests)
cargo xtask annots-parity [--update] [--corpus]  # every annotation subtype x colour x opacity x border on pages turned
                             #   0/90/180/270 (tools/render's annots-parity, through the facade) by PDFium, pdf.js
                             #   (tools/render/pdfjs-render.mjs, its own pnpm project) and hayro at 72 dpi vs
                             #   fixtures/render-golden/annots-parity/, pdf.js and hayro vs PDFium at 150 dpi (annots-parity.toml);
                             #   --corpus: every annotation of the pdf.js corpus's annotation-*.pdf moved 1 pt, saved
                             #   incrementally, reopened by Lamina PDF, qpdf --check, drawn by PDFium, pdf.js and hayro
                             #   as the original was; outcomes pinned in annots-parity-corpus.toml (nightly, macOS arm64)
cargo xtask verapdf [--dir <dir>]  # the Lamina PDF examples (or the PDFs in <dir>) through veraPDF for the profiles each one's XMP
                             #   claims (PDF/A-2b, 2u, 3b, PDF/UA-1); any violation fails; the pinned greenfield CLI (doctor.rs,
                             #   SHA-256 checked) installed into $RUNNER_TOOL_CACHE/lamina-verapdf or target/verapdf, run with
                             #   $LAMINA_PDF_JAVA, Homebrew's openjdk@21 or java (17+); an external process only (ADR 0019)
cargo xtask extract-bench [--subset <name>]… [--sample N] [--tables lattice-and-stream] [--update-lock]  # L3f: Lamina PDF's extraction
                             #   scored on olmOCR-bench (tools/bench/extract-datasets.toml: revision, ODC-BY-1.0, subsets, refused datasets;
                             #   SHA-256 per file in tools/bench/olmocr-bench.lock): files fetched into target/bench/datasets/, Markdown
                             #   from crates/laminapdf/examples/differential/extract-bench.rs, scored by the benchmark's own tests
                             #   (tools/bench/olmocr_score.py, Python with tools/bench/requirements.txt, $LAMINA_PDF_PYTHON); report
                             #   in target/bench/extract/<dataset>/ with <candidate>-results.json (scores, pages by outcome, summed and
                             #   median ms, pages a second on one thread, version, commit, date, machine); never in CI; the scores are on
                             #   the docs site's Benchmarks page (crates/laminapdf-extract/README.md explains the failures)
cargo xtask benchmarks [--no-run]  # T1c: apps/docs/src/data/benchmarks.json, the docs site's Benchmarks page (version 1): runs `size`
                             #   (after `wasm`), `pdf-runtimes` in every runtime and `bench` unless --no-run, then copies their reports
                             #   (generation: per runtime hello-first, hello, invoice, report, wasm-memory with the tool's version, and
                             #   criterion's invoice_one_page and report_100_pages; sizes: gzip/brotli per module and JS) and the newest
                             #   extract-bench results of `laminapdf` and `laminapdf-stream`; each section has its run stamp
                             #   (tools/xtask/src/machine.rs: OS, CPU, cores, memory, load average); a missing report keeps the committed
                             #   section; run on an idle machine, before a release (refreshing the file is an exempt PR)
cargo run -p laminapdf-cli -- <command> …  # the `laminapdf` CLI (ADR 0031): build, info, check, reveal, pages rotate, extract, render, explain, schema, completions, help exit-codes
cargo xtask cli-docs         # the CLI's man pages (clap_mangen, one per command) and shell completions into target/cli-docs/
cargo xtask cli-dist         # cli-docs plus THIRD_PARTY_NOTICES.md (cargo-about, about/ in laminapdf-cli, `mcp` on): what
                             #   dist-workspace.toml's `include` puts in every release archive; then `dist build --artifacts=local
                             #   --target <triple>` (dist pinned in doctor.rs) writes the archive into target/distrib/
pnpm --filter @laminapdf/desktop run desktop:dev  # the desktop reader with its engine and PDFium (after `pnpm --filter
                             #   "@laminapdf/desktop^..." run build`; Linux: libwebkit2gtk-4.1-dev libdbus-1-dev); `desktop:build
                             #   --debug --no-bundle` is what the full lane's `desktop` job builds on Linux, macOS and Windows when
                             #   `cargo xtask changes` says desktop=true, then checks the engine and PDFium sit beside the app,
                             #   the main process links no parser or PDFium, and the build has no e2e command, plugin or probe
pnpm --filter @laminapdf/desktop run desktop:build:e2e  # the e2e build: debug, feature `e2e`, tauri.e2e.conf.json; then
                             #   `run e2e` runs the eleven WebdriverIO scenarios and the cold start with a path (Linux: under
                             #   dbus-run-session and xvfb-run; needs
                             #   target/fixtures/1000-pages.pdf and 1gb.pdf from `cargo xtask fixtures large-pdf`) and
                             #   `run cold-start` reports five launches' median to the empty state; the full lane's
                             #   `desktop-e2e` job runs both on Linux, macOS and Windows
cargo xtask desktop-notices [--bundle]  # the desktop app's THIRD_PARTY_NOTICES.md (cargo-about, MPL-2.0 text included) into
                             #   target/desktop/; --bundle adds the frontend's npm packages and fonts into
                             #   src-tauri/resources/THIRD_PARTY_NOTICES.txt, which tauri.release.conf.json bundles (Help ▸ View licences)
cargo xtask desktop-release plan | sizes <dir> | sbom <target> <dir>  # the desktop release's decisions (D6b): the run's
                             #   version, channel, environment and matrix into $GITHUB_OUTPUT (desktop_release/plan.rs, table-tested);
                             #   installers against the 16 MB budget; the app's and engine's CycloneDX SBOMs and the npm SBOM
cargo xtask crash symbolicate <report.json> [--symbols <dir>] [--fetch]  # a desktop crash report's native frames with
                             #   function names and lines (D7d): each module's Breakpad symbols from <dir>, else target/crash-symbols/,
                             #   and with --fetch only, fetched there from the private R2 bucket lamina-pdf-symbols by the repository's
                             #   Wrangler with your own login (runbook docs/guidelines/desktop-release.md §7); frames without symbols
                             #   print module and offset; the report's strings are printed without control characters
cargo xtask desktop-sidecar [--release|--debug] [--e2e]  # the engine into apps/desktop/src-tauri/binaries/ (Tauri's -<triple> name) and
                             #   pinned PDFium into src-tauri/resources/pdfium/ (both git-ignored); the profile defaults to the
                             #   Tauri CLI's (release unless its hook sets TAURI_ENV_DEBUG=true); desktop:dev/desktop:build run it first
cargo xtask fixtures large-pdf [--pages N] [--out PATH]  # the engine's 1 GB fixture (20,000 image pages, ~1.1 GB, never
                             #   committed) into target/fixtures/1gb.pdf; needs ~1.5 GB of RAM
cargo run -p laminapdf-cli --features mcp -- mcp --root <dir> [--allow-write [--allow-in-place]]  # the MCP server on stdio (L4c)
cargo xtask models check     # tools/models/models.toml: code and weights licences in the allowed set, pins well formed, no
                             #   weights file in the repository (part of `check docs`)
cargo xtask models fetch <name>  # a model's weights into target/models/<name>/<sha256>/, kept only when hash and size match
cargo xtask fetch-pdfium     # the pinned native PDFium (pdfium-binaries chromium/7881, SHA-256 per host) and its notices,
                             #   unpacked into $LAMINA_PDF_PDFIUM_CACHE, $RUNNER_TOOL_CACHE/lamina-pdfium or target/pdfium; prints LAMINA_PDF_PDFIUM_DIR
python tools/fonts/subset.py <dir> [--check]  # @laminapdf/fonts from the npm tarballs in <dir> (packages/fonts/README.md; Python with
                             #   tools/fonts/requirements.txt); --check fails unless files/, fonts.css and the lock are current
cargo xtask corpus facts     # rewrite fixtures/corpus/expected.json with pypdf ($LAMINA_PDF_PYTHON or python3)
cargo xtask differential     # Lamina PDF vs pypdf's precomputed facts (corpus + fixed-seed mutants; part of `test`)
cargo xtask differential --update  # regenerate fixtures/differential/expected.json with pypdf
cargo xtask differential --text [--update]  # Lamina PDF's page text vs pdf.js and pdftotext (whitespace and bidi controls removed) from
                             #   fixtures/differential/text.json (tools/differential/text.mjs), exceptions in text-exceptions.json
cargo xtask quads [--update] # Lamina PDF's glyph quads vs PDFium's loose char boxes, fixtures/text/charboxes.json (tools/render/charbox.py)
cargo xtask differential --full [--update]  # the full lane's set: 36 mutants per corpus file (full.json; needs cargo-nextest)
cargo xtask roundtrip [--tools qpdf,pdfjs,pdfium]  # Lamina PDF saves fixtures x ops x modes, reopens each output; qpdf --check,
                             #   pdf.js and PDFium check them (needs qpdf, Node, Python with tools/roundtrip/requirements.txt)
cargo xtask fuzz <target> [--time N]  # cargo +nightly fuzz: lex, parse_object, flate_decode, open, page_tree, form_tree, content,
                                      #   content_append_scan, content_ir, ops_json, save_roundtrip, copy_pages, extract_text, extract,
                                      #   markdown, document, xmp_parse, engine_protocol_decode, xfdf, fdf
cargo xtask fuzz --all [--time N] [--jobs N]  # every fuzz target, N at once (the full lane: --jobs 4,
                                      #   30 s per target on PRs, 120 s on pushes to main)
cargo xtask fuzz cmin [<target>]      # minimise fuzz/corpus/ (the full lane does this before caching it on main)
cargo xtask changes [--base <rev>]    # paths changed since the merge base with <rev> (default origin/main, else main);
                                      #   code=true unless all are on the docs allowlist; gallery=true when they touch
                                      #   apps/gallery, packages/{ui,tokens,icons,fonts} or pnpm-lock.yaml; web=true when they touch
                                      #   what the reader is built from (the `web` suite's graph inputs, the manifests, tools/xtask,
                                      #   web-deploy.yml; `web next-release` asks the same); suites lists the runtime suites
                                      #   (changes/suites.rs: the SDK's examples, nine packages' browser tests, the reader, the
                                      #   docs site, the website), each from its package's workspace dependencies, the crates of
                                      #   the WebAssembly when it reads them, its own files, and what every leg reads (fixtures,
                                      #   manifests, tools/xtask, ci.yml, root files), which ci.yml turns into the runtime legs
                                      #   (X12, CI caching plan §8.3);
                                      #   desktop=true when they touch the crates and packages the desktop app builds (from
                                      #   cargo metadata and the package.json files), the manifests, lockfiles, toolchain, the
                                      #   fixtures its scenarios open, tools/xtask, or the full lane (changes/desktop.rs, X10a);
                                      #   the full lane splits it into desktop-build and desktop-e2e; desktop-key is a digest of
                                      #   those inputs' contents (X10b); codeql lists the CodeQL languages
                                      #   the paths are in (X10c);
                                      #   render=true when they touch crates/, the example and render goldens, the corpus,
                                      #   tools/render, tools/xtask, the manifests and toolchain, or full.yml (RENDER_INPUTS, X9b);
                                      #   differential-<facts|text|xmp|inspect>=true when they touch that live differential's
                                      #   code, exceptions files, committed PDFs, example crate and the crates it builds from,
                                      #   tool runners or pins, or full.yml (changes/differential.rs, over live/mode.rs's modes).
                                      #   differential=true when any is, and the full lane's `live-differentials` job then runs
                                      #   those with --sample 40;
                                      #   rust=true unless all are docs, apps/{web,docs,site,gallery,updates} or packages/ (packages/sdk
                                      #   and any package's src/ still count); ts=true unless all are docs or in fuzz/, spikes/,
                                      #   tools/ or other workflows (tools/xtask, cli-release, fonts, Biome-linted files and the
                                      #   two lanes still count). Each group of jobs runs on its flag, and both gates check it
                                      #   (X9c; CI caching plan §8.1)
cargo xtask desktop-key               # the key of the desktop's inputs at HEAD: each desktop leg in the full lane saves
                                      #   desktop-passed-<key>-<build|e2e>-<os> when it passes, and the changes job leaves out
                                      #   legs that the PR's earlier runs or main already passed on that key (X10b)
cargo xtask fuzz tmin <target>        # minimise every crash in fuzz/artifacts/<target>/
cargo xtask corpus fetch     # external corpora (pinned commits, SHA-256 per file in fixtures/corpus-manifest.toml) → target/corpus/
cargo xtask differential --live [--corpus external|mutants|all] [--source <name>]… [--tools pdfjs,pdfium,qpdf,pypdf] [--sample N]
                             # nightly: Lamina PDF vs live pdf.js, PDFium, qpdf, pypdf over the external corpus and the mutants
                             #   (250 per corpus file from the fast set's series + the full set's 36); --sample is stratified per source
cargo xtask differential --live --text [--source <name>]… [--sample N]  # nightly: Lamina PDF's page text vs live pdf.js and pdftotext
                             #   over the external corpus, page by page (B9c's rule; a page agrees when Lamina PDF matches one tool);
                             #   agreement per source in the summary, report in target/nightly/differential-text/
cargo xtask differential --live --xmp [--source <name>]… [--sample N]  # nightly: the XMP values Lamina PDF reads (the xmp-facts example
                             #   of laminapdf-model) vs ExifTool's (`exiftool -j -G1 -n -XMP:all`, pinned in doctor.rs) over the external
                             #   corpus and fixtures/{corpus,golden,golden/examples}/*.pdf, field by field (dates as instants); a difference
                             #   fails unless fixtures/differential/xmp-exceptions.toml has a [[known]] entry; report in target/nightly/differential-xmp/
cargo xtask render-diff [--corpus external] [--source <name>]… [--pages N] [--sample N] [--jobs N] [--top N] [--reuse] \
    [--history <dir> [--runner <label>] [--record] [--dry-run]]  # nightly: hayro vs PDFium (laminapdf-render's adapters, tools/render's
                             #   render-diff) at 96 dpi with C1a's metric and thresholds (ADR 0004 §6 note); per-file processes
                             #   under time limits; pass+minor share per source; report.json + summary.html (worst diffs) in
                             #   target/nightly/render-diff/; --history: a drop > 2 points vs the last point writes regression.md
                             #   (--dry-run prints the issue), --record appends `render-diff/<source>` to history.jsonl; --jobs
                             #   defaults to the cores; --reuse reports the restored report again when the renderer's code, corpus
                             #   manifest and settings hash to its inputs.sha256 (render_diff/reuse.rs); a file the pinned PDFium
                             #   crashed or hung on alone in the last report (same file, under 7 days old) skips PDFium and
                             #   is rendered by hayro alone (render_diff/known.rs)
cargo xtask scaling          # nightly: the scaling tests (SCALING_TESTS: the #[ignore]d tests sent to the nightly lane; a test keeps
                             #   the list complete) in the `scaling` profile (release without LTO), one retry reported as flaky
cargo xtask bench [--quick]  # nightly: criterion benches (crates/laminapdf-cos/benches, crates/laminapdf-save/benches, crates/laminapdf/benches:
                             #   the 100-page text report, the flowing report, merging 100 files); --history <dir> flags >10% regressions
cargo xtask nightly [--quick]  # the whole nightly lane locally, in the workflow's order (--quick: short budgets)
cargo xtask proof [--from <dir>] [--jobs <json>] [--history <dir> --runner <label> [--record]]
                             # the nightly's robustness figures in one record (kind "proof", version 1): Lamina PDF's outcome per
                             #   corpus source, the differentials, the render diff, the fuzz summaries, veraPDF's report.json, the
                             #   round trip's summary.json, pdf-runtimes' runtimes.json, each read where its task writes it (under
                             #   --from); a missing input is a null section and --jobs (toJSON(needs)) says why; `full` from the
                             #   reports' budgets; writes target/nightly/proof/proof.json and a summary table; --record (with --history
                             #   and --runner) appends to <dir>/history.jsonl only a full record; `nightly` runs it last, never recording
cargo xtask proof snapshot [--offline]  # write apps/site/src/data/proof.json, How we test's figures (T1b): the newest full proof
                             #   record on origin/bench-data cut to what the page shows, how many of the last 30 full nights passed
                             #   (every job success), the fuzzing hours over every full night; --offline, no branch or no full
                             #   record keeps the committed file with a warning, and a newest night over 3 days old warns; the
                             #   website's production deploy runs it before every build; refreshing it is an exempt PR
cargo xtask ci               # the whole fast lane locally: check + wasm + size
pnpm changeset               # add a changeset to a user-visible change (contributing guide: changesets; ADR 0017)
pnpm changeset version       # release prep: consume the changesets, bump the fixed group's packages, write their CHANGELOG.md
cargo xtask version sync [--check]  # the fixed group's version (members read from .changeset/config.json; all must agree)
                             #   into [workspace.package], the internal [workspace.dependencies], every tracked
                             #   Cargo.lock, CHANGELOG.md (every member's entries merged) and @laminapdf/sdk's VERSION (packages/sdk/src/version.ts);
                             #   --check fails on any drift or a crate without version.workspace = true (part of `check docs`); both fail on a private
                             #   packages/* or apps/* package missing from .changeset/config.json's `ignore` (Changesets would version it)
cargo xtask version check-tag <tag>  # fail unless <tag> is v<version> (the release lane)
cargo xtask verify-install [node|deno|bun|chromium|workerd|rust]… [--node <bin>]… [--git-ref <tag-or-sha>] [--git-url <url>] [--keep]
                             # after `wasm`: pnpm pack @laminapdf/wasm and @laminapdf/sdk (not the render packages yet), run the quick start in clean projects in the temp dir
                             #   (npm-installed tarballs; a Vite app in Playwright's Chromium; a Worker under wrangler dev
                             #   --local; Rust binaries with path and git deps on Lamina PDF, git default HEAD of this clone);
                             #   each output must equal fixtures/golden/examples/hello.pdf
cargo xtask verify-install … --from-registry [--version <v>] [--wait <minutes>]  # the same runtimes, installed from npm
                             #   (@laminapdf/sdk and @laminapdf/wasm at <v>, default the checkout's) and crates.io (laminapdf = "=<v>");
                             #   --wait polls the registries every minute until both have <v> (the release lane, after publishing)
cargo xtask semver [--baseline <v>]  # after `wasm`: is the pending changesets' bump big enough? cargo-semver-checks (pinned in
                             #   doctor.rs) on every published crate vs crates.io at <v> (default the newest published), crates missing there
                             #   listed as unchecked; API Extractor reports of @laminapdf/sdk's entry points vs the published package;
                             #   breaking changes fail unless a pending changeset of the fixed group is minor or major
cargo xtask release prepare --run <nightly-run-id>
                             # clean checkout of that full nightly's exact commit; plans with Changesets,
                             # refuses a pending release, commits a checked version bump, never pushes
cargo xtask release-guard [--run <id>] [--commit <rev>] [--version-only]  # fail unless the latest completed full nightly.yml run on main, or run <id>, succeeded on <rev>
                             #   (default HEAD) or an ancestor (gh, git merge-base); full: scheduled, or dispatched with every
                             #   input at its default, titled "Nightly (full budgets)"; --version-only writes nightly=skip to
                             #   $GITHUB_OUTPUT when <rev> adds only a version bump to the commit that run tested (the changelogs, consumed
                             #   changesets and pre-mode files, and lines in manifests, lockfiles and version.ts whose only change is
                             #   the old version swapped for the new), else nightly=run
pnpm --filter @laminapdf/updates manifest <show|validate|put|pause|resume|set-rollout|promote> …  # the desktop
                             #   channel manifests in R2 (apps/updates/README.md; --local for wrangler dev's R2); every write checked
pnpm --filter @laminapdf/updates dev # the update Worker on `wrangler dev` with local R2
pnpm --filter @laminapdf/web dev     # the web reader (after `wasm` and the viewer's build:wasm; apps/web/README.md); `build`, then
                             #   `preview` serves dist/ through `wrangler dev` with the production headers on port 8788
cargo xtask web build        # the reader into apps/web/dist/ from a fresh checkout: the WebAssembly modules without their tests,
                             #   @laminapdf/sdk, the packages the app imports, then the app (what web-deploy.yml deploys)
cargo xtask web docs-runner  # what the docs' example runner needs (B7e): the WebAssembly modules without their tests,
                             #   @laminapdf/sdk, @laminapdf/render and -hayro; the docs build then makes the runner
pnpm --filter @laminapdf/site dev    # the website at localhost:4321; `build` writes apps/site/dist/ (noindex unless
                             #   LAMINA_PDF_ENV=production) and fails on a page that breaks the website's rules
pnpm --filter @laminapdf/site run downloads  # refresh src/data/downloads.json, the download page's release lists, from
                             #   the download hosts (production deploys run it; LAMINA_PDF_DOWNLOADS builds from another file)
pnpm --filter @laminapdf/docs dev    # the docs site at localhost:4321; `build` writes apps/docs/dist/ (what deploys), with a
                             #   Markdown twin per page (`/install.md`), llms.txt and llms-full.txt; `test` runs plugins/ and scripts/ tests
node apps/docs/scripts/package-docs.mjs <out-dir> [--api]  # the docs/ folder the packages ship: pages, code pages, AGENTS.md
                             #   (packages/sdk/examples/AGENTS.md), llms.txt, renames.json, schemas; --api adds the TypeDoc Markdown;
                             #   @laminapdf/sdk's prepack runs it; `release prepare` commits crates/laminapdf/docs/ in the release tree,
                             #   `publish crates` regenerates it (--allow-dirty only when it isn't committed as generated) and checks
                             #   `cargo package --list`, `publish npm-pack` checks the sdk tarball (tools/xtask/src/package_docs.rs)
node tools/agent-eval/run.mjs [--tasks a,b] [--modes docs,no-docs] [--dry-run]  # the agent eval (after `publish npm-pack`;
                             #   ANTHROPIC_API_KEY from the environment); results in target/agent-eval/
node tools/answer-check/run.mjs [--engine claude|openai] [--questions a,b] [--repeats N] [--dry-run]  # the answer-engine
                             #   check (W3i; after `pnpm --dir tools/answer-check install`; Claude: ANTHROPIC_API_KEY or an `ant auth login`
                             #   profile; OpenAI's Responses API: OPENAI_API_KEY, and NODE_USE_ENV_PROXY=1 behind a proxy); results in
                             #   target/answer-check/<date>-<engine>/
cargo xtask doctor           # are rustc, node, pnpm, cargo-deny, cargo-nextest, wasm-opt … at the pinned versions? On Linux,
                             #   are WebKitGTK 4.1 and D-Bus there (pkg-config), which building the workspace needs?
cargo xtask doctor --install # install the pinned cargo tools
```
