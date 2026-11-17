// The Organise mode in the desktop window (Paper O06, E2n): @laminapdf/ui's mode, the web reader's
// too, over this window's engine. The engine reports the pages (sizes, rotations and IDs, as each
// edit leaves them) and the label ranges, so the grid's labels are worked out from the ranges here
// and read again after every edit. Each edit is asked of the engine's save planner first, so a
// signed page and a certified file are asked about before anything changes. Pages from other files,
// merging and writing parts to a folder aren't on the desktop yet, nor is Save a copy on the
// certification's banner.

import type { CommandRegistry, ExternalStore, KeyPlatform, ModeRegistry, ToastQueue } from "@laminapdf/ui";
import {
  OrganiseController,
  type OrganiseEngine,
  OrganiseMode,
  type OrganisePage,
  organiseActions,
  pageLabels,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useSyncExternalStore } from "react";
import type { DesktopEngineClient } from "./engine/desktop-engine-client.ts";

/** The shown document's pages, with their labels, for the grid. */
interface PagesState {
  readonly pages: readonly OrganisePage[];
}

/** This window's engine as the Organise mode reads it. */
export class DesktopOrganiseEngine implements OrganiseEngine, ExternalStore<PagesState> {
  readonly #client: DesktopEngineClient;
  #state: PagesState = { pages: [] };
  readonly #listeners = new Set<() => void>();
  /** The newest read of the labels: an older one that answers late is dropped. */
  #read = 0;

  constructor(client: DesktopEngineClient) {
    this.#client = client;
  }

  getState(): PagesState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Follows the shown document's edits until the returned function is called. */
  follow(): () => void {
    void this.refresh();
    return this.#client.on("edit", () => void this.refresh());
  }

  /** Reads the pages again, then their labels from the label ranges. */
  async refresh(): Promise<void> {
    const read = ++this.#read;
    const pages = this.#client.document?.pages ?? [];
    const ids = pages.map((p) => p.id);
    // The pages show at once with the labels they had; the new labels follow.
    const before = new Map(this.#state.pages.map((p) => [p.id, p.label]));
    this.#set(
      pages.map((p) => ({ id: p.id, width: p.width, height: p.height, rotation: p.rotation, label: before.get(p.id) })),
    );
    const ranges = await this.#client.labelRanges().catch(() => undefined);
    if (read !== this.#read || ranges === undefined) return;
    const labels = pageLabels(ranges, ids);
    this.#set(this.#state.pages.map((p, i) => ({ ...p, label: labels[i] })));
  }

  wouldViolate(ops: readonly object[]): ReturnType<OrganiseEngine["wouldViolate"]> {
    return this.#client.wouldViolate(ops as Parameters<DesktopEngineClient["wouldViolate"]>[0]);
  }

  pageBoxes(index: number): ReturnType<OrganiseEngine["pageBoxes"]> {
    return this.#client.pageBoxes(index);
  }

  labelRanges(): ReturnType<OrganiseEngine["labelRanges"]> {
    return this.#client.labelRanges();
  }

  thumbnail(index: number, scale: number, signal: AbortSignal): ReturnType<OrganiseEngine["thumbnail"]> {
    return this.#client.render({ page: index, scale }, { priority: "background", signal });
  }

  #set(pages: readonly OrganisePage[]): void {
    this.#state = { pages };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** The Organise mode's controller for the shown document, and its page commands while it is the mode. */
export function useDesktopOrganise(
  engine: { readonly client: DesktopEngineClient; readonly viewer: Viewer } | undefined,
  doc: number | undefined,
  toasts: ToastQueue,
  modes: ModeRegistry,
) {
  const organise = useMemo(() => {
    if (engine === undefined || doc === undefined) return undefined;
    const pages = new DesktopOrganiseEngine(engine.client);
    return { pages, controller: new OrganiseController(pages, engine.viewer, toasts) };
  }, [engine, doc, toasts]);
  useEffect(() => organise?.pages.follow(), [organise]);
  const organising = useSyncExternalStore(
    (listener) => modes.subscribe(listener),
    () => modes.getState().current === "organise",
  );
  const actions = useMemo(
    () => (organising && organise !== undefined ? organiseActions(organise.controller) : {}),
    [organising, organise],
  );
  return { organise, actions };
}

/** The mode itself, once the commands' registry exists: registered while a document shows. */
export function DesktopOrganiseMode({
  organise,
  viewer,
  modes,
  registry,
  keys,
}: {
  readonly organise: ReturnType<typeof useDesktopOrganise>["organise"];
  readonly viewer: Viewer | undefined;
  readonly modes: ModeRegistry;
  readonly registry: CommandRegistry;
  readonly keys: KeyPlatform;
}): ReactNode {
  if (organise === undefined || viewer === undefined) return null;
  return (
    <OrganiseMode
      controller={organise.controller}
      engine={organise.pages}
      viewer={viewer}
      modes={modes}
      registry={registry}
      keys={keys}
    />
  );
}
