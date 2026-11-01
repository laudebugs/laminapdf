# @laminapdf/test-utils

The test helpers lamina's TypeScript packages share, so no package reaches into another's `test/` folder. It is private and lives only in the workspace: never published, and left out of the version every other package shares.

Its modules are TypeScript source. Vitest compiles them for the tests, and Node strips their types when a Vitest config imports one, so there is nothing to build first. A module that a config imports reaches another one through the package name (`@laminapdf/test-utils/firefox`), not a relative path.

## Modules

- **`@laminapdf/test-utils/render-contract`:** `renderContract(options)`, the suite every `RenderBackend` runs, in Node and in Chromium, Firefox and WebKit. You pass a backend factory and a file reader, and in browsers a PNG decoder for the reference renders. It checks:
  - every corpus file's page count and size, against lamina's facts;
  - a blank tile is white;
  - four tiles stitch back into the whole page;
  - a 90° view equals the upright render rotated, with and without widgets;
  - typed errors (`render-failed`, `password-required`, `wrong-password`, `page-index-out-of-range`, `render-too-large`, and `data-pending` with its ranges);
  - `AbortError` and `disposed`;
  - 50 renders don't grow WebAssembly memory beyond 4 MiB;
  - in browsers, page 1 against the backend's reference renders (SSIM).
- **`@laminapdf/test-utils/browser-harness`:** `fetcher(urls, prefix)` reads fixtures served as Vite assets, and `decodePng(png)` decodes a PNG with the browser's own decoder.
- **`@laminapdf/test-utils/browser-config`:** `renderBrowserConfig(packageDir)`, the Vitest browser config the render adapters share.
- **`@laminapdf/test-utils/firefox`:** `firefoxProvider(packageDir)`, for every package's browser config. Playwright's Firefox looks for the user's Firefox data folder and fails where the system refuses access, as it does on the build machines. The provider gives it a folder of its own under the package's `node_modules`.
- **`@laminapdf/test-utils/root`:** `fromRoot(path)`, a file's URL from its path from the repository root (`fixtures/corpus/hello.pdf`), so a test needn't count `../` from its own folder.

## Where it doesn't reach

- **Vite globs:** `import.meta.glob` needs a literal path, so browser tests still list their fixtures relative to their own folder.
- **Deno, Bun and workerd:** these runners load the SDK's tests their own way, and they keep their own path to the root. So do the SDK's Cloudflare Worker example, which uses only the public API, and its benchmark script, which runs in plain Node.
- **The render package:** `@laminapdf/test-utils` depends on `@laminapdf/render`, so render's own tests keep their own path to the root and don't depend back on it.
