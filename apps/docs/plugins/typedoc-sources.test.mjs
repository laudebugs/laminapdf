// Tests for typedoc-sources.mjs: `node --test plugins/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { test } from "node:test";
import { isDependencyFile } from "./typedoc-sources.mjs";

test("a declaration in node_modules is a dependency's, on any platform", () => {
  assert.equal(
    isDependencyFile("/repo/node_modules/.pnpm/typescript@6/node_modules/typescript/lib/lib.es5.d.ts"),
    true,
  );
  assert.equal(isDependencyFile("C:\\repo\\node_modules\\typescript\\lib\\lib.es5.d.ts"), true);
});

test("the SDK's own files keep their links", () => {
  assert.equal(isDependencyFile("/repo/packages/sdk/src/document.ts"), false);
});
