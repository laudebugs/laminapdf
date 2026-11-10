// Where the reader stopped in each document (T5b): kept on this device under the document's
// fingerprint (`Platform.positions`), never under its name or address, and put back when the same
// file opens again, however it came in. The view is saved a second after it stops changing, when
// the page is hidden and when the document closes; a link that names a page or destination wins
// over it. "Remember where I stopped reading" (Settings › Storage and privacy) turns it off.

import type { Platform, ReadingPosition } from "@laminapdf/ui";
import type { Viewer, ViewPosition } from "@laminapdf/viewer";

/** How long the view must stay still before its position is saved. */
export const SAVE_AFTER_MS = 1000;

/** The storage key of the document open when the app last closed: `{ recentId }`. */
export const SESSION_KEY = "session";

/** What a position records the view with: the viewer's own position type. */
export type KeptView = ViewPosition;

const MODES = new Set(["fit-width", "fit-page", "actual-size"]);

/**
 * `view` as a position the viewer can restore, or undefined when it isn't one: a stored value is
 * checked on read, since the store keeps it opaque and an older version may have written it.
 */
export function parseView(view: unknown): KeptView | undefined {
  if (typeof view !== "object" || view === null) return undefined;
  const { point, zoom, rotation } = view as Partial<Record<keyof KeptView, unknown>>;
  if (typeof point !== "object" || point === null) return undefined;
  const { page, x, y } = point as Partial<Record<"page" | "x" | "y", unknown>>;
  // A point beside the page (the viewport's edge left of a page narrower than it) is a place on
  // the page's edge: clamped, not refused.
  const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
  if (typeof page !== "number" || !Number.isInteger(page) || page < 0 || !finite(x) || !finite(y)) return undefined;
  const zoomOk =
    (typeof zoom === "number" && zoom >= 10 && zoom <= 800) || (typeof zoom === "string" && MODES.has(zoom));
  if (!zoomOk || (rotation !== 0 && rotation !== 90 && rotation !== 180 && rotation !== 270)) return undefined;
  const clamp = (v: number) => Math.min(1, Math.max(0, v));
  return { point: { page, x: clamp(x), y: clamp(y) }, zoom: zoom as KeptView["zoom"], rotation };
}

/**
 * How long opening a document waits for its kept position. Past it the document opens at page 1:
 * a slow or stuck store (a browser's IndexedDB still starting) never holds the document back.
 */
export const KEPT_WITHIN_MS = 250;

/**
 * Whether `view` is where every document opens, the top of page 1: coming back there is no
 * resume, so it is neither announced nor made a history jump. The zoom alone isn't restored.
 */
function atStart(view: KeptView): boolean {
  return view.point.page === 0 && view.point.y <= START_SLACK;
}

/** How far down page 1 (a share of its height) still counts as its top. */
const START_SLACK = 0.02;

/**
 * The position kept for `fingerprint`, when it is one this file can show, away from the top of
 * page 1, and the store answers within {@link KEPT_WITHIN_MS}.
 */
export async function keptPosition(platform: Platform, fingerprint: string): Promise<KeptView | undefined> {
  let stop = () => {};
  const late = new Promise<undefined>((resolve) => {
    const timer = setTimeout(resolve, KEPT_WITHIN_MS);
    stop = () => clearTimeout(timer);
  });
  try {
    const entry = await Promise.race([platform.positions.get(fingerprint).catch(() => undefined), late]);
    const view = parseView(entry?.view);
    return view === undefined || atStart(view) ? undefined : view;
  } finally {
    stop();
  }
}

/** The page a kept entry stopped at (0-based) and its page count, for the home screen. */
export function stoppedAt(entry: ReadingPosition | undefined): { page: number; pageCount: number } | undefined {
  const view = parseView(entry?.view);
  if (entry === undefined || view === undefined) return undefined;
  return { page: Math.min(view.point.page, Math.max(0, entry.pageCount - 1)), pageCount: entry.pageCount };
}

/** What the recorder needs of the page: its visibility, and its events. */
export interface PageVisibility {
  readonly visibilityState: DocumentVisibilityState;
  addEventListener(type: "visibilitychange", listener: () => void): void;
  removeEventListener(type: "visibilitychange", listener: () => void): void;
}

/** A running recorder: `flush` saves now and goes on; `stop` saves a last time and stops. */
export interface PositionRecorder {
  flush(): void;
  stop(): void;
}

/**
 * Saves `viewer`'s position with `save` {@link SAVE_AFTER_MS} after it stops changing, at once
 * when `page` is hidden (a write on close may not finish), on `flush`, and on `stop`, which ends
 * recording: call it before the viewer shows another document.
 */
export function recordPosition(
  viewer: Pick<Viewer, "subscribe" | "viewPosition">,
  save: (view: KeptView) => void,
  page: PageVisibility = document,
): PositionRecorder {
  let last = viewer.viewPosition();
  let timer: ReturnType<typeof setTimeout> | undefined;
  const flush = () => {
    clearTimeout(timer);
    timer = undefined;
    if (last !== undefined) save(last);
  };
  const unsubscribe = viewer.subscribe(() => {
    last = viewer.viewPosition() ?? last;
    clearTimeout(timer);
    timer = setTimeout(flush, SAVE_AFTER_MS);
  });
  const hidden = () => {
    if (page.visibilityState === "hidden") flush();
  };
  page.addEventListener("visibilitychange", hidden);
  return {
    flush,
    stop() {
      unsubscribe();
      page.removeEventListener("visibilitychange", hidden);
      flush();
    },
  };
}
