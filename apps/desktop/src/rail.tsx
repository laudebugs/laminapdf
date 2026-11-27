// The desktop's sidebar (D9d): the Pages and Outline tabs `@laminapdf/ui` builds for both readers,
// over this window's engine. The pages are the viewer's thumbnails strip, drawn from the engine's
// small renders. The outline is read from the engine the first time its tab shows, once per shown
// document, and its items go to their page through the viewer. `view.toggle-sidebar` opens and
// closes it while a document shows; the layout store keeps its width and state (ShellLayout).

import {
  LeftRail,
  type OutlineSource,
  type OutlineState,
  outlineModel,
  type RailTab,
  readerRailTabs,
  type ShellLayout,
} from "@laminapdf/ui";
import type { ThumbnailStrip, Viewer } from "@laminapdf/viewer";
import { type ReactNode, useMemo } from "react";
import type { DesktopEngineClient } from "./engine/desktop-engine-client.ts";

/** Where an outline item goes in the viewer. */
type Destination = Parameters<Viewer["goToDestination"]>[0];

/** The outline's state: what the Outline tab reads. */
interface DesktopOutlineState {
  readonly outline: OutlineState<Destination>;
}

/**
 * The shown document's outline, read when the Outline tab asks for it. One per shown document: the
 * window makes a new one when another document shows, so a read that comes back late for the
 * document before it is dropped.
 */
export class DesktopOutline implements OutlineSource<Destination> {
  readonly #client: Pick<DesktopEngineClient, "outline" | "document">;
  readonly #viewer: Pick<Viewer, "jumpToDestination">;
  readonly #listeners = new Set<() => void>();
  #state: DesktopOutlineState = { outline: undefined };

  constructor(client: Pick<DesktopEngineClient, "outline" | "document">, viewer: Pick<Viewer, "jumpToDestination">) {
    this.#client = client;
    this.#viewer = viewer;
  }

  getState(): DesktopOutlineState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => void this.#listeners.delete(listener);
  }

  async loadOutline(): Promise<void> {
    if (this.#state.outline !== undefined) return;
    this.#set({ outline: "loading" });
    try {
      const nodes = await this.#client.outline();
      this.#set({ outline: outlineModel(nodes, this.#client.document?.pages ?? []) });
    } catch {
      // The tab says the outline couldn't be read, rather than that there is none.
      this.#set({ outline: "failed" });
    }
  }

  goToOutlineItem(id: string): Promise<void> {
    const outline = this.#state.outline;
    const target = typeof outline === "object" ? outline.targets.get(id) : undefined;
    if (target !== undefined) this.#viewer.jumpToDestination(target);
    return Promise.resolve();
  }

  #set(state: DesktopOutlineState): void {
    this.#state = state;
    for (const listener of [...this.#listeners]) listener();
  }
}

/** What the window gives the sidebar. */
export interface DesktopRailParts {
  readonly client: DesktopEngineClient;
  readonly viewer: Viewer;
  readonly strip: ThumbnailStrip;
}

/**
 * The window's sidebar: its tabs and the rail that shows them, while a document shows (`shown`);
 * none otherwise. A new outline reader serves each shown document.
 */
export function useDesktopRail(
  engine: DesktopRailParts | undefined,
  layout: ShellLayout,
  shown: boolean,
): { readonly rail: ReactNode; readonly tabs: readonly RailTab[] } {
  const doc = shown ? engine?.client.current?.doc : undefined;
  const source = useMemo(
    () => (engine === undefined || doc === undefined ? undefined : new DesktopOutline(engine.client, engine.viewer)),
    [engine, doc],
  );
  const tabs = useMemo(
    () =>
      engine === undefined || source === undefined
        ? []
        : readerRailTabs({ source, layout, strip: engine.strip, viewer: engine.viewer }),
    [engine, source, layout],
  );
  return { tabs, rail: tabs.length === 0 ? undefined : <LeftRail layout={layout} tabs={tabs} /> };
}
