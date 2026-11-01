// The repository root, so a test reads `fixtures/…` and other packages' build output by their
// path from the root instead of counting `../` from its own folder.

/** The repository root, as a `file:` URL ending in `/`. */
const ROOT = new URL("../../../", import.meta.url);

/** `path` (relative to the repository root, such as `fixtures/corpus/hello.pdf`) as a URL. */
export function fromRoot(path: string): URL {
  return new URL(path, ROOT);
}
