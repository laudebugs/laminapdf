// The Comment mode's toolbar (Paper C01, §7.24), under the top bar while Comment is the mode:
// Select · Highlight · Underline · Strike · Caret · Note · Text box ▾ · Ink · Shapes ▾, then the colour and
// width the next mark takes while a creation tool is on, or the page's comment count while Select
// is. Each tool is its command's view (useCommand), so it is disabled, with the reason, when the
// command can't run. Arrow keys move between the tools; Tab leaves the toolbar.

import { Icon } from "@laminapdf/icons/react";
import { type KeyboardEvent, type ReactNode, useEffect, useRef, useState } from "react";
import {
  Button as AriaButton,
  Menu as AriaMenu,
  Popover as AriaPopover,
  Header,
  MenuItem,
  MenuSection,
  MenuTrigger,
  Text,
} from "react-aria-components";
import { useCommand } from "../commands/react.js";
import { Select } from "../components/field.js";
import { StableLabel } from "../components/stable-label.js";
import { type Swatch, SwatchGroup } from "../components/swatches.js";
import { ToolToggle } from "../components/toggle.js";
import { type LaterMessageId, laterCountMessage, laterMessage } from "../later-messages.js";
import { POPOVER_LAYER } from "../overlays/popover.js";
import {
  COMMENT_TOOLS,
  type CommentColour,
  type CommentTool,
  LINE_WIDTHS,
  SHAPE_ICONS,
  SHAPE_KINDS,
  type ShapeKind,
  TEXT_ICONS,
  TEXT_KINDS,
  type TextKind,
  TOOL_ICONS,
  textCommand,
  toolCommand,
} from "./commands.js";

/** Props of `CommentToolbar`. */
export interface CommentToolbarProps {
  /** The tool on. */
  readonly tool: CommentTool;
  /** The shape Shapes ▾ draws. */
  readonly shape: ShapeKind;
  /** What Text box ▾ makes: the last text tool used. */
  readonly text: TextKind;
  /** The colours the tool on offers (fill or line colours); none for Select. */
  readonly colours: readonly Swatch<CommentColour>[];
  /** The colour the next mark takes. */
  readonly colour: CommentColour | undefined;
  /** Sets the colour the next mark takes. */
  readonly onColour: (colour: CommentColour) => void;
  /** The line width the next mark takes, for a line tool; undefined for the others. */
  readonly width: number | undefined;
  /** Sets the line width the next mark takes. */
  readonly onWidth: (width: number) => void;
  /** The comments on the current page, shown while Select is on. */
  readonly count: number;
  /** The one-line hint for the tool on (C01's), which a polite live region says. */
  readonly hint: string;
}

/** One tool as a toggle, pressed while it is on. */
function Tool({ tool, on }: { tool: Exclude<CommentTool, "shape">; on: boolean }): ReactNode {
  const view = useCommand(toolCommand(tool));
  if (view.availability.state === "hidden") return null;
  const disabled = view.availability.state === "disabled";
  const reason = view.availability.state === "disabled" ? view.availability.reason : undefined;
  return (
    <ToolToggle
      icon={TOOL_ICONS[tool]}
      isSelected={on}
      isDisabled={disabled}
      data-tool={tool}
      {...(reason === undefined ? {} : { "aria-description": reason })}
      onChange={() => view.run()}
    >
      {laterMessage(`comment.tool.${tool}` as LaterMessageId)}
    </ToolToggle>
  );
}

/** Shapes ▾: a menu of radio items, its button pressed while a shape tool is on. */
function Shapes({ on, shape }: { on: boolean; shape: ShapeKind }): ReactNode {
  const view = useCommand(toolCommand("shape"));
  if (view.availability.state === "hidden") return null;
  const label = laterMessage("comment.tool.shape");
  return (
    <MenuTrigger>
      <AriaButton
        className="paper-tool-toggle"
        data-selected={on || undefined}
        data-tool="shape"
        isDisabled={view.availability.state === "disabled"}
        aria-label={`${label}: ${laterMessage(`comment.shape.${shape}`)}`}
      >
        <Icon name={SHAPE_ICONS[shape]} size={16} />
        <span className="paper-toggle-label">
          <StableLabel>{label}</StableLabel>
        </span>
        <Icon name="chevron-down" size={16} />
      </AriaButton>
      <AriaPopover className="paper-popover paper-menu-popover" placement="bottom start" style={POPOVER_LAYER}>
        <AriaMenu
          className="paper-menu"
          aria-label={laterMessage("comment.shapes")}
          selectionMode="single"
          selectedKeys={[shape]}
          onAction={(key) => view.run(key as ShapeKind)}
        >
          <MenuSection>
            {SHAPE_KINDS.map((kind) => (
              <MenuItem
                key={kind}
                id={kind}
                textValue={laterMessage(`comment.shape.${kind}`)}
                className="paper-menu-item"
              >
                <Icon name={SHAPE_ICONS[kind]} />
                <Text slot="label" className="paper-menu-label">
                  {laterMessage(`comment.shape.${kind}`)}
                </Text>
              </MenuItem>
            ))}
          </MenuSection>
          <MenuSection>
            <Header className="paper-menu-reason paper-comment-shapes-hint">
              {laterMessage("comment.shapes.hint")}
            </Header>
          </MenuSection>
        </AriaMenu>
      </AriaPopover>
    </MenuTrigger>
  );
}

/**
 * Text box ▾, a split tool button (C01, §7.24): the toggle turns on the last text tool used and
 * names it, and the 20 px chevron ("Choose a text tool") opens the menu of all three.
 */
function TextTools({ on, kind }: { on: boolean; kind: TextKind }): ReactNode {
  const view = useCommand(textCommand(kind));
  const callout = useCommand(textCommand("callout"));
  const typewriter = useCommand(textCommand("typewriter"));
  const box = useCommand(textCommand("textBox"));
  if (view.availability.state === "hidden") return null;
  const disabled = view.availability.state === "disabled";
  const views = { textBox: box, callout, typewriter };
  return (
    <div className="paper-comment-split" data-selected={on || undefined}>
      <ToolToggle
        icon={TEXT_ICONS[kind]}
        isSelected={on}
        isDisabled={disabled}
        data-tool="text"
        onChange={() => view.run()}
      >
        {laterMessage(`comment.text.${kind}`)}
      </ToolToggle>
      <MenuTrigger>
        <AriaButton
          className="paper-tool-toggle paper-comment-split-menu"
          data-selected={on || undefined}
          isDisabled={disabled}
          aria-label={laterMessage("comment.texts")}
        >
          <Icon name="chevron-down" size={16} />
        </AriaButton>
        <AriaPopover className="paper-popover paper-menu-popover" placement="bottom start" style={POPOVER_LAYER}>
          <AriaMenu
            className="paper-menu"
            aria-label={laterMessage("comment.texts")}
            selectionMode="single"
            selectedKeys={[kind]}
            onAction={(key) => views[key as TextKind].run()}
          >
            {TEXT_KINDS.map((k) => (
              <MenuItem key={k} id={k} textValue={laterMessage(`comment.text.${k}`)} className="paper-menu-item">
                <Icon name={TEXT_ICONS[k]} />
                <Text slot="label" className="paper-menu-label">
                  {laterMessage(`comment.text.${k}`)}
                </Text>
              </MenuItem>
            ))}
          </AriaMenu>
        </AriaPopover>
      </MenuTrigger>
    </div>
  );
}

/** The tools in `toolbar`, in order: the toggles that can be pressed now. */
function toolsIn(toolbar: Element): HTMLElement[] {
  return [...toolbar.querySelectorAll<HTMLElement>(".paper-tool-toggle:not([disabled])")];
}

/**
 * The arrow keys move between the tools, in the reading direction (← and → swap in a
 * right-to-left layout), and the focus takes the one tab stop with it, so Tab leaves the tools.
 */
function moveBetweenTools(event: KeyboardEvent<HTMLDivElement>): void {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  const tools = toolsIn(event.currentTarget.querySelector(".paper-comment-tools") ?? event.currentTarget);
  const at = tools.indexOf(document.activeElement as HTMLElement);
  if (at < 0) return;
  event.preventDefault();
  const rtl = getComputedStyle(event.currentTarget).direction === "rtl";
  const forward = (event.key === "ArrowRight") !== rtl;
  const next = tools[(at + (forward ? 1 : tools.length - 1)) % tools.length];
  for (const t of tools) t.tabIndex = t === next ? 0 : -1;
  next?.focus();
}

/** One tab stop among the tools: the tool on, or the first. */
function useOneTabStop(tool: CommentTool) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const tools = ref.current === null ? [] : toolsIn(ref.current);
    const on = tools.find((t) => t.dataset.tool === tool) ?? tools[0];
    for (const t of tools) t.tabIndex = t === on ? 0 : -1;
  });
  return ref;
}

/**
 * Text a polite live region says: the region mounts empty and takes its text after, so a screen
 * reader announces the first text too, and it changes only when the text does.
 */
function LiveText({ text, className }: { text: string; className: string }): ReactNode {
  const [shown, setShown] = useState("");
  useEffect(() => setShown(text), [text]);
  return (
    <span className={className} role="status">
      {shown}
    </span>
  );
}

/** The toolbar. */
export function CommentToolbar(props: CommentToolbarProps): ReactNode {
  const { tool, shape, text, colours, colour, width, count } = props;
  const creating = tool !== "select";
  const tools = useOneTabStop(tool);
  return (
    <div
      className="paper-comment-toolbar"
      role="toolbar"
      aria-label={laterMessage("comment.tools")}
      onKeyDown={moveBetweenTools}
    >
      <div className="paper-comment-tools" ref={tools}>
        {COMMENT_TOOLS.map((t) =>
          t === "shape" ? (
            <Shapes key={t} on={tool === "shape"} shape={shape} />
          ) : t === "text" ? (
            <TextTools key={t} on={tool === "text"} kind={text} />
          ) : (
            <Tool key={t} tool={t} on={tool === t} />
          ),
        )}
      </div>
      <span className="paper-organise-divider" aria-hidden="true" />
      {creating && colours.length > 0 ? (
        <SwatchGroup
          label={laterMessage("comment.colour")}
          swatches={colours}
          value={colour ?? (colours[0]?.id as CommentColour)}
          onChange={props.onColour}
        />
      ) : null}
      {creating && width !== undefined ? (
        <div className="paper-comment-width">
          <Select
            label={laterMessage("comment.width")}
            options={LINE_WIDTHS.map((w) => ({
              value: String(w),
              label: laterMessage("comment.width.value", { width: w }),
            }))}
            value={String(width)}
            onChange={(value) => props.onWidth(Number(value))}
          />
        </div>
      ) : null}
      {creating ? null : (
        <LiveText
          className="paper-comment-count"
          text={laterCountMessage(count, {
            zero: "comment.count.none",
            one: "comment.count.one",
            many: "comment.count",
          })}
        />
      )}
      <LiveText className="paper-visually-hidden" text={props.hint} />
    </div>
  );
}
