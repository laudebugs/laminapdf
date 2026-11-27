//! `cargo xtask help`: every task and its arguments, one place for the text `main` prints.

pub(crate) const USAGE: &str = "\
usage: cargo xtask <task> [args]

tasks:
  check [rust|ts|docs]
                    everything a PR must pass (default: docs, rust and ts); runs docs-prose,
                    docs-snippets, biome-examples and clippy-examples with --all-strict
                    docs: docs-links, docs-prose, docs-snippets, todos --check, portable paths, the capability
                    matrix's static checks (capabilities --check runs the tests),
                    gen codes --check, gen boundary --check, gen licence --check and version sync --check
                    rust: fmt --check, maintainability, portability, clippy (native + wasm32), clippy-examples,
                          tests, rustdoc, check since, cargo-deny
                    ts:   biome ci, biome-examples, typecheck and tests of the pnpm
                          workspace, the docs site's and the website's builds (apps/docs,
                          apps/site), npm licences
  check since       every public item of a published crate has an `Added in X.Y.Z.` line in its
                    docs (rustdoc's JSON, RUSTC_BOOTSTRAP=1)
  check maintainability
                    docs/guidelines/maintainability.md's Rust rules, without a build: modules of at most 800
                    lines of non-test code and no more size-lint #[expect]s than
                    tools/xtask/maintainability-baseline.toml records, no public type name in two crates
                    and no name the lamina façade re-exports twice beyond
                    tools/xtask/maintainability-duplicates.toml (both files only shrink)
  check docs-prose [--all-strict] [--warnings] [--json]
                    reader-facing text (docs site, docs/errors, package READMEs) against
                    docs/guidelines/docs-writing.md: on its own, errors on files changed since the
                    merge base with main (drafts included), counted warnings elsewhere;
                    --all-strict fails on every report, --warnings lists them
                    (--json lists every report, so it takes no --warnings)
  check docs-snippets [--all-strict] [--warnings] [--json]
                    the examples and Snippet's glob against docs/guidelines/examples.md: on its own,
                    strict on changed examples, warnings elsewhere; TS/Rust pairs in
                    tools/xtask/docs-snippets.toml; skips examples/differential/ (a developer tool)
  biome-examples [--all-strict]
                    Biome with --error-on-warnings on the changed TS examples (all with --all-strict)
  clippy-examples [--all-strict] [--warnings] [--json]
                    clippy on lamina's examples (functions of at most 40 lines, no discarded
                    results): on its own, strict on changed examples, warnings elsewhere
  gen codes [--check|--md]
                    validate fixtures/codes.json against fixtures/codes.schema.json and write
                    the error contract from it: ErrorCode/WarningCode
                    (crates/laminapdf-syntax/src/generated/), the façade's explain pages
                    (crates/laminapdf/src/generated/), @laminapdf/sdk's unions, tables and explain
                    pages (packages/sdk/src/generated/), the laminapdf.error/1 JSON Schema
                    (packages/sdk/schemas/, crates/laminapdf/schemas/), @laminapdf/render's code
                    tables (packages/render/src/generated/) and docs/errors/README.md's tables;
                    --check fails naming each stale file; --md prints the tables
  gen licence [--check]
                    apps/desktop/src-tauri/licence.rtf, the licence the macOS disk image shows,
                    from LICENSE-MIT: the same words, whole paragraphs, the system's sans-serif
  gen boundary [--check]
                    the TypeScript side of the lists that cross the WASM boundary, read from the
                    Rust source: @laminapdf/sdk's builder enums (laminapdf-wasm's name tables,
                    TagKind::as_str) and @laminapdf/render-hayro's HAYRO_VERSION (the hayro pin);
                    --check fails naming each stale file
  fmt [--check]     format Rust (cargo fmt) and TS/JSON (biome); --check only verifies
  lint              clippy native (all targets) and wasm32 (core crates), biome ci
  test              Rust tests (cargo-nextest when installed, else cargo test) + doc tests, TS tests
  portability       core crates: forbidden-API grep, dependency and unsafe rules,
                    cargo check for wasm32, iOS and Android
  wasm              build the wasm32 crates with the size-optimised `wasm-release` profile, then
                    the @laminapdf/wasm modules (lamina_pdf_base, lamina_pdf_text) into packages/wasm/dist/
                    (wasm-bindgen, wasm-opt) and their tests in Node
  web build         the reader (apps/web) into apps/web/dist/ from a fresh checkout: the
                    @laminapdf/wasm modules without their tests, @laminapdf/sdk, the packages the
                    app imports, then the app (what .github/workflows/web-deploy.yml deploys)
  web docs-runner   what the docs' example runner needs, from a fresh checkout: the WebAssembly
                    modules (without their tests), @laminapdf/sdk, @laminapdf/render and
                    @laminapdf/render-hayro; then the docs build makes the runner
  web size          the reader's startup JavaScript (index.html's scripts and module preloads in
                    apps/web/dist/), gzipped, against apps/web/size-budget.json; after `web build`
  web next-release [--commit <rev>] [--since <tag>] [--notes <file>]
                    whether <rev> (default HEAD) is a new reader release: what ships changed since
                    <tag> (default the newest web-v* tag); prints and sets ready, tag and previous
                    in $GITHUB_OUTPUT, and writes the notes (the commits that changed it) to <file>
  web check-tag <tag>
                    fail unless <tag> is web-v<version> on apps/web/package.json's major and minor,
                    below no other web-v* tag
  size [--update-baseline]
                    measure packages/wasm/dist/ (raw, gzip, brotli; glue min+gzip), write
                    target/size-report.json, fail above packages/wasm/size-budgets.json
  deny              cargo-deny (licences, advisories, bans, sources) + npm licence check
  npm-licenses      npm licence check only (pnpm licenses list)
  docs-links [--external [--cache <file>]] [--json]
                    check every relative link and #anchor in the tracked Markdown files;
                    --external checks the http(s) URLs over the network instead (nightly;
                    allowlist in .config/link-check.toml, report in target/nightly/links/);
                    --cache <file> skips URLs that answered OK in the last 4 to 7 days and
                    updates the file
  todos [--check]   check the todo files, point each Depends on link at its todo's folder, print
                    the todos that can start now and write the whole board (never committed) to
                    target/todos/board.md; --check only verifies
  progress [--offline]
                    write apps/site/src/data/progress.json, the website's roadmap and dashboard:
                    the board's counts, the roadmap's milestones, merged and open pull requests
                    (gh); --offline keeps the last snapshot's in-progress list
  indexnow snapshot <sitemap> --out <file>
  indexnow submit --before <snapshot|sitemap> --after <sitemap> --key-file <file> [--dry-run]
                    IndexNow pings (production deploys only): snapshot records the live sitemap
                    before a deploy; submit posts the pages added or whose <lastmod> moved to
                    api.indexnow.org with the site's key; failures only warn
  capabilities [--check]
                    run every test tagged `/// capability: <ID> <surface> <status>` and
                    rewrite docs/capabilities/README.md from what they prove; --check only
                    verifies (and fails if a tagged test fails)
  golden [--update] regenerate every golden in fixtures/golden/ from its source and print the
                    list (--update writes them, the lamina examples' in fixtures/golden/examples/,
                    the reading-order corpus in fixtures/reading-order/, and the shaping goldens
                    in fixtures/shaping/ when hb-shape is installed;
                    without it, fail if they would change)
  examples          run the lamina crate's examples into target/examples-out/, compare them
                    with fixtures/golden/examples/ and run qpdf --check on each (needs qpdf)
  docs-previews     the docs' Result-tab pictures: every page of fixtures/golden/examples/ by hayro at
                    144 dpi into apps/docs/public/examples/preview/, with manifest.json (`check docs`
                    compares it with the PDFs)
  render-golden [--update]
                    render every page of fixtures/golden/examples/ with PDFium (pypdfium2:
                    tools/render/requirements.txt, $LAMINA_PDF_PYTHON) and hayro (tools/render, its
                    own workspace) at 72 dpi, compare with fixtures/render-golden/ (thresholds.toml;
                    diff images in target/xtask/render-golden/), and hayro vs PDFium at 150 dpi;
                    --update rewrites the PNGs (and still fails on a parity disagreement)
                    then laminapdf-render's PDFium and hayro adapters on fixtures/corpus/ at 96 dpi
                    against fixtures/golden/render/ (fetches PDFium first; laminapdf-render-pdfium's
                    ignored tests, or its `goldens` example with --update)
  render-diff [--corpus external] [--source <name>]… [--pages N] [--sample N] [--jobs N] [--top N]
              [--history <dir> [--runner <label>] [--record] [--dry-run]]
                    nightly: every page of the external corpus by laminapdf-render's hayro and PDFium
                    adapters at 96 dpi, compared with the hayro-parity spike's metric (tools/render's
                    render-diff, per-file processes with time limits); report.json, summary.html
                    with the worst diffs in target/nightly/render-diff/; with --history, a pass-rate
                    drop of more than 2 points writes regression.md (--dry-run prints the issue)
  annots-parity [--update] [--corpus]
                    every annotation subtype x colour x opacity x border on pages turned 0, 90, 180
                    and 270 degrees (tools/render's annots-parity, through the facade), rendered by
                    PDFium (tools/render/pdfium.py), pdf.js (tools/render/pdfjs-render.mjs, its own
                    pnpm project) and hayro at 72 dpi against fixtures/render-golden/annots-parity/,
                    and pdf.js and hayro vs PDFium at 150 dpi (annots-parity.toml; diff images in
                    target/xtask/annots-parity/diff/); --update rewrites the PNGs; --corpus also
                    moves every annotation of the pdf.js corpus's annotation-*.pdf by 1 pt, saves
                    incrementally, and checks each file with Lamina PDF, hayro, qpdf --check and PDFium
  corpus facts      rewrite fixtures/corpus/expected.json with pypdf (needs Python: see
                    tools/corpus-facts/requirements.txt; $LAMINA_PDF_PYTHON or python3)
  corpus scripts    the share of scripts Lamina PDF handles (ADR 0021 §3), per corpus source
                    and for the test forms, into target/nightly/form-scripts/ (needs corpus fetch)
  corpus fetch [--source <name>]… [--update-manifest]
                    check out the external corpora pinned in fixtures/corpus-manifest.toml into
                    target/corpus/ and verify every file's SHA-256 (--update-manifest rewrites the lists)
  differential [--full | --text] [--update]
                    lamina vs pypdf over the corpus and fixed-seed mutants, from the precomputed
                    fixtures/differential/expected.json; --update regenerates it with pypdf.
                    --full: the full lane's 36 mutants per corpus file instead (full.json),
                    run through cargo-nextest
                    --text: lamina's page text vs pdf.js and pdftotext (whitespace and bidi
                    controls removed) from fixtures/differential/text.json; --update regenerates
                    it (tools/differential/text.mjs; pdftotext at the pinned poppler)
  differential --live [--corpus external|mutants|all] [--source <name>]… [--tools a,b] [--sample N]
                    nightly: lamina vs live pdf.js, PDFium, qpdf and pypdf over the external corpus
                    and 250 mutants per corpus file; report in target/nightly/differential/
  differential --live --text [--source <name>]… [--sample N]
                    nightly: lamina's page text vs live pdf.js and pdftotext over the external
                    corpus (whitespace and bidi controls removed; a page agrees when lamina
                    matches one tool); report in target/nightly/differential-text/
  differential --live --xmp [--source <name>]… [--sample N]
                    nightly: the XMP values lamina reads vs ExifTool's (pinned in doctor.rs) over
                    the external corpus and the committed fixture PDFs, field by field; exceptions
                    in fixtures/differential/xmp-exceptions.toml; report in target/nightly/differential-xmp/
  differential --live --forms [--source <name>]… [--sample N]
                    nightly: each form field's name, kind and value lamina reads vs pypdf's and
                    pdf.js's over the external corpus and fixtures/forms; exceptions in
                    fixtures/differential/forms-exceptions.toml; report in target/nightly/differential-forms/
  fetch-pdfium      download the pinned native PDFium (pdfium-binaries chromium/7881) for this host,
                    check its SHA-256, unpack the library and its notices, print LAMINA_PDF_PDFIUM_DIR
                    (cache: $LAMINA_PDF_PDFIUM_CACHE, $RUNNER_TOOL_CACHE/lamina-pdfium or target/pdfium)
  cli-docs          the laminapdf CLI's man pages and shell completions into target/cli-docs/
  cli-dist          cli-docs plus the third-party notices (cargo-about) into target/cli-docs/:
                    the files dist-workspace.toml puts in every release archive
  desktop-notices [--bundle]
                    the desktop app's third-party notices, MPL-2.0 text included (cargo-about),
                    into target/desktop/; --bundle adds the frontend's npm packages and fonts and
                    writes apps/desktop/src-tauri/resources/THIRD_PARTY_NOTICES.txt, which release
                    builds bundle (tauri.release.conf.json)
  desktop-release plan | sizes <dir> | sbom <target> <dir>
                    the desktop release workflow's decisions: the run's version, channel,
                    environment and build matrix from the Actions environment into $GITHUB_OUTPUT;
                    the installers under <dir> against the 16 MB budget; the app's and engine's
                    CycloneDX SBOMs for <target> and the frontend's npm SBOM into <dir>
  crash symbolicate <report.json> [--symbols <dir>] [--fetch]
                    a desktop crash report's native frames with function names and source lines:
                    each module's Breakpad symbols from <dir>, else target/crash-symbols/, and with
                    --fetch, fetched there from the private R2 bucket lamina-pdf-symbols by Wrangler
                    with your own Cloudflare login; a frame without symbols prints its module and offset
  fixtures large-pdf [--pages N] [--out PATH]
                    write an image-heavy PDF with the laminapdf builder (default 20,000 pages, about
                    1.1 GB, into target/fixtures/1gb.pdf; never committed): the desktop engine's
                    large-file fixture. Needs about 1.5 GB of RAM
  desktop-frontend  the packages the desktop app's frontend imports through their built dist/: the
                    WebAssembly modules without their tests, @laminapdf/sdk, the viewer with its render
                    backends, @laminapdf/ui with its tokens, icons and fonts
  desktop-sidecar [--release | --debug]
                    build the desktop engine binary into apps/desktop/src-tauri/binaries/ (Tauri's
                    sidecar naming) and the pinned PDFium into src-tauri/resources/pdfium/, which
                    tauri.bundle.conf.json bundles; the profile defaults to the Tauri CLI's
                    (release unless its hook sets TAURI_ENV_DEBUG=true), else debug
  extract-bench [--subset <name>]… [--sample N] [--tables lattice-and-stream] [--update-lock]
                    score lamina's extraction on the pinned public benchmark of
                    tools/bench/extract-datasets.toml: fetch into target/bench/datasets/ (SHA-256 per
                    file against its lock), write Markdown with the extract-bench example, score it with
                    the benchmark's own tests (Python with tools/bench/requirements.txt; $LAMINA_PDF_PYTHON
                    or python3); report in target/bench/extract/<dataset>/, with <candidate>-results.json
                    (scores, pages a second, the machine) for `benchmarks`
  benchmarks [--no-run]
                    write apps/docs/src/data/benchmarks.json, the docs site's benchmarks page: run
                    `size` (after `wasm`), `pdf-runtimes` in every runtime and `bench` (not with
                    --no-run), then copy their reports and the newest extract-bench results; a
                    missing report keeps the committed section; run it on an idle machine
  verapdf [--dir <dir>]
                    run the lamina examples into target/examples-out/ (or take the PDFs in <dir>)
                    and validate each with veraPDF for the profiles its XMP claims (PDF/A-2b, 2u,
                    3b, PDF/UA-1); any violation fails; summary in target/xtask/verapdf/summary.md.
                    veraPDF's pinned CLI (doctor.rs) is installed into $RUNNER_TOOL_CACHE/lamina-verapdf
                    or target/verapdf; Java: $LAMINA_PDF_JAVA, Homebrew's openjdk@21, or java on PATH
  quads [--update]  lamina's glyph quads vs PDFium's character boxes from the precomputed
                    fixtures/text/charboxes.json; --update regenerates it (tools/render/charbox.py,
                    pypdfium2 from tools/render/requirements.txt; $LAMINA_PDF_PYTHON or python3)
  roundtrip [--tools qpdf,pdfjs,pdfium]
                    lamina saves every fixture x proving op x mode, reopens each output and
                    checks it with qpdf --check, pdf.js and PDFium (needs qpdf, Node and
                    Python with tools/roundtrip/requirements.txt; outputs in target/xtask/roundtrip)
  fuzz <target> [--time <seconds>]
                    run a fuzz target (lex, parse_object, flate_decode, open, page_tree, form_tree, af_script, content,
                    content_append_scan, content_ir, ops_json, save_roundtrip, copy_pages, extract_text,
                    extract, markdown, document, xmp_parse, engine_protocol_decode, xfdf, fdf, form_json) with
                    cargo +nightly fuzz (default 60 s); seeds come from fixtures/ and fuzz/seeds/
  fuzz --all [--time <seconds>] [--jobs <n>]
                    run every fuzz target, <n> at once (default 1: in turn; at most 8); built
                    once first, each run logged to target/xtask/fuzz/<target>.log and printed
                    whole when it ends; the per-input timeout is 10 s x <n> (the full lane: 30 s
                    on PRs, 120 s on main, --jobs 4)
  fuzz seeds [<target>]
                    only build the seed corpora in fuzz/corpus/
  fuzz cmin [<target>]
                    minimise the corpora in fuzz/corpus/ (cargo fuzz cmin)
  fuzz tmin <target>
                    minimise every crash input in fuzz/artifacts/<target>/
  scaling           nightly: release-mode #[ignore]d tests (scaling ratios); one retry, reported as flaky
  bench [--quick] [--history <dir> [--runner <label>] [--record]]
                    nightly: criterion benches; with --history, flag >10% regressions (never fails)
  pdf-runtimes [node|chromium|firefox|webkit|deno|bun|workerd]… [--suites <name,…>] [--label <name>] [--history <dir> --runner <label> --record]
                    @laminapdf/sdk's example suite in each runtime (default: all; needs `wasm` first;
                    installs the Playwright browsers it runs): byte-identical to the goldens, with
                    the timings and WebAssembly memory against pdf-library §4 in the job summary;
                    a browser also runs the packages' browser tests and the apps' e2e suites;
                    --suites runs only the named suites (`cargo xtask changes` prints them); a suite
                    still running after 10 minutes (an e2e suite: 20) is stopped and run once more,
                    and a pass then is reported flaky;
                    --label writes target/xtask/pdf-runtimes/measurements-<name>.json
                    and target/xtask/pdf-runtimes/; --record appends them to <dir>/history.jsonl
  viewer-perf [chromium|firefox|webkit]… [--quick] [--history <dir> --runner <label> [--record] [--dry-run]]
                    @laminapdf/viewer's perf harness (needs `wasm` first): first page, scrolling 1,000
                    pages, memory, fling and zoom against the web plan's §4 targets, written to
                    target/viewer-perf/ and the job summary (never fails on a target); --history flags
                    >10% regressions, --record appends to <dir>/history.jsonl, --dry-run prints the issue
  desktop-reload [--gate]
                    the desktop's content edit shown (E6f): a shape added to a page of files of about 1, 10
                    and 100 MB, the engine's appended revision, native PDFium reopening, the page drawn
                    again (builds the engine in release mode, fetches PDFium); the table goes to
                    target/desktop-reload/ and the job summary; --gate fails over a budget + 20 %
  semver [--baseline <version>]
                    the published crates (cargo-semver-checks) and @laminapdf/sdk's API report (API
                    Extractor, after `wasm`) against the newest published version; breaking changes
                    fail unless the changesets give the fixed group a minor bump (pending ones, or
                    those a version PR consumed while its version awaits npm)
  gallery [--test|--update]
                    the Paper component gallery (apps/gallery): serve it; --test runs its screenshot,
                    axe and font tests in the pinned Playwright image (Docker); --update rewrites
                    apps/gallery/e2e/__screenshots__/ there and lists what changed
  nightly [--quick] the whole nightly lane locally (corpus, differential, fuzz, scaling, render-diff,
                    verapdf, roundtrip, annots-parity --corpus, external links, bench, wasm +
                    pdf-runtimes, viewer-perf, desktop-reload),
                    then `proof` over what it wrote, never recording
  proof [--from <dir>] [--jobs <json>] [--history <dir> --runner <label> [--record]]
                    the nightly's robustness figures (corpus outcomes, differentials, render diff,
                    fuzz runs, veraPDF, round trip, runtimes) read from each task's output under
                    --from (default: the workspace root) into one record (kind proof) in
                    target/nightly/proof/proof.json and the job summary; a missing input is a null
                    section; --record appends it to <dir>/history.jsonl, only for a full-budget night
  proof snapshot [--offline]
                    apps/site/src/data/proof.json, the website's How we test figures: the newest
                    full proof record on origin/bench-data, how many of the last 30 full nights
                    passed, the fuzzing hours; keeps the committed file (with a warning) offline,
                    without the branch or with no full record, and warns when it is over 3 days old
  changes [--base <rev>]
                    list the paths changed since the merge base with <rev> (default origin/main)
                    and say which groups of CI jobs they can affect; sets `code`, `gallery`,
                    `web`, `desktop`, `render`, `rust`, `ts` and `codeql` (the CodeQL languages,
                    as JSON) in $GITHUB_OUTPUT
                    (default base: origin/main, or main when there is no origin/main); also
                    `desktop-key`, as `desktop-key` prints it
  desktop-key       print a key naming the contents of every file the desktop app's full-lane
                    jobs read at HEAD (the crates and packages it builds, from cargo metadata
                    and the package.json files): a leg that passed with it needn't run again
  version sync [--check]
                    copy the npm fixed group's version (`fixed` in .changeset/config.json;
                    every member must agree) into [workspace.package] version, the internal
                    [workspace.dependencies], every tracked Cargo.lock, CHANGELOG.md and
                    packages/sdk/src/version.ts (ADR 0017); --check only verifies
                    (and that every member crate has version.workspace = true)
  version check-tag <tag>
                    fail unless <tag> is v<version> (the release lane)
  verify-install [node|deno|bun|chromium|workerd|rust]… [--node <binary>]… [--git-ref <tag-or-sha>]
                 [--git-url <url>] [--keep]
                    pnpm pack both packages and run the quick start in clean projects in the
                    temp directory: Node (each --node), Deno, Bun, a Vite app in Chromium, a
                    Worker under wrangler dev --local, and Rust binaries with a path and a git
                    dependency on lamina (default ref: HEAD; default URL: this clone); each
                    output must equal fixtures/golden/examples/hello.pdf (needs `wasm` first)
  release prepare --run <nightly-run-id>
                    prepare a version-and-package-docs commit from the exact full-nightly commit; never pushes
  release-guard [--run <id>] [--commit <rev>]
                    fail unless the latest completed full nightly.yml run on main, or run
                    <id>, succeeded on <rev> (default HEAD) or an ancestor; full: scheduled, or
                    dispatched with every input at its default (gh; the release lane's first job)
  ci                check + wasm + size: the whole fast lane, locally
  doctor [--install]
                    check that external tools are installed at their pinned versions;
                    --install runs the `cargo install` fixes
  help              show this message";
