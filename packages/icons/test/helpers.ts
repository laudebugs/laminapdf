// What the tests share: the repository files they compare the icons with.

import { readFileSync } from "node:fs";
import { fromRoot } from "@laminapdf/test-utils/root";

/** A repository file as text. */
export function readRepo(path: string): string {
  return readFileSync(fromRoot(path), "utf8");
}
