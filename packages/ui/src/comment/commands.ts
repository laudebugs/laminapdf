// Comment mode's commands (Paper C01, C02): the tools (`comment.*`), the colour and width the next
// mark takes, and the inspector's changes to the selected annotations (`annot.*`). The shell runs
// them (`delegated`): it holds the tool, the selection and the edit session. Their home is the
// Comment toolbar and the inspector; Highlight, Underline and Ink have single keys on the page,
// which the Settings switch for single-key shortcuts turns off with the others.

import type { IconName } from "@laminapdf/icons";
import { parseChoice } from "../commands/args.js";
import type { CommandRegistry } from "../commands/registry.js";
import type { ArgsSchema, Command, CommandContext, CommandHome, CommandId, Shortcut } from "../commands/types.js";
import { blockedReason } from "../edit/outcome.js";
import { type LaterMessageId, laterMessage } from "../later-messages.js";
import { STATUS_CHOICES } from "./comment-thread.js";

/** The tools of the Comment toolbar, in its order. */
export const COMMENT_TOOLS = [
  "select",
  "highlight",
  "underline",
  "strike",
  "caret",
  "note",
  "text",
  "ink",
  "shape",
  "stamp",
  "attach",
] as const;

/** The touch tools a phone's annotate bar and a tablet's tool shelf add (C08, C09). */
export const TOUCH_TOOLS = ["highlighter", "eraser", "lasso"] as const;

/** One of them. */
export type TouchTool = (typeof TOUCH_TOOLS)[number];

/** A Comment tool: the toolbar's, or a touch tool. */
export type CommentTool = (typeof COMMENT_TOOLS)[number] | TouchTool;

/** What Shapes ▾ draws, in its menu's order. */
export const SHAPE_KINDS = ["rectangle", "ellipse", "line", "arrow", "polygon", "polyline"] as const;

/** One of them. */
export type ShapeKind = (typeof SHAPE_KINDS)[number];

/** What Text box ▾ makes (C03), in its menu's order. */
export const TEXT_KINDS = ["textBox", "callout", "typewriter"] as const;

/** One of them. */
export type TextKind = (typeof TEXT_KINDS)[number];

/** Each text tool's icon. */
export const TEXT_ICONS: Readonly<Record<TextKind, IconName>> = {
  textBox: "text-box",
  callout: "callout",
  typewriter: "add-text",
};

/** A text tool's command id: `comment.text-box`, `comment.callout`, `comment.typewriter`. */
export const textCommand = (kind: TextKind): CommandId =>
  `comment.${kind === "textBox" ? "text-box" : kind}` as CommandId;

/** The standard fonts a text box takes on the web (C03), by the names the spec takes. */
export const TEXT_FONTS = ["helvetica", "times", "courier"] as const;

/** A box's alignment (`/Q`). */
export const TEXT_ALIGNMENTS = ["left", "center", "right"] as const;

/** One of them. */
export type TextAlignment = (typeof TEXT_ALIGNMENTS)[number];

/** The sizes the Size field lists, in points; any size from 1 to 500 can be typed. */
export const TEXT_SIZES = [8, 9, 10, 11, 12, 14, 16, 18, 24, 36, 48] as const;

/** The four fill colours (highlights, notes, a shape's fill), named as the design language names them. */
export const FILL_COLOURS = ["yellow", "green", "blue", "pink"] as const;

/** The four line colours (underlines, strike-outs, carets, ink, shapes' outlines). */
export const LINE_COLOURS = ["black", "red", "blue", "green"] as const;

/** A colour's name, in either set. */
export type CommentColour = (typeof FILL_COLOURS)[number] | (typeof LINE_COLOURS)[number];

/** The widths a line tool takes, in points (C01: 0.5–8 pt). */
export const LINE_WIDTHS = [0.5, 1, 1.5, 2, 3, 4, 6, 8] as const;

/** An outline's style (C02's Border toggle group; cloudy is `/BE`). */
export const BORDER_STYLES = ["solid", "dashed", "cloudy"] as const;

/** One of them. */
export type BorderStyleName = (typeof BORDER_STYLES)[number];

/** Every line ending (`/LE`, ISO 32000-2 Table 179), in the order the Line start and Line end selects list them. */
export const LINE_ENDINGS = [
  "none",
  "openArrow",
  "closedArrow",
  "circle",
  "square",
  "diamond",
  "butt",
  "slash",
  "rOpenArrow",
  "rClosedArrow",
] as const;

/** One of them. */
export type LineEndingName = (typeof LINE_ENDINGS)[number];

/** Each tool's icon. */
export const TOOL_ICONS: Readonly<Record<CommentTool, IconName>> = {
  select: "select",
  highlight: "highlight",
  underline: "underline",
  strike: "strike",
  caret: "caret",
  note: "comment",
  text: "text-box",
  ink: "ink",
  shape: "shapes",
  stamp: "stamp",
  attach: "paperclip",
  highlighter: "highlight",
  eraser: "eraser",
  lasso: "lasso",
};

/** Each shape's icon. */
export const SHAPE_ICONS: Readonly<Record<ShapeKind, IconName>> = {
  rectangle: "rectangle",
  ellipse: "ellipse",
  line: "line",
  arrow: "arrow",
  polygon: "polygon",
  polyline: "polyline",
};

/** A tool's command id: Text box ▾'s is its first text tool's. */
export const toolCommand = (tool: CommentTool): CommandId =>
  tool === "text"
    ? textCommand("textBox")
    : tool === "stamp"
      ? "comment.place-stamp"
      : tool === "attach"
        ? "comment.attach-file"
        : `comment.${tool}`;

/** The standard stamps the picker offers (C05), as `StampName`s, in its order. */
export const PICKER_STAMPS = [
  "approved",
  "notApproved",
  "reviewed",
  "draft",
  "final",
  "confidential",
  "forComment",
  "void",
] as const;

/** One of them. */
export type PickerStamp = (typeof PICKER_STAMPS)[number];

/** The dynamic stamps the picker offers: each a title, written with the name and time when placed. */
export const DYNAMIC_STAMPS = ["approved", "reviewed", "received", "notApproved"] as const;

/** One of them. */
export type DynamicStampName = (typeof DYNAMIC_STAMPS)[number];

/** A file attachment's icons (`/Name`), in the inspector's order. */
export const ATTACHMENT_ICONS = ["paperclip", "pushPin", "graph", "tag"] as const;

/** One of them. */
export type AttachmentIconName = (typeof ATTACHMENT_ICONS)[number];

/** The single key of the tools that have one (C01, C02). */
const KEYS: Partial<Record<CommentTool, string>> = { highlight: "H", underline: "U", ink: "P" };

/** The annotation commands of the inspector (C02). */
const ANNOT_COMMANDS = [
  "annot.set-colour",
  "annot.set-fill",
  "annot.set-width",
  "annot.set-opacity",
  "annot.set-border-style",
  "annot.set-line-ends",
  "annot.set-smoothing",
  "annot.delete",
] as const;

/** The text inspector's commands (C03): a text box's look, and leaving the box. */
const TEXT_COMMANDS = [
  "annot.toggle-bold",
  "annot.toggle-italic",
  "annot.set-alignment",
  "annot.set-font",
  "annot.set-font-size",
  "annot.done-editing",
] as const;

/** The attachment inspector's and the stamp library's commands (C05). */
const FILE_COMMANDS = ["attachment.save", "attachment.replace", "stamps.add-custom"] as const;

/** ⌘B and ⌘I while a box is typed into: the box takes the keys itself, so the bindings only show. */
const TEXT_KEYS: Partial<Record<(typeof TEXT_COMMANDS)[number], Shortcut[]>> = {
  "annot.toggle-bold": [{ mac: "Mod+B", other: "Mod+B", scope: "global", dispatch: false }],
  "annot.toggle-italic": [{ mac: "Mod+I", other: "Mod+I", scope: "global", dispatch: false }],
};

/** Whether the session can take an edit now, and the shell gave the command a handler (Comment is open). */
function available(id: CommandId) {
  return (ctx: CommandContext): true | string => {
    if (ctx.edits === undefined) return laterMessage("comment.unavailable");
    const blocked = blockedReason(ctx.edits.getState().blocked);
    if (blocked !== undefined) return blocked;
    return ctx.actions?.[id] === undefined ? laterMessage("comment.inComment") : true;
  };
}

/** A choice among `values`, each named by the later catalogue's `prefix` + value. */
function choices<Value extends string>(values: readonly Value[], prefix: string): ArgsSchema<Value> {
  const label = (value: Value) => laterMessage(`${prefix}${value}` as LaterMessageId);
  return {
    prompt: laterMessage("command.args.choice.prompt", { choices: values.map(label).join(", ") }),
    choices: values,
    parse: (text) => parseChoice(text, values, label),
  };
}

/** A number from `min` to `max`, in `unit`. */
function numberArgs(min: number, max: number, unit: string): ArgsSchema<number> {
  return {
    prompt: laterMessage("comment.args.number", { min, max, unit }),
    parse: (text) => {
      const value = Number.parseFloat(text);
      return Number.isFinite(value) && value >= min && value <= max
        ? { ok: true, value, summary: `${value} ${unit}` }
        : { ok: false, error: laterMessage("comment.args.number", { min, max, unit }) };
    },
  };
}

/** A delegated command of Comment mode. */
function commentCommand<Args>(
  id: CommandId,
  extra: { icon?: IconName; shortcuts?: Shortcut[]; args?: ArgsSchema<Args> | undefined; home: CommandHome[] },
): Command<Args> {
  return {
    id,
    title: laterMessage(`command.${id}` as LaterMessageId),
    keywords: ["comment", "annotate", "markup"],
    group: "edit",
    ...(extra.icon === undefined ? {} : { icon: extra.icon }),
    ...(extra.shortcuts === undefined ? {} : { shortcuts: extra.shortcuts }),
    ...(extra.args === undefined ? {} : { args: extra.args }),
    delegated: true,
    home: extra.home,
    enabled: available(id),
    run: (ctx, args) => ctx.actions?.[id]?.(args),
  };
}

/**
 * The interchange commands (C06, C07), which the Comments panel's More menu and the palette run:
 * export, import, flatten and the summary. Import and Flatten edit the document, so a blocked
 * session refuses them; the rest only read it, so any open document takes them.
 */
const INTERCHANGE_COMMANDS = [
  "comments.export",
  "comments.export-markdown",
  "comments.copy-markdown",
  "comments.import",
  "comments.flatten",
  "comments.summary",
  "comments.print-summary",
] as const;

/** The interchange commands that change the document. */
const INTERCHANGE_EDITS: ReadonlySet<CommandId> = new Set(["comments.import", "comments.flatten"]);

/** Whether a document is open and the shell gave command `id` a handler: the reading commands' availability. */
function readable(id: CommandId) {
  return (ctx: CommandContext): true | string => {
    if (ctx.edits === undefined) return laterMessage("comment.unavailable");
    return ctx.actions?.[id] === undefined ? laterMessage("comment.inComment") : true;
  };
}

/** One interchange command. ⌥⌘E exports highlights as Markdown on macOS only: Ctrl Alt E is AltGr on many layouts (C06). */
function interchangeCommand(id: (typeof INTERCHANGE_COMMANDS)[number]): Command<unknown> {
  const markdown = id === "comments.export-markdown";
  return {
    id,
    title: laterMessage(`command.${id}`),
    keywords: ["comment", "export", "import", "flatten", "summary", "xfdf", "fdf", "markdown", "review"],
    group: "edit",
    delegated: true,
    home: ["comments-panel"],
    ...(markdown
      ? { shortcuts: [{ mac: "Alt+Mod+E", other: "Alt+Mod+E", on: "mac" as const, scope: "global" as const }] }
      : {}),
    enabled: INTERCHANGE_EDITS.has(id) ? available(id) : readable(id),
    run: (ctx, args) => ctx.actions?.[id]?.(args),
  } as Command<unknown>;
}

/** The Comments panel's commands (C04): its toggle, its controls and what it does to a thread. */
function panelCommands(): Command<unknown>[] {
  return [
    commentCommand("comments.toggle-panel", { icon: "comments-list", home: ["comment-toolbar"] }),
    commentCommand("comments.search", { icon: "find", home: ["comment-toolbar"] }),
    commentCommand("comments.filter", { icon: "filter", home: ["comment-toolbar"] }),
    commentCommand("comments.sort", { home: ["comment-toolbar"] }),
    commentCommand("comments.delete-selected", { icon: "delete", home: ["comment-toolbar"] }),
    commentCommand("comment.go-to", { home: ["inspector"] }),
    commentCommand("comment.reply", { icon: "comment", home: ["inspector"] }),
    commentCommand("comment.set-status", { args: choices(STATUS_CHOICES, "comments.status."), home: ["inspector"] }),
  ];
}

/** Every command Comment mode registers. */
export function commentCommands(): Command<unknown>[] {
  const tools = [...COMMENT_TOOLS, ...TOUCH_TOOLS]
    .filter((tool) => tool !== "text")
    .map((tool) => {
      const key = KEYS[tool];
      return commentCommand(toolCommand(tool), {
        icon: TOOL_ICONS[tool],
        home: ["comment-toolbar"],
        ...(key === undefined ? {} : { shortcuts: [{ mac: key, other: key, scope: "page" }] }),
        ...(tool === "shape" ? { args: { ...choices(SHAPE_KINDS, "comment.shape."), optional: true } } : {}),
      });
    });
  // Draw with finger (C09) and Done (C08): the touch bars' own commands.
  const touch = [
    commentCommand("comment.draw-with-finger", { icon: "pen", home: ["comment-toolbar"] }),
    commentCommand("comment.done", { icon: "check", home: ["comment-toolbar"] }),
  ];
  const texts = TEXT_KINDS.map((kind) =>
    commentCommand(textCommand(kind), {
      icon: TEXT_ICONS[kind],
      home: ["comment-toolbar"],
      ...(kind === "textBox" ? { shortcuts: [{ mac: "T", other: "T", scope: "page" as const }] } : {}),
    }),
  );
  const colours = [...new Set<CommentColour>([...FILL_COLOURS, ...LINE_COLOURS])];
  return [
    {
      id: "mode.comment",
      title: laterMessage("command.mode.comment"),
      keywords: ["annotate", "markup", "highlight", "draw"],
      group: "view",
      icon: "comment",
      delegated: true,
      home: ["mode-switcher"],
      run: (ctx) => ctx.actions?.["mode.comment"]?.(undefined),
    },
    ...tools,
    ...touch,
    ...panelCommands(),
    ...INTERCHANGE_COMMANDS.map(interchangeCommand),
    ...texts,
    commentCommand("comment.set-colour", { args: choices(colours, "colour."), home: ["comment-toolbar"] }),
    commentCommand("comment.width", { args: numberArgs(0.5, 8, "pt"), home: ["comment-toolbar"] }),
    commentCommand("comment.set-author", { home: ["inspector"] }),
    ...ANNOT_COMMANDS.map((id) =>
      commentCommand<unknown>(id, {
        home: ["inspector"],
        ...(id === "annot.delete" ? { icon: "delete" as const } : {}),
        ...(id === "annot.set-colour" || id === "annot.set-fill" ? { args: choices(colours, "colour.") } : {}),
        ...(id === "annot.set-width" ? { args: numberArgs(0.5, 8, "pt") } : {}),
        ...(id === "annot.set-opacity" ? { args: numberArgs(0, 100, "%") } : {}),
        ...(id === "annot.set-border-style" ? { args: choices(BORDER_STYLES, "inspector.border.") } : {}),
      }),
    ),
    ...FILE_COMMANDS.map((id) =>
      commentCommand<unknown>(id, { home: [id === "stamps.add-custom" ? "comment-toolbar" : "inspector"] }),
    ),
    ...TEXT_COMMANDS.map((id) => {
      const shortcuts = TEXT_KEYS[id];
      return commentCommand<unknown>(id, {
        home: ["inspector"],
        ...(shortcuts === undefined ? {} : { shortcuts }),
        ...(id === "annot.set-alignment" ? { args: choices(TEXT_ALIGNMENTS, "inspector.align.") } : {}),
        ...(id === "annot.set-font" ? { args: choices(TEXT_FONTS, "inspector.font.") } : {}),
        ...(id === "annot.set-font-size" ? { args: numberArgs(1, 500, "pt") } : {}),
      });
    }),
  ] as Command<unknown>[];
}

/** Registers Comment mode's commands in `registry`; returns the function that removes them. */
export function registerCommentCommands(registry: CommandRegistry): () => void {
  const removers = commentCommands().map((command) => registry.register(command));
  return () => {
    for (const remove of removers) remove();
  };
}
