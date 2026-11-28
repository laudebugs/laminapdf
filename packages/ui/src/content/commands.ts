// Edit mode's commands for adding content (Paper A01, A02): the mode, its four tools (Select, Add
// text, Image, Shape ▾), Insert symbol… and the actions on a selected object: Replace image…,
// Duplicate, Delete and the four arrange commands, and the page marks commands (Page marks ▾). Each is delegated: Edit mode registers its
// handler while it is on, so the palette lists them with a reason otherwise. Single-key shortcuts
// (V and T in the design) wait for the shortcut scopes to tell Edit mode from Comment mode, whose
// `T` they would meet.

import type { IconName } from "@laminapdf/icons";
import { parseChoice } from "../commands/args.js";
import { delegatedCommand } from "../commands/delegated.js";
import type { CommandRegistry } from "../commands/registry.js";
import type { ArgsSchema, Command, CommandHome, CommandId } from "../commands/types.js";
import { laterMessage } from "../later-messages.js";

/** The tools of Edit mode. */
export const CONTENT_TOOLS = ["select", "text", "image", "shape"] as const;

/** One of them. */
export type ContentTool = (typeof CONTENT_TOOLS)[number];

/** The shapes the Shape tool draws. */
export const CONTENT_SHAPES = ["line", "rect", "ellipse", "polygon"] as const;

/** One of them. */
export type ContentShape = (typeof CONTENT_SHAPES)[number];

/** Each tool's icon. */
export const CONTENT_TOOL_ICONS: Readonly<Record<ContentTool, IconName>> = {
  select: "select",
  text: "add-text",
  image: "image",
  shape: "shapes",
};

/** Each shape's icon. */
export const CONTENT_SHAPE_ICONS: Readonly<Record<ContentShape, IconName>> = {
  line: "line",
  rect: "rectangle",
  ellipse: "ellipse",
  polygon: "polygon",
};

/** The command that turns a tool on. */
export const contentToolCommand = (tool: ContentTool): CommandId =>
  `tool.${tool === "text" ? "add-text" : tool}` as CommandId;

/** The colours content takes (Paper A01: Ink, Grey, Teal, Red), as the design language names them, with their values. */
export const CONTENT_COLOURS = [
  { id: "ink", token: "content-page-ink", rgb: [38 / 255, 36 / 255, 33 / 255] },
  { id: "grey", token: "content-page-grey", rgb: [110 / 255, 106 / 255, 100 / 255] },
  { id: "teal", token: "content-page-teal", rgb: [31 / 255, 92 / 255, 99 / 255] },
  { id: "red", token: "content-page-red", rgb: [155 / 255, 35 / 255, 53 / 255] },
] as const;

/** A content colour's name. */
export type ContentColour = (typeof CONTENT_COLOURS)[number]["id"];

/** The sizes the text size select offers, in points. */
export const CONTENT_TEXT_SIZES = [8, 9, 10, 11, 12, 14, 16, 18, 24, 36, 48, 72] as const;

/** An outline's pattern. */
export const CONTENT_DASHES = ["solid", "dashed", "dotted"] as const;

/** The arrange commands, backmost last. */
const ARRANGE_COMMANDS = [
  "edit.bring-to-front",
  "edit.bring-forward",
  "edit.send-backward",
  "edit.send-to-back",
] as const;

/** A delegated command of Edit mode. */
function contentCommand<Args>(
  id: CommandId,
  extra: { icon?: IconName; home: CommandHome[]; args?: ArgsSchema<Args>; keywords?: readonly string[] },
): Command<Args> {
  return delegatedCommand(id, {
    ...extra,
    keywords: ["edit", "add", "content", ...(extra.keywords ?? [])],
    noSession: "content.unavailable",
    notInMode: "content.inEdit",
  });
}

/** Every command Edit mode registers. */
export function contentCommands(): Command<unknown>[] {
  const label = (value: ContentShape) => laterMessage(`content.shape.${value}`);
  const shapeArgs: ArgsSchema<ContentShape> = {
    prompt: laterMessage("command.args.choice.prompt", { choices: CONTENT_SHAPES.map(label).join(", ") }),
    choices: CONTENT_SHAPES,
    optional: true,
    parse: (text) => parseChoice(text, CONTENT_SHAPES, label),
  };
  return [
    {
      id: "mode.edit",
      title: laterMessage("command.mode.edit"),
      keywords: ["edit", "add", "text", "image", "shape", "content"],
      group: "view",
      icon: "edit-text",
      delegated: true,
      home: ["mode-switcher"],
      run: (ctx) => ctx.actions?.["mode.edit"]?.(undefined),
    },
    contentCommand("tool.select", { icon: "select", home: ["content-toolbar"], keywords: ["pointer", "move"] }),
    contentCommand("tool.add-text", { icon: "add-text", home: ["content-toolbar"], keywords: ["text box", "type"] }),
    contentCommand("tool.image", {
      icon: "image",
      home: ["content-toolbar"],
      keywords: ["picture", "logo", "photo", "place"],
    }),
    contentCommand("tool.shape", {
      icon: "shapes",
      home: ["content-toolbar"],
      args: shapeArgs,
      keywords: ["line", "rectangle", "ellipse", "circle", "polygon", "draw"],
    }),
    contentCommand("edit.insert-symbol", { icon: "add-text", home: ["inspector"], keywords: ["character", "special"] }),
    contentCommand("edit.replace-image", { icon: "replace", home: ["inspector"], keywords: ["swap", "picture"] }),
    contentCommand("edit.duplicate", { icon: "copy", home: ["inspector"], keywords: ["copy", "clone"] }),
    contentCommand("edit.delete-object", { icon: "delete", home: ["inspector"], keywords: ["remove"] }),
    contentCommand("edit.add-watermark", {
      icon: "page-marks",
      home: ["content-toolbar"],
      keywords: ["watermark", "draft", "confidential", "background", "stamp"],
    }),
    contentCommand("edit.add-header-footer", {
      icon: "page-marks",
      home: ["content-toolbar"],
      keywords: ["header", "footer", "page numbers", "numbering", "date", "page number"],
    }),
    contentCommand("edit.add-bates", {
      icon: "page-marks",
      home: ["content-toolbar"],
      keywords: ["bates", "numbers", "legal", "stamp", "page numbers"],
    }),
    contentCommand("edit.manage-page-marks", {
      icon: "page-marks",
      home: ["content-toolbar"],
      keywords: ["watermarks", "headers", "footers", "remove", "edit", "marks"],
    }),
    ...ARRANGE_COMMANDS.map((id) =>
      contentCommand(id, { icon: "layers", home: ["inspector"], keywords: ["order", "stack"] }),
    ),
  ] as Command<unknown>[];
}

/** Registers Edit mode's commands in `registry`; returns the function that removes them. */
export function registerContentCommands(registry: CommandRegistry): () => void {
  const removers = contentCommands().map((command) => registry.register(command));
  return () => {
    for (const remove of removers) remove();
  };
}
