// Comment mode's touch bars (C08, C09): the phone's annotate bar, which takes the bottom bar's
// place, with the options strip above it; and the tablet's tool shelf, which takes the Comment
// toolbar's place once a pen is seen. Each tool button runs its command, so the palette, the
// keys and the bars agree.

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import { Button as AriaButton, ToggleButton as AriaToggleButton } from "react-aria-components";
import { useCommand } from "../commands/react.js";
import { Switch } from "../components/choice.js";
import { type Swatch, SwatchGroup } from "../components/swatches.js";
import { type LaterMessageId, laterMessage } from "../later-messages.js";
import { type CommentColour, type CommentTool, TOOL_ICONS, toolCommand } from "./commands.js";

/** The widths' names, thinnest first. */
const WIDTH_NAMES = ["thin", "medium", "thick"] as const;

/** What the options strip offers for the tool on. */
export interface TouchOptions {
  /** The colours; none for the Eraser and the Lasso, which take no options. */
  readonly colours: readonly Swatch<CommentColour>[];
  readonly colour: CommentColour | undefined;
  readonly onColour: (colour: CommentColour) => void;
  /** Thin, Medium and Thick, in points; none for a tool without a width. */
  readonly widths: readonly [number, number, number] | undefined;
  readonly width: number | undefined;
  readonly onWidth: (width: number) => void;
}

/** One tool as a toggle in a touch bar: icon over label, pressed while it is on. */
function TouchTool({ tool, on, label }: { tool: CommentTool; on: boolean; label?: string }): ReactNode {
  const view = useCommand(toolCommand(tool));
  if (view.availability.state === "hidden") return null;
  return (
    <AriaToggleButton
      className="paper-touch-tool"
      isSelected={on}
      isDisabled={view.availability.state === "disabled"}
      data-tool={tool}
      onChange={() => view.run()}
    >
      <Icon name={tool === "ink" ? "pen" : TOOL_ICONS[tool]} />
      <span>{label ?? laterMessage(`comment.tool.${tool}` as LaterMessageId)}</span>
    </AriaToggleButton>
  );
}

/** Colours and widths for the tool on; nothing for one that takes none. */
export function OptionsStrip(props: TouchOptions): ReactNode {
  const { colours, widths } = props;
  if (colours.length === 0 && widths === undefined) return null;
  return (
    <fieldset className="paper-touch-options" aria-label={laterMessage("comment.touch.options")}>
      {colours.length === 0 ? null : (
        <SwatchGroup
          label={laterMessage("comment.colour")}
          swatches={colours}
          value={props.colour ?? (colours[0]?.id as CommentColour)}
          onChange={props.onColour}
        />
      )}
      {widths === undefined ? null : <Widths widths={widths} width={props.width} onWidth={props.onWidth} />}
    </fieldset>
  );
}

/** Thin, Medium and Thick: toggles showing a line at each width. */
function Widths({
  widths,
  width,
  onWidth,
}: {
  widths: readonly [number, number, number];
  width: number | undefined;
  onWidth: (width: number) => void;
}): ReactNode {
  return (
    <fieldset className="paper-touch-widths" aria-label={laterMessage("comment.width")}>
      {widths.map((w, i) => (
        <AriaToggleButton
          key={w}
          className="paper-touch-width"
          isSelected={width === w}
          onChange={() => onWidth(w)}
          aria-label={laterMessage(`comment.touch.width.${WIDTH_NAMES[i] ?? "medium"}`)}
          data-width={WIDTH_NAMES[i]}
        >
          <span className="paper-touch-width-line" aria-hidden="true" />
        </AriaToggleButton>
      ))}
    </fieldset>
  );
}

/** Props of `AnnotateBar`. */
export interface AnnotateBarProps {
  /** The tool on. */
  readonly tool: CommentTool;
  /** What the strip above the bar offers for it. */
  readonly options: TouchOptions;
}

/** The phone's annotate bar (C08): Pen, Highlighter, Eraser, Note and Done, under the options strip. */
export function AnnotateBar({ tool, options }: AnnotateBarProps): ReactNode {
  const done = useCommand("comment.done" as never);
  return (
    <div className="paper-annotate">
      <OptionsStrip {...options} />
      <div
        className="paper-bottom-bar paper-annotate-bar"
        role="toolbar"
        aria-label={laterMessage("comment.touch.bar")}
      >
        <TouchTool tool="ink" on={tool === "ink"} label={laterMessage("comment.tool.pen")} />
        <TouchTool tool="highlighter" on={tool === "highlighter"} />
        <TouchTool tool="eraser" on={tool === "eraser"} />
        <TouchTool tool="note" on={tool === "note"} />
        <AriaButton className="paper-touch-tool" onPress={() => done.run()}>
          <Icon name="check" />
          <span>{laterMessage("comment.touch.done")}</span>
        </AriaButton>
      </div>
    </div>
  );
}

/** Props of `ToolShelf`. */
export interface ToolShelfProps {
  readonly tool: CommentTool;
  readonly options: TouchOptions;
  /** Whether a finger draws. */
  readonly drawWithFinger: boolean;
  readonly onDrawWithFinger: (on: boolean) => void;
}

/** The tablet's tool shelf (C09): the pen tools, the colours and widths, and Draw with finger. */
export function ToolShelf({ tool, options, drawWithFinger, onDrawWithFinger }: ToolShelfProps): ReactNode {
  return (
    <div className="paper-tool-shelf" role="toolbar" aria-label={laterMessage("comment.touch.shelf")}>
      <div className="paper-tool-shelf-tools">
        <TouchTool tool="ink" on={tool === "ink"} label={laterMessage("comment.tool.pen")} />
        <TouchTool tool="highlighter" on={tool === "highlighter"} />
        <TouchTool tool="eraser" on={tool === "eraser"} />
        <TouchTool tool="lasso" on={tool === "lasso"} />
        <TouchTool tool="shape" on={tool === "shape"} />
        <TouchTool tool="text" on={tool === "text"} />
        <TouchTool tool="note" on={tool === "note"} />
      </div>
      <OptionsStrip {...options} />
      <Switch isSelected={drawWithFinger} onChange={onDrawWithFinger}>
        {laterMessage("comment.touch.finger")}
      </Switch>
    </div>
  );
}
