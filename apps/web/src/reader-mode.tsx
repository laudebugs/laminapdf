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
import { type ReactNode, type RefObject, useCallback, useEffect, useRef, useState } from "react";
import { pageLabel } from "./outline.ts";
import { cropClip, cropScale } from "./reader-crop.ts";
import { type OriginBox, outlineOrigin } from "./reader-origin.ts";
import { blockAt } from "./reader-start.ts";
import type { DocumentReflow, Engine } from "./session.ts";

/** The inspector content id of the reader settings panel. */
const READER_SETTINGS_PANEL = "reader-settings";

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

/** How far outside the column's visible part a crop is drawn ahead of being scrolled to. */
const CROP_MARGIN = "100% 0px";

/**
 * Whether `target` is in, or within {@link CROP_MARGIN} of, the reading column's visible part: a
 * crop is drawn only then and its pixels let go once it is far away, so a long scan, all page
 * images, doesn't draw every page up front or keep every page in memory.
 */
function useNearView(target: RefObject<HTMLElement | null>): boolean {
  const [near, setNear] = useState(typeof IntersectionObserver === "undefined");
  useEffect(() => {
    const element = target.current;
    if (element === null || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver((entries) => setNear(entries.some((e) => e.isIntersecting)), {
      root: element.closest(".paper-reader-mode"),
      rootMargin: CROP_MARGIN,
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [target]);
  return near;
}

/**
 * A crop: the block's part of its page, drawn by the document worker at twice the device's pixels
 * per point (less for a crop too big for the renderer's limits), while it is near the view.
 */
function Crop({ engine, block }: { engine: Engine; block: Block }): ReactNode {
  const canvas = useRef<HTMLCanvasElement>(null);
  const near = useNearView(canvas);
  const bbox = block.bbox;
  useEffect(() => {
    const target = canvas.current;
    if (bbox === null || target === null) return;
    if (!near) {
      // Far away: give the pixels back. The canvas keeps its size by its aspect ratio.
      target.width = 0;
      target.height = 0;
      return;
    }
    const controller = new AbortController();
    const scale = cropScale(bbox, 2 * Math.min(2, Math.max(1, window.devicePixelRatio || 1)));
    const clip = cropClip(bbox, scale, engine.getState().pages[block.page]);
    void engine.renderPart({ page: block.page, scale, clip }, controller.signal).then(
      ({ bitmap }) => {
        if (!controller.signal.aborted) {
          target.width = bitmap.width;
          target.height = bitmap.height;
          target.getContext("2d")?.drawImage(bitmap, 0, 0);
        }
        bitmap.close();
      },
      () => undefined,
    );
    return () => controller.abort();
  }, [engine, block.page, bbox, near]);
  if (bbox === null) return null;
  // Shown at the page's own size (96 CSS px per 72 pt), never wider than the column.
  return (
    <canvas ref={canvas} style={{ inlineSize: `${(bbox[2] * 4) / 3}px`, aspectRatio: `${bbox[2]} / ${bbox[3]}` }} />
  );
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
  /** Leaves at `block`'s place on its page and outlines it there: "View original page". */
  leaveAt(block: Block): void;
  /** Leaves at the top of `page`: a page image's "Open page n". */
  openPage(page: number): void;
  onTopBlock(index: number): void;
}

/** Where the page view goes once reader mode is off, and what it outlines there. */
interface JumpTarget {
  readonly page: number;
  readonly y: number;
  readonly outline?: OriginBox;
}

/**
 * Where the page view goes once reader mode is off: the column's top block's place on its page.
 * The page view stayed laid out under the column; once it is uncovered, it jumps there (a scroll
 * while it was hidden isn't reported in every browser, so its page would be wrong), and the
 * canvas takes the focus the column took with it.
 */
function useJumpBack(on: boolean, viewer: Viewer | undefined): (to: JumpTarget | undefined) => void {
  const jump = useRef<JumpTarget>(undefined);
  useEffect(() => {
    const to = jump.current;
    if (on || to === undefined || viewer === undefined) return;
    jump.current = undefined;
    viewer.goToDestination({ page: to.page, view: { kind: "at", y: to.y } });
    if (to.outline !== undefined) outlineOrigin(viewer, to.outline);
    if (document.activeElement === null || document.activeElement === document.body) {
      document.querySelector<HTMLElement>(".paper-reader-canvas")?.focus();
    }
  }, [on, viewer]);
  return useCallback((to) => {
    jump.current = to;
  }, []);
}

/** Calls `leave` when another document, or none, takes `document`'s place, and closes the settings panel. */
function useLeaveOnNewDocument(document: unknown, layout: ShellLayout, leave: () => void): void {
  const shownFor = useRef(document);
  const latest = useRef(leave);
  latest.current = leave;
  useEffect(() => {
    if (shownFor.current === document) return;
    shownFor.current = document;
    latest.current();
    if (layout.getState().inspector?.id === READER_SETTINGS_PANEL) layout.closeInspector();
  }, [document, layout]);
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
  useLeaveOnNewDocument(document, layout, () => {
    turn.current += 1;
    setOn(false);
    setContent(undefined);
  });
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
        setContent({ view: viewOf(reflow, blocks), blocks, startAt });
      },
      () => {
        if (mine === turn.current) setContent({ view: { status: "unavailable" }, blocks: [], startAt: 0 });
      },
    );
  }, [engine, viewer]);
  const jump = useJumpBack(on, viewer);
  const leaveTo = useCallback(
    (to: JumpTarget | undefined) => {
      turn.current += 1;
      setOn(false);
      if (layout.getState().inspector?.id === READER_SETTINGS_PANEL) layout.closeInspector();
      jump(to);
    },
    [layout, jump],
  );
  const leave = useCallback(() => {
    const block = content?.blocks[top.current];
    leaveTo(block?.bbox ? { page: block.page, y: block.bbox[1] } : undefined);
  }, [content, leaveTo]);
  const leaveAt = useCallback(
    (block: Block) => {
      const size = engine.getState().pages[block.page];
      if (block.bbox === null) return leaveTo({ page: block.page, y: 0 });
      const outline = size === undefined ? {} : { outline: { page: block.page, bbox: block.bbox, size } };
      leaveTo({ page: block.page, y: block.bbox[1], ...outline });
    },
    [engine, leaveTo],
  );
  const openPage = useCallback((page: number) => leaveTo({ page, y: 0 }), [leaveTo]);
  const onTopBlock = useCallback(
    (index: number) => {
      top.current = index;
      setTopPage(content?.blocks[index]?.page ?? 0);
    },
    [content],
  );
  return { on, content, topPage, enter, leave, leaveAt, openPage, onTopBlock };
}

/** The view's content for `reflow`: its blocks, and why it is approximate when it is. */
function viewOf(reflow: DocumentReflow, blocks: readonly Block[]): ReaderViewContent {
  if (!reflow.available) return { status: "unavailable" };
  const approximate =
    reflow.source === "approximate" ? (reflow.reason === "untagged" ? "untagged" : "tree") : undefined;
  return {
    status: "ready",
    blocks,
    ...(reflow.lang === undefined ? {} : { lang: reflow.lang }),
    ...(approximate === undefined ? {} : { approximate }),
  };
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
      onViewOriginal={(block: ReaderBlock) => reader.leaveAt(block as Block)}
      onOpenPage={reader.openPage}
      nameOfPage={(page: number) => pageLabel(pages, page)}
    />
  ) : undefined;
  const inspector = settingsOpen ? (
    <Inspector layout={layout}>
      <ReaderSettingsPanel settings={settings} onChange={setSettings} onClose={() => layout.closeInspector()} />
    </Inspector>
  ) : undefined;
  const status = on ? (
    <StatusBarItem key="reader-mode" onPress={leave}>
      {message(
        content?.view.status === "ready" && content.view.approximate !== undefined
          ? "reader.mode.status.approximate"
          : "reader.mode.status",
      )}
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
