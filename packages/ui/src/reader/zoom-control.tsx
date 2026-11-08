// The top bar's zoom (§7.1): zoom out, the percentage, zoom in. The percentage is a menu button:
// fit width, fit page, actual size and the presets. Every part runs a registered command
// (`view.zoom-out`, `view.fit-width`, `view.set-zoom` …), so its labels, shortcuts and limits are
// the palette's; every part is reachable from the keyboard, and the buttons stop at the range.

import { type ReactNode, useMemo } from "react";
import { Button as AriaButton, Group } from "react-aria-components";
import { type CommandView, useCommand } from "../commands/react.js";
import { IconButton } from "../components/button.js";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { Menu, type MenuEntry, MenuTrigger } from "../overlays/popover.js";
import { useStoreState } from "../state/hooks.js";
import { type ViewerControls, ZOOM_PRESETS } from "./viewer-controls.js";

/** Props of `ZoomControl`. */
export interface ZoomControlProps {
  /** The viewer, whose zoom the percentage shows. */
  viewer: ViewerControls;
  /** How shortcut chips are written. */
  keys: KeyPlatform;
}

type ZoomId = "fit-width" | "fit-page" | "actual-size" | `${number}`;

/** A command as a menu entry: its title and chip, disabled with its reason when it can't run. */
function entry<Id extends string>(id: Id, command: CommandView, extra: Partial<MenuEntry<Id>> = {}): MenuEntry<Id> {
  const a = command.availability;
  return {
    id,
    label: command.title,
    ...(command.shortcut === undefined ? {} : { shortcut: command.shortcut }),
    ...(a.state === "disabled" ? { isDisabled: true, disabledReason: a.reason } : {}),
    ...extra,
  };
}

/** Zoom out, the percentage with its menu, and zoom in. */
export function ZoomControl({ viewer, keys }: ZoomControlProps): ReactNode {
  const percent = Math.round(useStoreState(viewer, (state) => state.percent));
  const zoomOut = useCommand("view.zoom-out");
  const zoomIn = useCommand("view.zoom-in");
  const fitWidth = useCommand("view.fit-width");
  const fitPage = useCommand("view.fit-page");
  const actualSize = useCommand("view.actual-size");
  const setZoom = useCommand("view.set-zoom");
  const entries = useMemo(
    (): MenuEntry<ZoomId>[] => [
      entry("fit-width", fitWidth, { icon: "fit-width" }),
      entry("fit-page", fitPage, { icon: "fit-page" }),
      entry("actual-size", actualSize),
      ...ZOOM_PRESETS.map((p) =>
        entry(`${p}` as const, setZoom, { label: message("reader.zoom.percent", { percent: p }) }),
      ),
    ],
    [fitWidth, fitPage, actualSize, setZoom],
  );
  const choose = (id: ZoomId) => {
    if (id === "fit-width") fitWidth.run();
    else if (id === "fit-page") fitPage.run();
    else if (id === "actual-size") actualSize.run();
    else setZoom.run(Number(id));
  };
  return (
    <Group className="paper-zoom-control" aria-label={message("reader.zoom")}>
      <IconButton
        icon="zoom-out"
        size={26}
        label={zoomOut.title}
        isDisabled={zoomOut.availability.state !== "available"}
        onPress={() => zoomOut.run()}
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
        label={zoomIn.title}
        isDisabled={zoomIn.availability.state !== "available"}
        onPress={() => zoomIn.run()}
      />
    </Group>
  );
}
