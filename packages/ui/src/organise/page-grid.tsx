// The Organise mode's page grid (Paper O06): every page as a tile, in a multi-select listbox laid
// out as a grid (a reflowing grid has no fixed rows, so the ARIA grid pattern doesn't fit; O06's
// recommendation, confirmed here). React Aria gives the listbox its selection (click selects, ⇧
// extends, ⌘/Ctrl toggles, ⌘A selects all), its arrow keys in two dimensions, and drag and drop
// with an insertion bar between tiles, auto-scroll and keyboard dragging. The grid adds O06's keys:
// ⇧⌘←/→ and ⇧⌘↑/↓ (Ctrl Shift elsewhere) move the selection a page or a row, R and ⇧R turn it,
// Delete and Backspace delete it, Enter opens the focused page in Read mode, and Shift+F10 or the
// context-menu key opens the page menu. Every move is one `onMove` with a gap (moves.ts). The app
// draws each tile's picture; the grid asks for it when the tile comes near the view, and again when
// its page's `drawn` key changes.

import { Icon } from "@laminapdf/icons/react";
import { COMPONENT, SPACE } from "@laminapdf/tokens";
import { type KeyboardEvent, type ReactNode, useEffect, useRef, useState } from "react";
import {
  DropIndicator,
  GridLayout,
  ListBox,
  ListBoxItem,
  type Selection,
  Size,
  useDragAndDrop,
  Virtualizer,
} from "react-aria-components";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { laterMessage } from "../later-messages.js";
import { AnchoredMenu, type MenuEntry } from "../overlays/popover.js";
import { stepGap } from "./moves.js";

/** One page of the grid. */
export interface GridPage {
  readonly id: string;
  /** Its label, or its number when it has none ("A-1", "5"). */
  readonly label: string;
  /** Its displayed size in points (its rotation applied): the tile takes its shape. */
  readonly width: number;
  readonly height: number;
  /** Changes when the page draws differently (an edit turned it): the tile draws again. */
  readonly drawn: string;
}

/** What the grid's keys and its menu ask the app to do with the selection. */
export type GridCommand = "rotate-left" | "rotate-right" | "delete";

/** Props of `PageGrid`. */
export interface PageGridProps<Id extends string> {
  readonly "aria-label": string;
  readonly pages: readonly GridPage[];
  /** The selected pages' IDs. */
  readonly selected: ReadonlySet<string>;
  readonly onSelectionChange: (ids: Set<string>) => void;
  /** Draws page `index`'s picture into `canvas`, sized to the tile in device pixels. */
  readonly draw: (id: string, canvas: HTMLCanvasElement, signal: AbortSignal) => Promise<void>;
  /** Moves the selected pages (indices, in order) to gap `gap` (moves.ts). */
  readonly onMove: (pages: readonly number[], gap: number) => void;
  /** Runs one of the grid's single-key commands on the selection. */
  readonly onCommand: (command: GridCommand) => void;
  /** Opens page `index` in Read mode (double-click, Enter). */
  readonly onOpen: (index: number) => void;
  /** The page menu for the selection (or the page right-clicked, which becomes the selection). */
  readonly menu: readonly MenuEntry<Id>[];
  readonly onMenuAction: (id: Id) => void;
  readonly platform: KeyPlatform;
  /** Thumbnail size, a fraction of 132 × 171 (0.75–2). */
  readonly scale?: number;
  /** Nothing can move or change: drags and keys that edit are off. */
  readonly readOnly?: boolean;
}

/** A portrait tile's page box at size 1 (O06): 132 × 171, a landscape one 171 × 132. */
const TILE = { long: COMPONENT["organise-tile-size"], short: COMPONENT["organise-tile-short-size"] } as const;

/** The page box of a tile for `page` at `scale`, in CSS px, its shape the page's. */
function boxOf(page: GridPage, scale: number): { width: number; height: number } {
  const portrait = page.height >= page.width;
  const width = (portrait ? TILE.short : TILE.long) * scale;
  const height = (portrait ? TILE.long : TILE.short) * scale;
  const fit = Math.min(width / page.width, height / page.height);
  return { width: Math.round(page.width * fit), height: Math.round(page.height * fit) };
}

/** Room around a tile's page box: its padding (organise.css), and the number under it (CSS px). */
const TILE_PADDING = { inline: 2 * SPACE["10"], block: 2 * SPACE["8"], number: SPACE["8"] + SPACE["16"] } as const;

/** The grid's layout at `scale`: every cell a 100% tile's size, so portrait and landscape pages line up. */
function gridLayout(scale: number) {
  const size = new Size(
    TILE.long * scale + TILE_PADDING.inline,
    TILE.long * scale + TILE_PADDING.block + TILE_PADDING.number,
  );
  return {
    minItemSize: size,
    maxItemSize: size,
    minSpace: new Size(SPACE["16"], SPACE["16"]),
    preserveAspectRatio: true,
  };
}

/** The grid of pages. */
export function PageGrid<Id extends string>(props: PageGridProps<Id>): ReactNode {
  const { pages, selected, scale = 1 } = props;
  const list = useRef<HTMLDivElement>(null);
  // The menu's trigger outlives the menu: the popover still reads it while it closes.
  const [menu, setMenu] = useState<{ at: HTMLElement | null; open: boolean }>({ at: null, open: false });
  const setMenuAt = (at: HTMLElement) => setMenu({ at, open: true });
  const indexOf = (id: string) => pages.findIndex((p) => p.id === id);
  const dragAndDropHooks = usePageDrops(props, indexOf);
  const onKeyDown = (e: KeyboardEvent) => {
    // React's events bubble through portals: keys pressed in the page menu are the menu's.
    if (!(e.target instanceof Node && list.current?.contains(e.target))) return;
    if (gridKeyDown(e, props, list.current, setMenuAt, indexOf)) {
      e.preventDefault();
      e.stopPropagation();
    }
  };
  return (
    <div className="paper-page-grid" onKeyDownCapture={onKeyDown}>
      {/* Only the tiles near the view are rendered: a selection or a move re-renders a screenful, not every page. */}
      <Virtualizer layout={GridLayout} layoutOptions={gridLayout(scale)}>
        <ListBox
          ref={list}
          aria-label={props["aria-label"]}
          layout="grid"
          selectionMode="multiple"
          selectionBehavior="replace"
          items={pages}
          selectedKeys={selected}
          onSelectionChange={(keys: Selection) =>
            props.onSelectionChange(keys === "all" ? new Set(pages.map((p) => p.id)) : new Set([...keys].map(String)))
          }
          onAction={(key) => {
            const index = indexOf(String(key));
            if (index >= 0) props.onOpen(index);
          }}
          dragAndDropHooks={dragAndDropHooks}
          className="paper-page-grid-list"
        >
          {(page) => (
            <ListBoxItem
              id={page.id}
              textValue={page.label}
              aria-label={laterMessage("organise.tile", { page: page.label })}
              className="paper-page-tile"
              data-page-id={page.id}
            >
              <Tile page={page} scale={scale} draw={props.draw} onMenu={setMenuAt} />
            </ListBoxItem>
          )}
        </ListBox>
      </Virtualizer>
      <PageMenu {...props} at={menu.at} open={menu.open} onClose={() => setMenu({ at: menu.at, open: false })} />
    </div>
  );
}

/** Drag and drop: the selection dropped between two tiles moves there, unless the grid is read-only. */
function usePageDrops<Id extends string>(props: PageGridProps<Id>, indexOf: (id: string) => number) {
  const { dragAndDropHooks } = useDragAndDrop<GridPage>({
    getItems: (keys) => [...keys].map((key) => ({ "text/plain": String(key) })),
    onReorder: (e) => {
      const target = indexOf(String(e.target.key));
      if (target < 0) return;
      const moved = [...e.keys].map((key) => indexOf(String(key))).filter((i) => i >= 0);
      props.onMove(moved, e.target.dropPosition === "after" ? target + 1 : target);
    },
    renderDropIndicator: (target) => <DropIndicator target={target} className="paper-page-grid-drop" />,
    renderDragPreview: (items) => (
      <div className="paper-page-grid-ghost" data-count={items.length}>
        {items.length}
      </div>
    ),
    isDisabled: props.readOnly === true,
  });
  return dragAndDropHooks;
}

/** The page menu, at the tile it opened on (a right-click or Shift+F10), for the selection. */
function PageMenu<Id extends string>(
  props: PageGridProps<Id> & { at: HTMLElement | null; open: boolean; onClose: () => void },
) {
  return (
    <AnchoredMenu
      at={props.at}
      open={props.open}
      onClose={props.onClose}
      aria-label={props["aria-label"]}
      items={props.menu}
      onAction={props.onMenuAction}
      platform={props.platform}
    />
  );
}

/** Handles one of the grid's own keys (gridKey); `true` when it was one. */
function gridKeyDown<Id extends string>(
  e: KeyboardEvent,
  props: PageGridProps<Id>,
  list: HTMLElement | null,
  openMenu: (tile: HTMLElement) => void,
  indexOf: (id: string) => number,
): boolean {
  const key = gridKey(e, props.platform);
  if (key === undefined) return false;
  const focused = (document.activeElement as HTMLElement | null)?.closest<HTMLElement>("[data-page-id]") ?? null;
  if (key.kind === "menu") {
    if (focused !== null) openMenu(focused);
    return true;
  }
  if (key.kind === "open") {
    const index = focused === null ? -1 : indexOf(focused.dataset.pageId ?? "");
    if (index >= 0) props.onOpen(index);
    return true;
  }
  if (props.readOnly === true) return true;
  if (key.kind === "command") props.onCommand(key.command);
  else moveBy(key, selectedOf(props), props.pages.length, list, props.onMove);
  return true;
}

/** The selected pages' indices, in order. */
function selectedOf(props: { pages: readonly GridPage[]; selected: ReadonlySet<string> }): number[] {
  return props.pages.flatMap((p, i) => (props.selected.has(p.id) ? [i] : []));
}

/** One tile's picture, drawn once it nears the view and again when its page draws differently; and its number. */
function Tile({
  page,
  scale,
  draw,
  onMenu,
}: {
  page: GridPage;
  scale: number;
  draw: PageGridProps<string>["draw"];
  onMenu: (tile: HTMLElement) => void;
}): ReactNode {
  const canvas = useRef<HTMLCanvasElement>(null);
  const box = boxOf(page, scale);
  // What the picture shows: the page as drawn now, at this size. A change draws it again; a move,
  // which changes only where the page is, doesn't.
  const picture = `${page.drawn}:${box.width}x${box.height}`;
  useEffect(() => {
    const element = canvas.current;
    if (element === null || picture === "") return;
    const abort = new AbortController();
    const seen = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return;
        seen.disconnect();
        draw(page.id, element, abort.signal).catch(() => undefined);
      },
      { rootMargin: "50%" },
    );
    seen.observe(element);
    return () => {
      seen.disconnect();
      abort.abort();
    };
  }, [draw, page.id, picture]);
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: the option around it is the control; this opens its menu on a right-click.
    <span
      className="paper-page-tile-body"
      onContextMenu={(e) => {
        e.preventDefault();
        const tile = e.currentTarget.closest<HTMLElement>("[data-page-id]");
        if (tile !== null) onMenu(tile);
      }}
    >
      <span className="paper-page-tile-box" style={{ inlineSize: box.width, blockSize: box.height }} aria-hidden="true">
        <canvas ref={canvas} />
        <span className="paper-page-tile-check" aria-hidden="true">
          <Icon name="check" size={16} strokeWidth={2} />
        </span>
      </span>
      <span className="paper-page-tile-number" aria-hidden="true">
        {page.label}
      </span>
    </span>
  );
}

/** What a key in the grid does, beyond React Aria's own. */
type GridKey =
  | { readonly kind: "command"; readonly command: GridCommand }
  | { readonly kind: "step"; readonly by: "page" | "row"; readonly forward: boolean }
  | { readonly kind: "menu" }
  | { readonly kind: "open" };

/** O06's keys: ⇧⌘ (Ctrl Shift) with an arrow moves the selection; R, ⇧R and Delete edit it; Shift+F10 opens the menu. */
function gridKey(e: KeyboardEvent, platform: KeyPlatform): GridKey | undefined {
  const mod = platform === "mac" ? e.metaKey : e.ctrlKey;
  if (mod && e.shiftKey && e.key.startsWith("Arrow")) return stepKey(e.key);
  if (e.key === "ContextMenu" || (e.key === "F10" && e.shiftKey)) return { kind: "menu" };
  if (e.metaKey || e.ctrlKey || e.altKey) return undefined;
  return plainKey(e.key, e.shiftKey);
}

/** ⇧⌘ with an arrow: a page left or right, a row up or down. */
function stepKey(key: string): GridKey {
  const by = key === "ArrowUp" || key === "ArrowDown" ? "row" : "page";
  return { kind: "step", by, forward: key === "ArrowRight" || key === "ArrowDown" };
}

/** The keys without a modifier: Enter opens, Delete and Backspace delete, R and ⇧R turn. */
function plainKey(key: string, shift: boolean): GridKey | undefined {
  if (key === "Enter") return { kind: "open" };
  if (key === "Delete" || key === "Backspace") return { kind: "command", command: "delete" };
  if (key === "r" || key === "R") return { kind: "command", command: shift ? "rotate-left" : "rotate-right" };
  return undefined;
}

/** A keyboard move: the selection on a page or a row, a row being as many tiles as fit across. */
function moveBy(
  key: Extract<GridKey, { kind: "step" }>,
  selected: readonly number[],
  count: number,
  list: HTMLElement | null,
  onMove: PageGridProps<string>["onMove"],
): void {
  const step = key.by === "page" ? 1 : rowLength(list);
  const gap = stepGap(selected, key.forward ? step : -step, count);
  if (gap !== undefined) onMove(selected, gap);
}

/** How many tiles sit in the grid's first row: those whose top is the first one's. */
function rowLength(list: HTMLElement | null): number {
  const tiles = list === null ? [] : [...list.querySelectorAll<HTMLElement>("[data-page-id]")];
  const top = tiles[0]?.offsetTop;
  const row = tiles.findIndex((t) => t.offsetTop !== top);
  return row < 0 ? Math.max(1, tiles.length) : row;
}
