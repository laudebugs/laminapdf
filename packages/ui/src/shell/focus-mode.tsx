// Focus mode's status-bar item (design R16): while focus mode is on, a toggle named "Focus mode"
// with its shortcut, which leaves focus mode when pressed. Focus mode is also a layer, so Escape
// leaves it once every layer opened on top (a selection, a dialog) has closed. Turning it on or off
// is announced once, in a polite live region.

import { type ReactNode, useEffect, useRef, useState } from "react";
import { ToggleButton } from "react-aria-components";
import { useCommand, useCommands } from "../commands/react.js";
import { ShortcutChip } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { LayerScope } from "../overlays/layers.js";
import { useStoreState } from "../state/hooks.js";
import type { ShellLayout } from "./layout.js";

/** `Mod+\` as `aria-keyshortcuts` writes it on `platform`. */
function ariaKeys(keys: string, mac: boolean): string {
  return keys
    .split("+")
    .map((k) => (k === "Mod" ? (mac ? "Meta" : "Control") : k === "\\" ? "Backslash" : k))
    .join("+");
}

/** The status-bar item and the announcement; place it in the status bar. */
export function FocusModeItem({ layout }: { layout: ShellLayout }): ReactNode {
  const focus = useStoreState(layout, (s) => s.focus);
  const command = useCommand("view.focus-mode");
  const { keys } = useCommands();
  const [said, setSaid] = useState("");
  const first = useRef(true);
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    setSaid(message(focus ? "shell.focus.on" : "shell.focus.off"));
  }, [focus]);
  const shortcut = command.shortcut;
  const mac = keys === "mac";
  return (
    <>
      <span className="paper-visually-hidden" role="status">
        {said}
      </span>
      {focus ? (
        <LayerScope dismiss={() => layout.leaveFocus()}>
          <ToggleButton
            className="paper-status-item"
            isSelected
            onChange={() => layout.leaveFocus()}
            {...(shortcut === undefined
              ? {}
              : { "aria-keyshortcuts": ariaKeys(mac ? shortcut.mac : shortcut.other, mac) })}
          >
            {message("shell.focus.item")}
            {shortcut === undefined ? null : <ShortcutChip keys={shortcut} platform={keys} />}
          </ToggleButton>
        </LayerScope>
      ) : null}
    </>
  );
}
