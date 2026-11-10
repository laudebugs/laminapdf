// At start, the document open when the reader last closed comes back (T5b): reopened from its
// kept handle where the browser kept the permission to read it, since nothing has been clicked to
// allow a prompt; else the home screen offers it as Continue reading (recent.tsx).

import type { Platform, ReaderController } from "@laminapdf/ui";
import { readChoices } from "../storage-choices.ts";
import { SESSION_KEY } from "./positions.ts";

/** Whether the choices let the reader come back to the last document: recent files and positions kept. */
function remembers(): boolean {
  const choices = readChoices();
  return choices.rememberLocal && choices.rememberPositions;
}

/**
 * Reopens the document the session record names, when the user lets recent files and reading
 * positions be kept, its entry kept a handle and the browser still allows reading it, and nothing
 * else opened meanwhile. Call it only when the address names no document.
 */
export async function resumeLastDocument(
  platform: Platform,
  reader: Pick<ReaderController, "getState" | "open">,
  remember = remembers(),
): Promise<void> {
  if (!remember || platform.recent.reopen === undefined) return;
  const session = (await platform.storage.get(SESSION_KEY)) as { recentId?: unknown } | undefined;
  if (typeof session?.recentId !== "string") return;
  const entry = (await platform.recent.list()).find((e) => e.id === session.recentId);
  if (entry?.hasHandle !== true) return;
  const file = await platform.recent.reopen(entry.id, { ask: false });
  // Something else opened meanwhile (a file the OS handed over, a drop): it stays.
  if (file !== undefined && reader.getState().phase === "empty") await reader.open(file);
}
