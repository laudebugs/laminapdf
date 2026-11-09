// Reader mode (R14) in the web reader: the Reader toggle swaps the page view for the reading
// column of the open document's reflow (read once per document by the worker), keeps the place
// both ways, draws figures, formulas and tables as crops of their page, and keeps the reader's
// settings (text size, line spacing, line length) in platform storage, for every document.

import {
  DEFAULT_READER_SETTINGS,
  Inspector,
  message,
  type Platform,
  type ReaderBlock,
  type ReaderSettings,
  ReaderSettingsPanel,
  ReaderView,
  type ReaderViewContent,
  readerTextSize,
  type ShellLayout,
  StatusBarItem,
  useStoreState,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { pageLabel } from "./outline.ts";
import type { DocumentReflow, Engine } from "./session.ts";

/** The inspector content id of the reader settings panel. */
export const READER_SETTINGS_PANEL = "reader-settings";

/** Where the settings are kept. */
const SETTINGS_KEY = "reader-settings";

/** One block of the open document's reflow. */
type Block = DocumentReflow["blocks"][number];

/** The settings stored under `SETTINGS_KEY`, or the defaults where they are missing or wrong. */
function settingsFrom(value: unknown): ReaderSettings {
  const v = (typeof value === "object" && value !== null ? value : {}) as Partial<
    Record<keyof ReaderSettings, unknown>
  >;
  return {
    textSize: typeof v.textSize === "number" ? readerTextSize(v.textSize) : DEFAULT_READER_SETTINGS.textSize,
    lineSpacing:
      v.lineSpacing === "tight" || v.lineSpacing === "loose" || v.lineSpacing === "normal"
        ? v.lineSpacing
        : DEFAULT_READER_SETTINGS.lineSpacing,
    lineLength:
      v.lineLength === "narrow" || v.lineLength === "wide" || v.lineLength === "medium"
        ? v.lineLength
        : DEFAULT_READER_SETTINGS.lineLength,
  };
}

/** The reader's settings, loaded from and saved to platform storage. */
function useReaderSettings(platform: Platform): [ReaderSettings, (next: ReaderSettings) => void] {
  const [settings, setSettings] = useState(DEFAULT_READER_SETTINGS);
  // A change made before the stored settings arrive wins over them.
  const changed = useRef(false);
  useEffect(() => {
    let live = true;
    void platform.storage.get(SETTINGS_KEY).then(
      (value) => {
        if (live && !changed.current) setSettings(settingsFrom(value));
      },
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [platform]);
  const change = useCallback(
    (next: ReaderSettings) => {
      changed.current = true;
      setSettings(next);
      void platform.storage.set(SETTINGS_KEY, next).catch(() => undefined);
    },
    [platform],
  );
  return [settings, change];
}

/** The first block at or after `point` (page space) in reading order: where the column starts. */
export function blockAt(blocks: readonly Block[], point: { page: number; y: number } | undefined): number {
  if (point === undefined || blocks.length === 0) return 0;
  const index = blocks.findIndex(
    // A crop with no box has no place on its page: it starts the column only from a later page.
    (b) => b.page > point.page || (b.page === point.page && b.bbox !== null && b.bbox[1] + b.bbox[3] > point.y),
  );
  return index === -1 ? blocks.length - 1 : index;
}

/** A crop: the block's part of its page, drawn by the document worker at twice the device's pixels per point. */
function Crop({ engine, block }: { engine: Engine; block: Block }): ReactNode {
  const canvas = useRef<HTMLCanvasElement>(null);
  const bbox = block.bbox;
  useEffect(() => {
    if (bbox === null) return;
    const controller = new AbortController();
    const scale = 2 * Math.min(2, Math.max(1, window.devicePixelRatio || 1));
    const [x, y, width, height] = bbox;
    const clip = {
      x: Math.floor(x * scale),
      y: Math.floor(y * scale),
      width: Math.max(1, Math.ceil(width * scale)),
      height: Math.max(1, Math.ceil(height * scale)),
    };
    void engine.renderPart({ page: block.page, scale, clip }, controller.signal).then(
      ({ bitmap }) => {
        const target = canvas.current;
        if (target !== null && !controller.signal.aborted) {
          target.width = bitmap.width;
          target.height = bitmap.height;
          target.getContext("2d")?.drawImage(bitmap, 0, 0);
        }
        bitmap.close();
      },
      () => undefined,
    );
    return () => controller.abort();
  }, [engine, block.page, bbox]);
  if (bbox === null) return null;
  // Shown at the page's own size (96 CSS px per 72 pt), never wider than the column.
  return <canvas ref={canvas} style={{ inlineSize: `${(bbox[2] * 4) / 3}px` }} />;
}

/** What reader mode adds to the reader. */
export interface ReaderMode {
  /** Whether the reading column replaces the page view. */
  readonly on: boolean;
  /** The column, while on. */
  readonly view: ReactNode;
  /** The settings panel, while it is the inspector's content. */
  readonly inspector: ReactNode;
  /** The status bar's "Reader mode · from tags", while on. */
  readonly status: ReactNode;
  /** The top bar's state. */
  readonly topBar: { readonly readerMode: boolean; readonly readerSettingsOpen: boolean };
  /** The handlers of reader mode's commands. */
  readonly actions: Readonly<Record<string, (args: unknown) => void>>;
}

/** The column's state: on or off, its content, and the block at its top. */
interface ReaderContent {
  readonly on: boolean;
  readonly content: { view: ReaderViewContent; blocks: readonly Block[]; startAt: number } | undefined;
  readonly topPage: number;
  enter(): void;
  leave(): void;
  onTopBlock(index: number): void;
}

/**
 * Where the page view goes once reader mode is off: the column's top block's place on its page.
 * The page view stayed laid out under the column; once it is uncovered, it jumps there (a scroll
 * while it was hidden isn't reported in every browser, so its page would be wrong), and the
 * canvas takes the focus the column took with it.
 */
function useJumpBack(on: boolean, viewer: Viewer | undefined): (to: { page: number; y: number } | undefined) => void {
  const jump = useRef<{ page: number; y: number }>(undefined);
  useEffect(() => {
    const to = jump.current;
    if (on || to === undefined || viewer === undefined) return;
    jump.current = undefined;
    viewer.goToDestination({ page: to.page, view: { kind: "at", y: to.y } });
    if (document.activeElement === null || document.activeElement === document.body) {
      document.querySelector<HTMLElement>(".paper-reader-canvas")?.focus();
    }
  }, [on, viewer]);
  return useCallback((to) => {
    jump.current = to;
  }, []);
}

/**
 * Entering reads the open document's reflow and starts the column at the block at the view's
 * top; leaving brings that block's place on its page back to the view's top. A new document
 * (`document`), or none, leaves reader mode.
 */
function useReaderContent(
  engine: Engine,
  layout: ShellLayout,
  viewer: Viewer | undefined,
  document: unknown,
): ReaderContent {
  const [on, setOn] = useState(false);
  const [content, setContent] = useState<ReaderContent["content"]>();
  const top = useRef(0);
  const [topPage, setTopPage] = useState(0);
  // Counts entries and documents: a reflow that arrives for an older one is dropped.
  const turn = useRef(0);
  // Another document, or none, leaves reader mode and its settings panel.
  const shownFor = useRef(document);
  useEffect(() => {
    if (shownFor.current === document) return;
    shownFor.current = document;
    turn.current += 1;
    setOn(false);
    setContent(undefined);
    if (layout.getState().inspector?.id === READER_SETTINGS_PANEL) layout.closeInspector();
  }, [document, layout]);
  const enter = useCallback(() => {
    turn.current += 1;
    const mine = turn.current;
    // The view's top-left, its height on the page (a share of it) in points.
    const at = viewer?.pointAt(16, 16);
    const height = at === undefined ? undefined : engine.getState().pages[at.page]?.height;
    const point = at === undefined || height === undefined ? undefined : { page: at.page, y: at.y * height };
    setOn(true);
    setContent(undefined);
    void engine.reflow().then(
      (reflow) => {
        if (mine !== turn.current) return;
        const blocks = reflow.available ? reflow.blocks : [];
        const startAt = blockAt(blocks, point);
        top.current = startAt;
        setTopPage(blocks[startAt]?.page ?? point?.page ?? 0);
        const view: ReaderViewContent = reflow.available
          ? { status: "ready", blocks, ...(reflow.lang === undefined ? {} : { lang: reflow.lang }) }
          : { status: "unavailable", untagged: reflow.reason === "untagged" };
        setContent({ view, blocks, startAt });
      },
      () => {
        if (mine === turn.current)
          setContent({ view: { status: "unavailable", untagged: false }, blocks: [], startAt: 0 });
      },
    );
  }, [engine, viewer]);
  const jump = useJumpBack(on, viewer);
  const leave = useCallback(() => {
    turn.current += 1;
    setOn(false);
    if (layout.getState().inspector?.id === READER_SETTINGS_PANEL) layout.closeInspector();
    const block = content?.blocks[top.current];
    jump(block?.bbox ? { page: block.page, y: block.bbox[1] } : undefined);
  }, [content, layout, jump]);
  const onTopBlock = useCallback(
    (index: number) => {
      top.current = index;
      setTopPage(content?.blocks[index]?.page ?? 0);
    },
    [content],
  );
  return { on, content, topPage, enter, leave, onTopBlock };
}

/** The handlers of reader mode's commands. */
function readerActions(
  reader: ReaderContent,
  layout: ShellLayout,
  settings: ReaderSettings,
  setSettings: (next: ReaderSettings) => void,
): ReaderMode["actions"] {
  const set = (patch: Partial<ReaderSettings>) => setSettings({ ...settings, ...patch });
  const toggle = { "view.reader-mode": () => (reader.on ? reader.leave() : reader.enter()) };
  // The settings' commands have handlers, so the palette offers them, only while reader mode is on.
  if (!reader.on) return toggle;
  return {
    ...toggle,
    "view.reader-settings": () =>
      layout.getState().inspector?.id === READER_SETTINGS_PANEL
        ? layout.closeInspector()
        : layout.openInspector({ id: READER_SETTINGS_PANEL, selection: false }),
    "view.reader-text-smaller": () => set({ textSize: readerTextSize(settings.textSize - 1) }),
    "view.reader-text-larger": () => set({ textSize: readerTextSize(settings.textSize + 1) }),
    "view.reader-line-spacing": (value) => set({ lineSpacing: value as ReaderSettings["lineSpacing"] }),
    "view.reader-line-length": (value) => set({ lineLength: value as ReaderSettings["lineLength"] }),
    "view.reader-reset": () => setSettings(DEFAULT_READER_SETTINGS),
  };
}

/** Reader mode over `engine`'s open `document`, shown by `viewer` when it is off. */
export function useReaderMode(
  engine: Engine,
  layout: ShellLayout,
  platform: Platform,
  viewer: Viewer | undefined,
  document: unknown,
): ReaderMode {
  const pages = useStoreState(engine, (s) => s.pages);
  const reader = useReaderContent(engine, layout, viewer, document);
  const [settings, setSettings] = useReaderSettings(platform);
  const settingsOpen = useStoreState(layout, (s) => s.inspector?.id === READER_SETTINGS_PANEL);
  const { on, content, leave } = reader;
  const view = on ? (
    <ReaderView
      content={content?.view ?? { status: "loading" }}
      settings={settings}
      pageName={pageLabel(pages, reader.topPage)}
      onBack={leave}
      renderCrop={(block: ReaderBlock) => <Crop engine={engine} block={block as Block} />}
      {...(content === undefined ? {} : { startAt: content.startAt })}
      onTopBlock={reader.onTopBlock}
    />
  ) : undefined;
  const inspector = settingsOpen ? (
    <Inspector layout={layout}>
      <ReaderSettingsPanel settings={settings} onChange={setSettings} onClose={() => layout.closeInspector()} />
    </Inspector>
  ) : undefined;
  const status = on ? (
    <StatusBarItem key="reader-mode" onPress={leave}>
      {message("reader.mode.status")}
    </StatusBarItem>
  ) : undefined;
  return {
    on,
    view,
    inspector,
    status,
    topBar: { readerMode: on, readerSettingsOpen: settingsOpen },
    actions: readerActions(reader, layout, settings, setSettings),
  };
}
