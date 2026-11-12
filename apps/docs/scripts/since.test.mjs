// Tests for since.mjs: `node --test scripts/` (the docs site's `test` script).

import assert from "node:assert/strict";
import { test } from "node:test";
import { hasSince, missingSince } from "./since.mjs";

const tag = (/** @type {string} */ text) => ({ tag: "@since", content: [{ kind: "text", text }] });
const reflection = (/** @type {string} */ name, /** @type {object} */ extra) => ({
  name,
  getFullName: () => name,
  sources: [{ fileName: "src/a.ts", line: 3 }],
  ...extra,
});

test("a @since tag counts on the reflection, its signature or its accessor", () => {
  assert.equal(hasSince(reflection("A", { comment: { blockTags: [tag("0.2.0")] } })), true);
  assert.equal(hasSince(reflection("f", { signatures: [{ comment: { blockTags: [tag("0.2.0")] } }] })), true);
  assert.equal(hasSince(reflection("x", { getSignature: { comment: { blockTags: [tag("1.0.0")] } } })), true);
});

test("a @since tag needs a full version", () => {
  assert.equal(hasSince(reflection("A", { comment: { blockTags: [tag("0.2")] } })), false);
  assert.equal(hasSince(reflection("A", { comment: { blockTags: [tag("")] } })), false);
  assert.equal(hasSince(reflection("A", { comment: { blockTags: [] } })), false);
  assert.equal(hasSince(reflection("A", {})), false);
});

test("missing lists the reflections without one, leaving out inherited members", () => {
  const project = {
    getReflectionsByKind: () => [
      reflection("Has", { comment: { blockTags: [tag("0.2.0")] } }),
      reflection("Lacks", { comment: { blockTags: [] } }),
      reflection("Inherited", { inheritedFrom: {} }),
    ],
  };
  assert.deepEqual(missingSince(project, 0), ["src/a.ts:3 Lacks"]);
});
