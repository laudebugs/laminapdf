---
title: "Troubleshooting JavaScript and TypeScript"
description: "The problems people hit with @laminapdf/sdk, from a missing init() to a bundler that loses the WebAssembly, with the cause and the fix of each."
version: "0.4.0"
url: "https://docs.laminapdf.com/troubleshooting/javascript/"
---

# Troubleshooting JavaScript and TypeScript

The problems people hit with @laminapdf/sdk, from a missing init() to a bundler that loses the WebAssembly, with the cause and the fix of each.

These entries cover `@laminapdf/sdk` in Node, browsers, bundlers and Cloudflare Workers: loading the WebAssembly modules, and the memory they use. Each entry starts from what you see, then gives the cause and the fix.

## `@laminapdf/sdk` has no `init` or `Document`

**Cause:** your project is on version `0.0.0`, a placeholder that only reserves the name. npm installs the `latest` tag unless you name a version or another tag, and before `0.2.0` Lamina PDF published only alphas, under the `alpha` tag, so a plain `npm install @laminapdf/sdk` picked the placeholder.

**Fix:** install the version these docs describe: `npm install @laminapdf/sdk@0.4.0`. See [Install](../../pages/install.md#typescript-laminapdfsdk).

## Every call throws `not-initialized`

**Cause:** a function of `@laminapdf/sdk` ran before `init()` finished loading the WebAssembly.

**Fix:** `await init()` once, before anything else. After it, every call is synchronous. See [`not-initialized`](../../errors/not-initialized.md).

## Drawing text throws `text-module-not-loaded`

**Cause:** the builder lives in the base module, which has no fonts or text. Lamina PDF loads only the modules you ask for, so a page downloads only what it uses.

**Fix:** load the text module before you create the builder: `init({ text: true })`, or `loadText()` after `init()`. See [`text-module-not-loaded`](../../errors/text-module-not-loaded.md) and [Text and fonts](../../pages/guides/fonts-and-text.md).

## Reading text throws `extract-module-not-loaded`

**Cause:** the document was opened in the base module, which can't extract text.

**Fix:** load the extract module, with `init({ extract: true })` or `loadExtract()`, or the text module, before you open the document. A Cloudflare Worker can't fetch it, so import it and pass it: `import extract from "@laminapdf/wasm/lamina_pdf_extract.wasm"`, then `await loadExtract(extract)`. See [`extract-module-not-loaded`](../../errors/extract-module-not-loaded.md).

## The browser blocks the WebAssembly

**Cause:** the page's Content Security Policy doesn't allow compiling WebAssembly.

**Fix:** add `'wasm-unsafe-eval'` to its `script-src`. Lamina PDF needs nothing else from the policy. See [Browser and bundlers](../../pages/quick-start/browser.md).

## `init()` fails in a Vite app

**Cause:** Vite pre-bundles dependencies into its own cache, and the `.wasm` files don't move with the JavaScript that loads them.

**Fix:** exclude the package from pre-bundling: `optimizeDeps: { exclude: ["@laminapdf/sdk"] }` in `vite.config.ts`. A Web Worker that uses `@laminapdf/sdk/worker` needs the same setting. See [Browser and bundlers](../../pages/quick-start/browser.md).

## `init()` fails in a Cloudflare Worker

**Cause:** Cloudflare Workers can't compile WebAssembly at run time, so `init()` can't build the modules from bytes or a URL.

**Fix:** let Wrangler bundle the package, and call `init()` without a URL or bytes. Wrangler picks the package's entry for Cloudflare Workers, which imports the `.wasm` files as compiled modules. See [Cloudflare Workers](../../pages/quick-start/workers.md).

## Memory keeps growing

**Cause:** documents and builders live in WebAssembly memory, and the garbage collector frees them late: too late for a long-running page or server. WebAssembly memory also never shrinks, so one large file leaves it large.

**Fix:** free each one when you're done: `using doc = …` frees it at the end of the block, and `doc.free()` at once. A call on a freed one throws `disposed`. To give grown memory back, call `reset()` while no document is in use: every module starts afresh, and handles from before throw `instance-reset`. See [You free what you create](../../pages/concepts/how-lamina-thinks.md#you-free-what-you-create).

## A call throws `out-of-memory` or `wasm-trap`

**Cause:** the WebAssembly module ran out of memory (it may grow to 2 GiB), or it trapped on a bug in Lamina PDF. Either leaves the module's memory half-written, so Lamina PDF replaces the module's instance before the next call.

**Fix:** open the document again from its bytes: documents, pages and builders from the old instance throw `instance-reset`. For `out-of-memory`, keep fewer documents open at once and lower the open limits for files you don't trust. Report a `wasm-trap` with the file that causes it. See [`out-of-memory`](../../errors/out-of-memory.md) and [`wasm-trap`](../../errors/wasm-trap.md).

Next: [opening and saving files](../../pages/troubleshooting/files.md), for passwords, signed files and limits.
