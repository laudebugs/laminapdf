# Resolution

The example now picks its dispose key the way the compiled `using` code does: `Symbol.dispose` where the browser has it, else `Symbol.for("Symbol.dispose")`. Its close method sits under that key and is also returned as `close()`. The guide shows `using` for a viewer that lives inside one block and recommends `close()` for one that outlives it; a second `close()` does nothing.

## Proof

- **A regression test that fails without the fix.** `packages/viewer/test/browser/show-a-pdf-without-symbol-dispose.browser.test.ts` deletes `Symbol.dispose`, loads the example, shows a PDF and disposes of it as compiled `using` does, then checks the container is empty. Against the old example it failed in Chromium, Firefox and WebKit with "Object not disposable"; with the fix it passes in all three.
- **The example's own tests.** `show-a-pdf.browser.test.ts` gains a test that `close()` takes the PDF down and that closing again does nothing; its `using` test still passes.
- **The package's browser suites**, through `cargo xtask pdf-runtimes webkit`.

## Left for later

The test browsers all have `Symbol.dispose`, so only a test that deletes it covers the missing-symbol path. Any other example that writes `[Symbol.dispose]` as a key has the same exposure; at the time of the fix, `packages/viewer/examples/show-a-pdf.ts` was the only one.
