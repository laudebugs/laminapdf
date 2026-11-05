// What the tests share: tokens.json flattened, and the repository files they compare it with.

import { readFileSync } from "node:fs";
import { fromRoot } from "@laminapdf/test-utils/root";
import { flatten, type Token } from "../scripts/model.ts";

/** tokens.json as parsed JSON. */
export function tokensJson(): Record<string, unknown> {
  return JSON.parse(readFileSync(fromRoot("packages/tokens/tokens.json"), "utf8"));
}

/** tokens.json flattened by path. */
export function loadTokens(): Map<string, Token> {
  return flatten(tokensJson());
}

/** A repository file as text. */
export function readRepo(path: string): string {
  return readFileSync(fromRoot(path), "utf8");
}
