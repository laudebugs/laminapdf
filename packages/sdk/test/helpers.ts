// Shared test helpers: files from the repository's fixtures/ (tests may read files; src/ may not).
import { readFileSync } from "node:fs";
import { fromRoot } from "@laminapdf/test-utils/root";

/** A file under the repository root, as bytes. */
export function read(path: string): Uint8Array {
  return new Uint8Array(readFileSync(fromRoot(path)));
}

/** A file under the repository root, as text. */
export function readText(path: string): string {
  return readFileSync(fromRoot(path), "utf8");
}

/** What `f` threw. */
export function thrown(f: () => unknown): unknown {
  try {
    f();
  } catch (e) {
    return e;
  }
  throw new Error("expected the call to throw");
}
