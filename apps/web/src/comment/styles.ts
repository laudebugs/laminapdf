// What Comment mode's tools remember on this device: each tool's colour, line width and (for a
// closed shape) fill. Kept in localStorage; where storage is blocked, a choice lasts the visit.
// The name comments carry is ../author.ts's.

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
export type StyleKey = Exclude<CommentTool, "shape" | "select" | "stamp" | "eraser" | "lasso"> | ShapeKind;

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
  text: { colour: "black", width: 1 },
  attach: { colour: "yellow", width: 1 },
  highlighter: { colour: "yellow", width: 12 },
};

/** How opaque a Highlighter stroke is: the page shows through it (C08). */
const HIGHLIGHTER_OPACITY = 0.4;

const STYLES_KEY = "lamina.comment.styles";
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

/** The style key of `tool` with `shape`. */
export function styleKey(tool: CommentTool, shape: ShapeKind): StyleKey | undefined {
  // A stamp draws in its own colours, and the Eraser and the Lasso draw nothing: none offered.
  if (tool === "select" || tool === "stamp" || tool === "eraser" || tool === "lasso") return undefined;
  return tool === "shape" ? shape : tool;
}

/** Each style key's colour family: fill colours for highlights and notes, line colours for the rest. */
export function familyOf(key: StyleKey): "fill" | "line" {
  return key === "highlight" || key === "note" || key === "attach" || key === "highlighter" ? "fill" : "line";
}

/** The tools that draw lines, and so take a width. */
export function takesWidth(key: StyleKey): boolean {
  return familyOf(key) === "line" && key !== "underline" && key !== "strike" && key !== "caret" && key !== "text";
}

/** The style the tool under `key` draws with. */
export function markStyle(styles: ToolStyles, key: StyleKey): MarkStyle {
  const style = styles[key];
  return {
    colour: colourValue(style.colour, familyOf(key)),
    width: style.width,
    opacity: key === "highlighter" ? HIGHLIGHTER_OPACITY : 1,
    ...(style.fill === undefined ? {} : { fill: colourValue(style.fill, "fill") }),
  };
}
