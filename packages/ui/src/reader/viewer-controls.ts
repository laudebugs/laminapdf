// What the reader chrome needs of `@laminapdf/viewer`'s viewer, written as a shape rather than an
// import (the package never imports the viewer): its state as a store, and the three calls the
// top bar makes. The zoom steps and page parsing are pure, so their rules are tested in Node.

import type { ExternalStore } from "../state/hooks.js";

/** A fit mode or a percentage, as the viewer takes it. */
export type Zoom = "fit-width" | "fit-page" | "actual-size" | number;

/** The part of the viewer's state the chrome shows. */
export interface ViewerSlice {
  readonly pageCount: number;
  /** The page under the viewport's centre, 0-based. */
  readonly currentPage: number;
  readonly zoom: Zoom;
  /** The zoom as a percentage, whatever the mode. */
  readonly percent: number;
}

/** The viewer as the chrome drives it: `createViewer()`'s result fits. */
export interface ViewerControls extends ExternalStore<ViewerSlice> {
  /** Scrolls to page `index`, 0-based. */
  goToPage(index: number): void;
  /** Sets the zoom: a percentage, or a fit mode. */
  setZoom(zoom: Zoom): void;
  /** Rotates the view by a multiple of 90 degrees. */
  rotate(delta: number): void;
}

/** The smallest zoom the viewer takes, in percent. */
export const MIN_ZOOM = 10;
/** The largest zoom the viewer takes, in percent. */
export const MAX_ZOOM = 800;

/** The percentages zoom out and zoom in step through. */
const STEPS = [10, 25, 33, 50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400, 500, 600, 800];
/** The zoom menu's presets, in percent. */
export const ZOOM_PRESETS = [50, 75, 100, 125, 150, 200, 400] as const;

/** The next step above `percent` (a fit mode's percentage need not be a step). */
export function zoomIn(percent: number): number {
  return STEPS.find((step) => step > percent + 0.5) ?? MAX_ZOOM;
}

/** The next step below `percent`. */
export function zoomOut(percent: number): number {
  return [...STEPS].reverse().find((step) => step < percent - 0.5) ?? MIN_ZOOM;
}

/**
 * The 0-based page that `text` names: a page label when the document has `labels` (one per page,
 * matched without regard to case), else a page number from 1 to `pageCount`. Undefined when it
 * names no page.
 */
export function pageFromInput(text: string, pageCount: number, labels?: readonly string[]): number | undefined {
  const typed = text.trim();
  if (typed === "") return undefined;
  const labelled = labels?.findIndex((label) => label.toLocaleLowerCase() === typed.toLocaleLowerCase()) ?? -1;
  if (labelled >= 0 && labelled < pageCount) return labelled;
  if (!/^\d+$/.test(typed)) return undefined;
  const number = Number(typed);
  return number >= 1 && number <= pageCount ? number - 1 : undefined;
}
