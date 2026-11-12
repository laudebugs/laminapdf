// Tests for typedoc-sources.mjs: `node --test plugins/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { test } from "node:test";
import { USAGE_PAGES } from "./sidebar.mjs";
import {
  addUsageLinks,
  isDependencyFile,
  removeSummarisedMembers,
  SUMMARISED,
  sourceRevision,
} from "./typedoc-sources.mjs";

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

/**
 * A project whose classes, by name, have the given doc comment summaries (`undefined`: no comment).
 *
 * @param {Record<string, import("typedoc").CommentDisplayPart[] | undefined>} summaries
 */
function projectWith(summaries) {
  /** @type {Record<string, { comment?: { summary: import("typedoc").CommentDisplayPart[] } }>} */
  const classes = Object.fromEntries(
    Object.entries(summaries).map(([name, summary]) => [name, summary === undefined ? {} : { comment: { summary } }]),
  );
  return /** @type {Pick<import("typedoc").ProjectReflection, "getChildByName">} */ (
    /** @type {unknown} */ ({
      /** @param {string} name */
      getChildByName: (name) => classes[name],
    })
  );
}

test("each class with a usage page ends its summary with a link to it", () => {
  /** @type {Record<string, import("typedoc").CommentDisplayPart[]>} */
  const summaries = Object.fromEntries(
    Object.keys(USAGE_PAGES).map((name) => [name, [{ kind: "text", text: `The ${name} class.` }]]),
  );
  addUsageLinks(projectWith(summaries));
  for (const [name, slug] of Object.entries(USAGE_PAGES)) {
    const text = (summaries[name] ?? []).map((part) => part.text).join("");
    assert.ok(text.startsWith(`The ${name} class.\n\n`), text);
    assert.ok(text.endsWith(`[using ${name}](/${slug}/).`), text);
  }
});

test("a usage page whose class is missing or undocumented fails the build", () => {
  assert.throws(() => addUsageLinks(projectWith({})), /Document has a usage page/);
  const documented = { Document: [{ kind: /** @type {const} */ ("text"), text: "An open PDF." }] };
  assert.throws(() => addUsageLinks(projectWith({ ...documented, Builder: undefined })), /Builder has a usage page/);
});
