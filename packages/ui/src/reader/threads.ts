// Article threads as the reader follows them (O13, ISO 32000-2 §12.4.3): each thread's parts by
// page, and the step to the next or previous part, which skips a part whose page the file lacks.
// The Threads tab's panel (threads-panel.tsx) loads later; this is what the commands need at once.

/** One article thread, as the panel shows it. */
export interface ThreadData {
  /** Its title (document text), when it has one. */
  readonly title?: string | undefined;
  /** Its parts in reading order: each part's 0-based page, `undefined` when the file lacks it. */
  readonly parts: readonly { readonly page?: number | undefined }[];
}

/** A part: the thread's and the part's index. */
export interface ThreadPartRef {
  readonly thread: number;
  readonly part: number;
}

/**
 * The part after (`1`) or before (`-1`) `from` in its thread, skipping parts whose page is
 * missing; with no part shown yet, the first part of the first thread that has one.
 */
export function stepThreadPart(
  threads: readonly ThreadData[],
  from: ThreadPartRef | undefined,
  direction: 1 | -1,
): ThreadPartRef | undefined {
  if (from === undefined) {
    if (direction === -1) return undefined;
    for (const [thread, t] of threads.entries()) {
      const part = t.parts.findIndex((p) => p.page !== undefined);
      if (part >= 0) return { thread, part };
    }
    return undefined;
  }
  const parts = threads[from.thread]?.parts ?? [];
  for (let part = from.part + direction; part >= 0 && part < parts.length; part += direction) {
    if (parts[part]?.page !== undefined) return { thread: from.thread, part };
  }
  return undefined;
}
