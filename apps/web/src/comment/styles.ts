// What Comment mode's tools remember on this device: each tool's colour, line width and (for a
// closed shape) fill, and the name comments carry. Kept in localStorage; where storage is blocked,
// a choice lasts the visit.

import type { CommentColour, CommentTool, ShapeKind } from "@laminapdf/ui/editing";
import { colourValue, type MarkStyle } from "./marks.ts";

/** What a tool remembers: its colour and line width, and a closed shape's fill. */
export interface ToolStyle {
  readonly colour: CommentColour;
  readonly width: number;
  /** A shape's fill, set from the inspector; none without. */
  readonly fill?: CommentColour;
}

/** The key a style is remembered under: a tool, or a shape for Shapes. */
export type StyleKey = Exclude<CommentTool, "shape" | "select"> | ShapeKind;

/** Every tool's style. */
export type ToolStyles = Readonly<Record<StyleKey, ToolStyle>>;

const DEFAULT_STYLES: ToolStyles = {
  highlight: { colour: "yellow", width: 1 },
  underline: { colour: "green", width: 1 },
  strike: { colour: "red", width: 1 },
  caret: { colour: "blue", width: 1 },
  note: { colour: "yellow", width: 1 },
  ink: { colour: "blue", width: 2 },
  rectangle: { colour: "blue", width: 1.5 },
  ellipse: { colour: "blue", width: 1.5 },
  line: { colour: "black", width: 1.5 },
  arrow: { colour: "red", width: 1.5 },
  polygon: { colour: "blue", width: 1.5 },
  polyline: { colour: "black", width: 1.5 },
};

const STYLES_KEY = "lamina.comment.styles";
const AUTHOR_KEY = "lamina.comment.author";

/** Reads `key` from this device's storage; nothing where storage is blocked. */
function stored(key: string): string | undefined {
  try {
    return localStorage.getItem(key) ?? undefined;
  } catch {
    return undefined;
  }
}

function store(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage is blocked: the choice lasts this visit.
  }
}

/** The styles kept on this device, over the defaults. */
export function storedStyles(): ToolStyles {
  try {
    return { ...DEFAULT_STYLES, ...(JSON.parse(stored(STYLES_KEY) ?? "{}") as Partial<ToolStyles>) };
  } catch {
    return DEFAULT_STYLES;
  }
}

/** Keeps `styles` on this device. */
export function storeStyles(styles: ToolStyles): void {
  store(STYLES_KEY, JSON.stringify(styles));
}

/** The name comments carry: undefined until asked, empty when the person left it off. */
export function storedAuthor(): string | undefined {
  return stored(AUTHOR_KEY);
}

/** Keeps the name comments carry. */
export function storeAuthor(name: string): void {
  store(AUTHOR_KEY, name.trim());
}

/** The style key of `tool` with `shape`. */
export function styleKey(tool: CommentTool, shape: ShapeKind): StyleKey | undefined {
  if (tool === "select") return undefined;
  return tool === "shape" ? shape : tool;
}

/** Each style key's colour family: fill colours for highlights and notes, line colours for the rest. */
export function familyOf(key: StyleKey): "fill" | "line" {
  return key === "highlight" || key === "note" ? "fill" : "line";
}

/** The tools that draw lines, and so take a width. */
export function takesWidth(key: StyleKey): boolean {
  return familyOf(key) === "line" && key !== "underline" && key !== "strike" && key !== "caret";
}

/** The style the tool under `key` draws with. */
export function markStyle(styles: ToolStyles, key: StyleKey): MarkStyle {
  const style = styles[key];
  return {
    colour: colourValue(style.colour, familyOf(key)),
    width: style.width,
    opacity: 1,
    ...(style.fill === undefined ? {} : { fill: colourValue(style.fill, "fill") }),
  };
}
