// The left rail (UI/UX plan §5.1, design language §7.5): the navigation landmark with the rail's tabs
// (pages and the outline in C6a, bookmarks and attachments later), resizable with the pointer or with
// the arrow keys on its handle. Below the rail breakpoint (token `breakpoint-rail-open`) it is a drawer
// over the canvas, with a scrim, closed by Escape like any other layer.

import { type KeyboardEvent, type PointerEvent, type ReactNode, useEffect, useRef, useState } from "react";
import { SegmentedTabs, TabPanel } from "../components/tabs.js";
import { message } from "../messages.js";
import { LayerScope } from "../overlays/layers.js";
import { useStoreState } from "../state/hooks.js";
import { RAIL_MAX_WIDTH, RAIL_MIN_WIDTH, RAIL_STEP, type ShellLayout } from "./layout.js";

/** Below this window width, the rail is a drawer over the canvas: token `breakpoint-rail-open`. */
export const RAIL_DRAWER_QUERY = "(max-width: 1023.98px)";

/** One tab of the rail, and the panel it shows. */
export interface RailTab {
  readonly id: string;
  /** The visible name ("Pages", "Outline"). */
  readonly label: string;
  readonly panel: ReactNode;
}

/** Props of `LeftRail`. */
export interface LeftRailProps {
  readonly layout: ShellLayout;
  /** The tabs, in order; with none the rail draws nothing. */
  readonly tabs: readonly RailTab[];
}

/** Whether `query` matches now, kept up to date. */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(() => typeof matchMedia === "function" && matchMedia(query).matches);
  useEffect(() => {
    if (typeof matchMedia !== "function") return;
    const list = matchMedia(query);
    const update = () => setMatches(list.matches);
    update();
    list.addEventListener("change", update);
    return () => list.removeEventListener("change", update);
  }, [query]);
  return matches;
}

/** +1 when the inline direction runs left to right at `element`, -1 for right to left. */
function inlineSign(element: Element): 1 | -1 {
  return getComputedStyle(element).direction === "rtl" ? -1 : 1;
}

/** The handle that resizes the rail: a focusable separator with its width as its value. */
function ResizeHandle({ layout, width }: { layout: ShellLayout; width: number }): ReactNode {
  const drag = useRef<{ x: number; width: number } | undefined>(undefined);
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const sign = inlineSign(event.currentTarget);
    const keys: Record<string, number | undefined> = {
      ArrowRight: width + RAIL_STEP * sign,
      ArrowLeft: width - RAIL_STEP * sign,
      Home: RAIL_MIN_WIDTH,
      End: RAIL_MAX_WIDTH,
    };
    const next = keys[event.key];
    if (next === undefined) return;
    event.preventDefault();
    layout.setRailWidth(next);
  };
  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { x: event.clientX, width };
  };
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const start = drag.current;
    if (start === undefined) return;
    layout.setRailWidth(start.width + (event.clientX - start.x) * inlineSign(event.currentTarget));
  };
  const end = () => {
    drag.current = undefined;
  };
  return (
    // biome-ignore lint/a11y/useSemanticElements: a focusable splitter is a `separator` with a value (ARIA APG), which <hr> can't be.
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={message("shell.rail.resize")}
      aria-valuemin={RAIL_MIN_WIDTH}
      aria-valuemax={RAIL_MAX_WIDTH}
      aria-valuenow={width}
      tabIndex={0}
      className="paper-rail-handle"
      onKeyDown={onKeyDown}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={end}
      onPointerCancel={end}
    />
  );
}

/** The rail's tabs and panels. */
function RailTabs({ layout, tabs }: LeftRailProps): ReactNode {
  const selected = useStoreState(layout, (s) => s.rail.tab);
  const first = tabs[0];
  if (first === undefined) return null;
  const current = tabs.some((t) => t.id === selected) ? (selected ?? first.id) : first.id;
  return (
    <SegmentedTabs
      tabs={tabs}
      selectedKey={current}
      onSelectionChange={(tab) => layout.setRailTab(tab)}
      aria-label={message("reader.sidebar")}
    >
      {tabs.map((tab) => (
        <TabPanel key={tab.id} id={tab.id}>
          {tab.panel}
        </TabPanel>
      ))}
    </SegmentedTabs>
  );
}

/**
 * The rail, open or closed as `layout` says, hidden in focus mode. Docked beside the canvas on wide
 * windows; a drawer over it below the rail breakpoint.
 */
export function LeftRail({ layout, tabs }: LeftRailProps): ReactNode {
  const open = useStoreState(layout, (s) => s.rail.open && !s.focus);
  const width = useStoreState(layout, (s) => s.rail.width);
  const drawer = useMediaQuery(RAIL_DRAWER_QUERY);
  if (!open || tabs.length === 0) return null;
  const rail = (
    <nav
      className="paper-left-rail"
      aria-label={message("reader.sidebar")}
      data-drawer={drawer || undefined}
      style={{ inlineSize: drawer ? undefined : `${width}px` }}
    >
      <RailTabs layout={layout} tabs={tabs} />
      {drawer ? null : <ResizeHandle layout={layout} width={width} />}
    </nav>
  );
  if (!drawer) return rail;
  return (
    <LayerScope dismiss={() => layout.setRailOpen(false)}>
      <button
        type="button"
        className="paper-rail-scrim"
        aria-label={message("shell.rail.close")}
        onClick={() => layout.setRailOpen(false)}
      />
      {rail}
    </LayerScope>
  );
}
