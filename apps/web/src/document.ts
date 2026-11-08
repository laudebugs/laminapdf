// What the reader shows about the open document, worked out from what the viewer reports: the
// window's title, and the repairs listed in the repair details panel. Pure, so Node tests them.

import type { RepairKind } from "@laminapdf/ui";
import { hasKey, parseFragment } from "./open/url-fragment.ts";

/** The app's name, which ends every window title. */
export const APP_NAME = "Lamina PDF";

/** A warning as the document worker reports it (an ADR 0026 warning object). */
export interface DocumentWarning {
  readonly code: string;
  /** Where the problem is; a warning about one page names its 0-based index. */
  readonly location?: { readonly pageIndex?: number };
}

/**
 * The window's title: the document's metadata title, else its file name, then the app's name.
 * Only the app's name while no document is open.
 */
export function windowTitle(document: { readonly title?: string; readonly fileName: string } | undefined): string {
  if (document === undefined) return APP_NAME;
  const name = document.title?.trim() || document.fileName;
  return `${name} · ${APP_NAME}`;
}

/**
 * The repair details panel's entries: one per warning code, in the order the codes first came,
 * each with the 1-based pages its warnings name, sorted and without repeats.
 */
export function repairsFrom(warnings: readonly DocumentWarning[]): RepairKind[] {
  const pages = new Map<string, Set<number>>();
  for (const { code, location } of warnings) {
    const seen = pages.get(code) ?? new Set<number>();
    if (location?.pageIndex !== undefined) seen.add(location.pageIndex + 1);
    pages.set(code, seen);
  }
  return [...pages].map(([code, set]) => (set.size === 0 ? { code } : { code, pages: [...set].sort((a, b) => a - b) }));
}

/** The repairs as plain text, for the panel's Copy details: one code per line, with its pages. */
export function repairsText(repairs: readonly RepairKind[]): string {
  return repairs
    .map(({ code, pages }) => (pages === undefined ? code : `${code} (pages ${pages.join(", ")})`))
    .join("\n");
}

/**
 * Whether the address's fragment asks for the debug menu (ADR 0011 §3.6: a fragment, never sent):
 * a `debug` key, alone (`#debug`) or beside others (`#url=…&debug`).
 */
export function wantsDebug(hash: string): boolean {
  return hasKey(parseFragment(hash), "debug");
}
