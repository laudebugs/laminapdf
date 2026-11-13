// Following a file's article threads (O13): the part the reader went to last and going to a part,
// loaded with later.ts (#380) by the Threads tab and the Next and Previous part commands, so none
// of it is in the startup script. The part belongs to one list of threads: a newly opened
// document's threads are a new list, which forgets it.

import type { ArticleThread } from "@laminapdf/sdk";
import type { ThreadPartRef } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { useSyncExternalStore } from "react";

/** The part followed last, and the threads it is a part of. */
let followed: { readonly threads: readonly ArticleThread[]; readonly part: ThreadPartRef } | undefined;
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The part of `threads` the reader followed last, if any. */
export function followedPart(threads: readonly ArticleThread[]): ThreadPartRef | undefined {
  return followed?.threads === threads ? followed.part : undefined;
}

/** `followedPart(threads)`, kept current. */
export function useFollowedPart(threads: readonly ArticleThread[]): ThreadPartRef | undefined {
  return useSyncExternalStore(subscribe, () => followedPart(threads));
}

/**
 * Goes to part `ref` of `threads` in `viewer`, as a history jump: the part's top at the top of the
 * view, keeping the zoom. A part on a missing page goes nowhere.
 */
export function goToThreadPart(viewer: Viewer, threads: readonly ArticleThread[], ref: ThreadPartRef): void {
  const part = threads[ref.thread]?.parts[ref.part];
  if (part?.page === undefined) return;
  const y = part.rect?.[1];
  viewer.jumpToDestination({ page: part.page, view: y === undefined ? { kind: "fit" } : { kind: "at", y } });
  followed = { threads, part: ref };
  for (const listener of listeners) listener();
}
