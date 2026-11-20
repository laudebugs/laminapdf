// Comment mode on phones and tablets (C08, C09): the touch tools' commands, the phone's annotate
// bar in the bottom bar's place with its options strip and scroll hint, and the tablet's tool
// shelf once a pen has drawn.

import type { ModeRegistry } from "@laminapdf/ui";
import { AnnotateBar, type CommentTool, laterMessage, ToolShelf, type TouchOptions } from "@laminapdf/ui/editing";
import { type ReactNode, useEffect, useState } from "react";
import type { CommentController, CommentState } from "./controller.ts";
import { familyOf, styleKey, takesWidth } from "./styles.ts";
import { swatches } from "./swatches.ts";

/** Thin, Medium and Thick for a pen, in points (C08). */
const PEN_WIDTHS = [1.5, 3, 5] as const;
/** Thin, Medium and Thick for the Highlighter. */
const HIGHLIGHTER_WIDTHS = [8, 12, 18] as const;
/** How many annotate sessions show the scroll hint (C08). */
const HINT_SESSIONS = 3;
const HINT_KEY = "lamina.comment.scroll-hint";

/** The touch tools' and bars' commands. */
export function touchActions(
  on: (run: (controller: CommentController) => void) => () => void,
  tool: (name: CommentTool) => (args?: unknown) => void,
  modes: ModeRegistry,
) {
  return {
    "comment.highlighter": tool("highlighter"),
    "comment.eraser": tool("eraser"),
    "comment.lasso": tool("lasso"),
    // On a phone a finger draws whatever this says; the setting waits for a wider window.
    "comment.draw-with-finger": on((c) => c.setDrawWithFinger(!c.getState().drawWithFinger)),
    // Done leaves annotating; the changes stay unsaved (C08).
    "comment.done": () => modes.switchTo("read"),
  };
}

/** What the options strip offers for the tool on. */
function touchOptions(controller: CommentController, state: CommentState): TouchOptions {
  const key = styleKey(state.tool, state.shape);
  const style = key === undefined ? undefined : state.styles[key];
  const widths =
    key === "highlighter" ? HIGHLIGHTER_WIDTHS : key !== undefined && takesWidth(key) ? PEN_WIDTHS : undefined;
  return {
    colours: key === undefined ? [] : swatches(familyOf(key)),
    colour: style?.colour,
    onColour: (colour) => controller.setStyle({ colour }),
    widths,
    width: style?.width,
    onWidth: (width) => controller.setStyle({ width }),
  };
}

/** Whether this device's main pointer is a finger or a pen: a phone or a tablet. */
export function useCoarsePointer(): boolean {
  const query = "(any-pointer: coarse)";
  const [coarse, setCoarse] = useState(() => typeof matchMedia === "function" && matchMedia(query).matches);
  useEffect(() => {
    if (typeof matchMedia !== "function") return;
    const list = matchMedia(query);
    const change = () => setCoarse(list.matches);
    list.addEventListener("change", change);
    return () => list.removeEventListener("change", change);
  }, []);
  return coarse;
}

/** Whether this annotate session shows the scroll hint: the first three do (C08). */
function useScrollHint(): boolean {
  const [shown] = useState(() => {
    try {
      const seen = Number(localStorage.getItem(HINT_KEY) ?? "0");
      localStorage.setItem(HINT_KEY, String(seen + 1));
      return seen < HINT_SESSIONS;
    } catch {
      return true;
    }
  });
  return shown;
}

/** The phone's annotate area: the scroll hint, the options strip and the annotate bar. */
export function PhoneAnnotate({
  controller,
  state,
}: {
  controller: CommentController;
  state: CommentState;
}): ReactNode {
  const hint = useScrollHint();
  return (
    <div className="web-annotate">
      {hint ? (
        <p className="web-annotate-hint" aria-live="polite">
          {laterMessage("comment.touch.scroll-hint")}
        </p>
      ) : null}
      <AnnotateBar tool={state.tool} options={touchOptions(controller, state)} />
    </div>
  );
}

/** The tablet's tool shelf, with the note that says what draws (C09). */
export function TabletShelf({ controller, state }: { controller: CommentController; state: CommentState }): ReactNode {
  return (
    <div className="web-tool-shelf">
      <ToolShelf
        tool={state.tool}
        options={touchOptions(controller, state)}
        drawWithFinger={state.drawWithFinger}
        onDrawWithFinger={(on) => controller.setDrawWithFinger(on)}
      />
      <p className="web-tool-shelf-note" aria-live="polite">
        {laterMessage(state.drawWithFinger ? "comment.touch.finger-draws" : "comment.touch.pen-draws")}
      </p>
    </div>
  );
}
