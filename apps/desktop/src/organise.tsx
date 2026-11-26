// The Organise mode in the desktop window (Paper O06): @laminapdf/ui's mode, the web reader's
// too, over this window's engine. The engine reports the pages (sizes, rotations and IDs, as each
// edit leaves them) and the label ranges, so the grid's labels are worked out from the ranges here
// and read again after every edit. Each edit is asked of the engine's save planner first, so a
// signed page and a certified file are asked about before anything changes. Insert pages from a file
// and Replace pages are here (documents.tsx); merging, writing parts to a folder and Save a copy on
// the certification's banner aren't on the desktop yet.

import {
  type CommandRegistry,
  type ExternalStore,
  type KeyPlatform,
  type ModeRegistry,
  type Platform,
  shellActions,
  type ToastQueue,
  useShell,
  useStoreState,
} from "@laminapdf/ui";
import {
  ModeArea,
  OrganiseController,
  type OrganiseEngine,
  OrganiseMode,
  type OrganisePage,
  organiseActions,
  pageLabels,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo } from "react";
import { DesktopDocumentsController, DesktopDocumentsDialogs, documentsActions } from "./documents.tsx";
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
    // A failed read keeps the labels the pages had: the document closed or its engine restarted
    // mid-read, which the window already says, and the next edit or document reads them again.
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

/** The Organise mode over the shown document: its pages for the grid, and its controller. */
export interface DesktopOrganise {
  readonly pages: DesktopOrganiseEngine;
  readonly controller: OrganiseController;
  /** Insert pages from a file and Replace pages. */
  readonly documents: DesktopDocumentsController;
}

/**
 * The window's modes (Read, and Organise while a document shows), its shell layout, and the
 * Organise mode's controller for the shown document: what the window passes to the shell, the top
 * bar and the mode, plus the shell's focus-mode and mode-switch commands and the page commands
 * while Organise is the mode.
 */
export function useOrganising(
  platform: Platform,
  engine: { readonly client: DesktopEngineClient; readonly viewer: Viewer } | undefined,
  shown: boolean,
  toasts: ToastQueue,
) {
  const { layout, modes } = useShell(platform.storage);
  const doc = shown ? engine?.client.current?.doc : undefined;
  const organise = useMemo((): DesktopOrganise | undefined => {
    if (engine === undefined || doc === undefined) return undefined;
    const pages = new DesktopOrganiseEngine(engine.client);
    const controller = new OrganiseController(pages, engine.viewer, toasts);
    return { pages, controller, documents: new DesktopDocumentsController(engine.client, controller) };
  }, [engine, doc, toasts]);
  useEffect(() => organise?.pages.follow(), [organise]);
  const modeList = useStoreState(modes, (s) => s.modes);
  const organising = useStoreState(modes, (s) => s.current === "organise");
  const actions = useMemo(
    () => ({
      // Focus mode and mode switches as modes join (`modeList` rebuilds `actions` under React Compiler).
      ...shellActions(layout, modes, { rail: false, list: modeList }),
      ...(organising && organise !== undefined
        ? { ...organiseActions(organise.controller), ...documentsActions(organise.documents) }
        : {}),
    }),
    [layout, modes, modeList, organising, organise],
  );
  return {
    actions,
    shell: { layout, modes },
    /** The top bar's mode switcher, while a document shows. */
    topBar: organise === undefined ? {} : { modeArea: <ModeArea modes={modes} /> },
    mode: { organise, modes },
  };
}

/** The mode itself, once the commands' registry exists: registered while a document shows. */
export function DesktopOrganiseMode({
  organise,
  viewer,
  modes,
  registry,
  keys,
}: {
  readonly organise: DesktopOrganise | undefined;
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
      dialogs={
        <DesktopDocumentsDialogs documents={organise.documents} organise={organise.controller} pages={organise.pages} />
      }
      viewer={viewer}
      modes={modes}
      registry={registry}
      keys={keys}
    />
  );
}
