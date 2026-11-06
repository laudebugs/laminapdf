// The top bar's zoom (§7.1): zoom out, the percentage, zoom in. The percentage is a menu button:
// fit width, fit page, actual size and the presets. Every part is reachable from the keyboard,
// and the buttons stop at the viewer's range.

import { type ReactNode, useMemo } from "react";
import { Button as AriaButton, Group } from "react-aria-components";
import { IconButton } from "../components/button.js";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { Menu, type MenuEntry, MenuTrigger } from "../overlays/popover.js";
import { useStoreState } from "../state/hooks.js";
import {
  MAX_ZOOM,
  MIN_ZOOM,
  type ViewerControls,
  ZOOM_PRESETS,
  type Zoom,
  zoomIn,
  zoomOut,
} from "./viewer-controls.js";

/** Props of `ZoomControl`. */
export interface ZoomControlProps {
  viewer: ViewerControls;
  /** How shortcut chips are written. */
  keys: KeyPlatform;
}

type ZoomId = "fit-width" | "fit-page" | "actual-size" | `${number}`;

/** The menu's entries: the fit modes, then the presets. Built when rendered, in the catalogue's words. */
function zoomEntries(): MenuEntry<ZoomId>[] {
  return [
    { id: "fit-width", label: message("reader.zoom.fitWidth"), icon: "fit-width" },
    { id: "fit-page", label: message("reader.zoom.fitPage"), icon: "fit-page" },
    { id: "actual-size", label: message("reader.zoom.actualSize") },
    ...ZOOM_PRESETS.map((percent) => ({
      id: `${percent}` as const,
      label: message("reader.zoom.percent", { percent }),
    })),
  ];
}

/** Zoom out, the percentage with its menu, and zoom in. */
export function ZoomControl({ viewer, keys }: ZoomControlProps): ReactNode {
  const percent = Math.round(useStoreState(viewer, (state) => state.percent));
  const choose = (id: ZoomId) => viewer.setZoom(asZoom(id));
  const entries = useMemo(zoomEntries, []);
  return (
    <Group className="paper-zoom-control" aria-label={message("reader.zoom")}>
      <IconButton
        icon="zoom-out"
        size={26}
        label={message("reader.zoom.out")}
        isDisabled={percent <= MIN_ZOOM}
        onPress={() => viewer.setZoom(zoomOut(percent))}
      />
      <MenuTrigger>
        <AriaButton className="paper-zoom-percent" aria-label={message("reader.zoom.current", { percent })}>
          {message("reader.zoom.percent", { percent })}
        </AriaButton>
        <Menu aria-label={message("reader.zoom")} items={entries} onAction={choose} platform={keys} />
      </MenuTrigger>
      <IconButton
        icon="zoom-in"
        size={26}
        label={message("reader.zoom.in")}
        isDisabled={percent >= MAX_ZOOM}
        onPress={() => viewer.setZoom(zoomIn(percent))}
      />
    </Group>
  );
}

function asZoom(id: ZoomId): Zoom {
  return id === "fit-width" || id === "fit-page" || id === "actual-size" ? id : Number(id);
}
