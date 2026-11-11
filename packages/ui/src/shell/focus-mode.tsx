// Focus mode's status-bar item (design R16): while focus mode is on, a toggle named "Focus mode"
// with its shortcut, which leaves focus mode when pressed. Focus mode is also a layer: Escape closes
// every layer opened on top (a dialog) first, then clears a selection, then leaves focus mode. The
// item must be in the status bar for Escape to leave focus mode. Turning it on or off is announced
// once, in a polite live region.

import { type ReactNode, useEffect, useRef, useState } from "react";
import { ToggleButton } from "react-aria-components";
import { useCommand, useCommands } from "../commands/react.js";
import { ShortcutChip } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { LayerScope } from "../overlays/layers.js";
import { useStoreState } from "../state/hooks.js";
import type { ShellLayout } from "./layout.js";

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
  return (
    <>
      <span className="paper-visually-hidden" role="status">
        {said}
      </span>
      {focus ? (
        <LayerScope dismiss={() => layout.escapeFocus()}>
          <ToggleButton className="paper-status-item" isSelected onChange={() => layout.leaveFocus()}>
            {/* Named by its content, chip included, as the top bar's Commands button is: a name must
                hold all the visible text (WCAG 2.5.3). The space keeps the two apart in the name;
                the flex gap does on screen. */}
            {message("shell.focus.item")}
            {shortcut === undefined ? null : (
              <>
                {" "}
                <ShortcutChip keys={shortcut} platform={keys} />
              </>
            )}
          </ToggleButton>
        </LayerScope>
      ) : null}
    </>
  );
}
