# Working on `@laminapdf/sdk`

The package's [README](README.md) is for people who use it, and the docs site renders it. This page is for people who change it: how it is built, and how its example suite runs in every runtime it supports. The design is [ADR 0016](../../docs/decisions/0016-package-design.md); the error contract is [ADR 0026](../../docs/decisions/0026-error-and-diagnostic-contract.md).

## Development

```sh
cargo xtask wasm   # builds @laminapdf/wasm, then this package (tsc), its tests (Vitest: Node, Vite), publint, attw --pack
cargo xtask pdf-runtimes   # the example suite in every runtime (see Runtimes)
cargo xtask size   # includes this package's JS (glue + API, min+gzip) against its 20 KB budget
cargo xtask gen codes   # regenerate src/generated/ and schemas/ from fixtures/codes.json (`check docs` fails when stale)
```

## Examples

[`examples/`](examples) holds the TypeScript twins of the Rust examples. Each produces the bytes of its Rust golden in [`fixtures/golden/examples/`](../../fixtures/golden/examples); `report` (100 pages, two embedded fonts) is the performance case of [pdf-library §4](../../docs/plans/pdf-library.md#non-functional), checked by its SHA-256 instead of a golden.

## Runtimes

The one example list, [`test/suite/examples.ts`](test/suite/examples.ts), runs in every runtime the package supports ([B6c](../../docs/todos/done/B6c-runtime-matrix.md)). It uses no Node API and no test framework: `runSuite(runtime, read, register)` registers it with a runner's test function. Each runtime compares every example with its golden byte for byte, and the report with its hash.

| Runtime | Runner | Export condition |
| --- | --- | --- |
| Node 22, 24, 26 | Vitest, [`vitest.node.config.ts`](vitest.node.config.ts): project `node` (`test/*.test.ts`, also the Vite tests; what `cargo xtask wasm` runs) and project `measure` ([`test/node/measure.test.ts`](test/node/measure.test.ts)) | `node` |
| Chromium, Firefox, WebKit | Vitest browser mode with Playwright, [`vitest.browser.config.ts`](vitest.browser.config.ts) (`test/browser/`; fixtures fetched as Vite assets); its `vite-build` project runs a Vite production build of [`test/vite-app/`](test/vite-app) in a Chromium page | `browser` |
| Deno | `deno test` over [`test/deno/`](test/deno) | `deno` |
| Bun | `bun test` over [`test/bun/`](test/bun) | `node` |
| workerd | Vitest in Node, [`vitest.workers.config.ts`](vitest.workers.config.ts): wrangler's test harness runs [`test/workerd/worker.ts`](test/workerd/worker.ts) and the Worker example in workerd, and the test compares their responses | `workerd` |

Each runtime also prints the performance smoke's figures, measured first in a fresh module instance: in Node from its own file, elsewhere in the first test of the file or the first request to a fresh Worker. So the first hello is cold, and the memory is the hello and report runs' alone. The figures are hello after `init()` (first call and median of 20), the report (median of 3) and the WebAssembly memory after it.

They are recorded, never asserted. `cargo xtask pdf-runtimes` collects them into a table against the §4 targets (≤ 5 ms, ≤ 500 ms, < 64 MB in a Worker; the job summary in CI) and `target/xtask/pdf-runtimes/measurements.json`; the nightly lane appends them to the benchmark history.

```sh
cargo xtask wasm                          # first: builds packages/wasm/dist/ and this package's dist/
cargo xtask pdf-runtimes                  # every runtime, in turn
cargo xtask pdf-runtimes chromium deno    # some of them (node, chromium, firefox, webkit, deno, bun, workerd)
```

`pdf-runtimes node` uses the `node` on `PATH`. Deno, Bun, wrangler (with workerd) and Playwright are dev dependencies of this package, so `pnpm install` brings them. `pdf-runtimes` runs `playwright install <browser>` before a browser, into Playwright's per-user cache or `PLAYWRIGHT_BROWSERS_PATH`. By hand, from this folder:

```sh
pnpm exec vitest run -c vitest.node.config.ts --project measure   # then --project node
pnpm exec vitest run -c vitest.browser.config.ts --project chromium --project vite-build   # or firefox, webkit
pnpm exec deno test --sloppy-imports --no-check --allow-read test/deno/
pnpm exec bun test ./test/bun/
pnpm exec vitest run -c vitest.workers.config.ts
```

In CI, the fast lane runs Node 22 and 26 and Chromium on every PR; Node 24 (`.nvmrc`) is the `wasm` job's own run. The full lane runs Firefox, WebKit, Deno, Bun and workerd, and the nightly lane runs all of them with the figures recorded.

## Decisions behind the API

- Loading per runtime, the two modules and their budgets: [ADR 0016](../../docs/decisions/0016-package-design.md) §3, §4, §6.
- Passwords (tried as given, then in NFKC form): ADR 0016 §7.
- Page space and 0-based indexes: [ADR 0027](../../docs/decisions/0027-coordinates-units-and-page-indexing.md).
- The worker client: [L2a](../../docs/todos/done/L2a-worker-client.md); its extensions for the viewer: [C3a](../../docs/todos/done/C3a-viewer-worker-rpc.md) and [ADR 0009](../../docs/decisions/0009-web-client-architecture.md) §3.
- Disposed handles and memory bounds: [L2b](../../docs/todos/ready/L2b-disposal-and-memory.md).
- Publishing: P1 and P2 in the [architecture plan](../../docs/plans/architecture.md).
