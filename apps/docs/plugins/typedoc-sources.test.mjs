// Tests for typedoc-sources.mjs: `node --test plugins/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { test } from "node:test";
import { isDependencyFile, removeSummarisedMembers, SUMMARISED, sourceRevision } from "./typedoc-sources.mjs";

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

test("the released docs link to the release's tag, other builds to main", () => {
  assert.equal(sourceRevision({ version: "0.2.0-alpha.6", release: true }), "v0.2.0-alpha.6");
  assert.equal(sourceRevision({ version: "0.2.0-alpha.6", release: false }), "main");
});

test("a summarised type keeps its page but loses its members; other types keep theirs", () => {
  /** @type {string[]} */
  const removed = [];
  /** @param {string} name */
  const member = (name) => /** @type {import("typedoc").Reflection} */ (/** @type {unknown} */ ({ name }));
  /** @type {Record<string, { children: import("typedoc").Reflection[] }>} */
  const types = {
    [SUMMARISED[0] ?? ""]: { children: [member("blocks"), member("page")] },
    Other: { children: [member("a")] },
  };
  const project = /** @type {Pick<import("typedoc").ProjectReflection, "getChildByName" | "removeReflection">} */ (
    /** @type {unknown} */ ({
      /** @param {string} name */
      getChildByName: (name) => types[name],
      /** @param {import("typedoc").Reflection} child */
      removeReflection: (child) => {
        removed.push(child.name);
      },
    })
  );
  removeSummarisedMembers(project);
  assert.deepEqual(removed, ["blocks", "page"]);
});
