// The reader's copies of documents in the origin-private file system (OPFS), kept so a document
// can be read again without the network or a picker. They live in one folder, so clearing them is
// one call; the recent list is cleared beside it, by the platform.

import type { PlatformStorage } from "@laminapdf/ui";

/** The OPFS folder that holds the copies. */
export const CACHE_FOLDER = "documents";

/**
 * Removes every cached copy; nothing to do where the browser has no OPFS (or refuses it, as a
 * private window may) or nothing is cached.
 */
export async function clearDocumentCache(storage: PlatformStorage): Promise<void> {
  const root = await storage.opfs().catch(() => undefined);
  if (root === undefined) return;
  try {
    await root.removeEntry(CACHE_FOLDER, { recursive: true });
  } catch (error) {
    // No folder yet: nothing was cached.
    if (!(error instanceof DOMException && error.name === "NotFoundError")) throw error;
  }
}
