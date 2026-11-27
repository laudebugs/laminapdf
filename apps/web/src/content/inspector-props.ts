// What the inspector shows of the selected object (Paper A01, A02), and what each of its fields
// does: one change, one op, one undo step. Apart from the DOM, so the facts it reads off an object
// (its position, its colour, the font note) can be tested in Node.

import type {
  ContentChoice,
  ContentFontOption,
  ContentGeometry,
  ContentImageFields,
  ContentInspectorProps,
  ContentShapeFields,
} from "@laminapdf/ui/editing";
import { CONTENT_COLOURS, type ContentColour, laterMessage } from "@laminapdf/ui/editing";
import { fallbackChain } from "../fonts/fallback.ts";
import { covers } from "../fonts/ranges.ts";
import type { FontChoice } from "../fonts/registry.ts";
import type { ContentController } from "./controller.ts";
import { centreOf, frameOf, type Matrix, sizeOf } from "./geometry.ts";
import type { Item } from "./objects.ts";
import { nameOf } from "./objects.ts";
import { colour, type Rgb, rgbOf, strokeOf, textOf } from "./ops.ts";
import type { Look } from "./text-look.ts";

/** The content colour nearest `rgb`, when one is close (a colour this reader wrote). */
export function colourIdOf(rgb: Rgb | undefined): ContentColour | undefined {
  if (rgb === undefined) return undefined;
  let best: { id: ContentColour; distance: number } | undefined;
  for (const c of CONTENT_COLOURS) {
    const distance = Math.hypot(c.rgb[0] - rgb[0], c.rgb[1] - rgb[1], c.rgb[2] - rgb[2]);
    if (best === undefined || distance < best.distance) best = { id: c.id, distance };
  }
  return best !== undefined && best.distance < 0.03 ? best.id : undefined;
}

/** A content colour's value. */
export function rgbOfColour(id: ContentColour): Rgb {
  return CONTENT_COLOURS.find((c) => c.id === id)?.rgb ?? CONTENT_COLOURS[0].rgb;
}

/** The characters of `text` that `font` lacks, each once, at most `limit`. */
export function lacking(font: FontChoice | undefined, text: string, limit = 6): string[] {
  if (font === undefined) return [];
  const out: string[] = [];
  for (const character of new Set([...text])) {
    const code = character.codePointAt(0) ?? 0;
    if (code <= 0x20 || covers(font.coverage, code)) continue;
    out.push(character);
    if (out.length === limit) break;
  }
  return out;
}

/** "660 KB" for a size in bytes. */
export function sizeWords(bytes: number): string {
  return bytes >= 1_000_000 ? `${(bytes / 1_000_000).toFixed(1)} MB` : `${Math.round(bytes / 1000)} KB`;
}

/** The fonts the select offers, with what fetching a bundled one costs. */
export function fontOptions(controller: ContentController, current: string | undefined): ContentFontOption[] {
  const registry = controller.registry;
  const options: ContentFontOption[] = controller.fonts().map((font) => {
    const pending = registry.toFetch([font.id]).length > 0;
    return {
      id: font.id,
      label: font.family,
      ...(pending && font.bytes !== undefined && font.bytes > 100_000
        ? { note: laterMessage("content.font.download", { size: sizeWords(font.bytes) }) }
        : {}),
    };
  });
  if (current === undefined) options.unshift({ id: "file", label: laterMessage("inspector.font.other") });
  return options;
}

/** The note under the font select: where the font is from, and what falls back. */
function fontNote(controller: ContentController, fontId: string | undefined, text: string): string | undefined {
  const font = fontId === undefined ? undefined : controller.registry.get(fontId);
  if (font === undefined) return undefined;
  const base = laterMessage(
    font.source === "standard"
      ? "content.font.standard"
      : font.source === "bundled"
        ? "content.font.bundled"
        : "content.font.added",
  );
  const missing = lacking(font, text);
  if (missing.length === 0 || font.source === "standard" || controller.fonts().length < 2) return base;
  const next = fallbackChain(controller.fonts(), font.id, text).fonts[1];
  const name = next === undefined ? undefined : controller.registry.get(next)?.family;
  if (name === undefined) return base;
  return laterMessage("content.font.fallback", { characters: missing.map((c) => `“${c}”`).join(" "), font: name });
}

/** A choice over a value, applied by `set`. */
function choice<Value>(value: Value, onChange: (value: Value) => void): ContentChoice<Value> {
  return { value, onChange };
}

/** X, Y, W, H and the turn of a frame, as the fields show them: the box it has unturned, about its centre. */
export function geometryOf(frame: Matrix) {
  const { width, height, degrees } = sizeOf(frame);
  const centre = centreOf(frame);
  return { x: centre.x - width / 2, y: centre.y - height / 2, width, height, rotate: degrees };
}

/** What the inspector needs besides the object. */
export interface InspectorOptions {
  readonly keepRatio: boolean;
  /** Changes when the session learns something a field shows (a font fetched, a picture placed), so the fields are made again. */
  readonly known: number;
  readonly locked: string | undefined;
  readonly addFont: (() => void) | undefined;
}

/** The inspector's props for `item`. */
export function inspectorProps(
  controller: ContentController,
  item: Item,
  options: InspectorOptions,
): ContentInspectorProps {
  const base: ContentInspectorProps = {
    title: laterMessage(`content.${nameOf(item)}` as "content.object.text"),
    context: laterMessage("content.context", { page: item.page + 1 }),
    geometry: geometryFields(controller, item, options.keepRatio),
    opacity: choice(Math.round(item.added.opacity * 100), (percent) => controller.change({ opacity: percent / 100 })),
    arrange: {
      onFront: () => controller.arrangeSelection("front"),
      onForward: () => controller.arrangeSelection("forward"),
      onBackward: () => controller.arrangeSelection("backward"),
      onBack: () => controller.arrangeSelection("back"),
    },
    onDuplicate: () => controller.duplicateSelection(),
    deleteLabel: laterMessage(`content.delete.${item.kind}`),
    onDelete: () => controller.deleteSelection(),
    ...(options.locked === undefined ? {} : { locked: options.locked }),
  };
  const spec = item.added.object;
  if (spec === undefined) return base;
  if ("text" in spec) return { ...base, text: textFields(controller, item, spec.text, options.addFont) };
  const alt = altFields(controller, item.added.alt as unknown as { text: string } | "decorative" | undefined);
  if ("shape" in spec) return { ...base, shape: shapeFields(controller, spec.shape), alt };
  return { ...base, image: imageFields(controller, item), alt };
}

/** Position, size and turn: what the handles do, typed. */
function geometryFields(controller: ContentController, item: Item, keepRatio: boolean): ContentGeometry {
  const spec = item.added.object;
  const g = geometryOf(item.frame);
  const onChange: ContentGeometry["onChange"] = (change) => {
    const next = { ...g, ...change };
    if (item.kind === "text") {
      // A text box's height follows its text: only where it is and how wide.
      const block =
        spec !== undefined && "text" in spec && next.width !== g.width
          ? { ...spec.text, width: next.width }
          : undefined;
      void controller.setFrame(
        item,
        frameOf(next.x, next.y, next.width, g.height),
        "move",
        block === undefined ? {} : { text: block },
      );
      return;
    }
    const step =
      change.rotate !== undefined
        ? "turn"
        : change.width !== undefined || change.height !== undefined
          ? "resize"
          : "move";
    void controller.setFrame(item, frameOf(next.x, next.y, next.width, next.height, next.rotate), step);
  };
  return {
    ...g,
    keepRatio,
    textBox: item.kind === "text",
    onChange,
    onKeepRatio: (keep) => controller.setKeepRatio(keep),
  };
}

/** A shape's fill and outline. */
function shapeFields(
  controller: ContentController,
  shape: Extract<NonNullable<Item["added"]["object"]>, { shape: unknown }>["shape"],
): ContentShapeFields {
  const { fill, stroke } = shape;
  const strokeRgb = (stroke === undefined ? undefined : rgbOf(stroke.color)) ?? rgbOfColour("ink");
  const outline = (change: Partial<{ color: Rgb; width: number; dash: "solid" | "dashed" | "dotted" }>) =>
    controller.change({
      stroke: strokeOf({ color: strokeRgb, width: stroke?.width ?? 1.5, dash: stroke?.dash ?? "solid", ...change }),
    });
  return {
    fill: choice(fill === undefined ? "none" : colourIdOf(rgbOf(fill)), (id) => {
      if (id !== undefined) controller.change({ fill: id === "none" ? null : colour(rgbOfColour(id)) });
    }),
    stroke: choice(stroke === undefined ? "none" : colourIdOf(rgbOf(stroke.color)), (id) => {
      if (id === "none") controller.change({ stroke: null });
      else if (id !== undefined) outline({ color: rgbOfColour(id) });
    }),
    dash: choice(stroke?.dash ?? "solid", (dash) => outline({ dash })),
    width: choice(stroke?.width ?? 1.5, (width) => outline({ width })),
  };
}

/** A picture's file line and Replace. */
function imageFields(controller: ContentController, item: Item): ContentImageFields {
  const facts = controller.factsOf(item.id);
  const file =
    facts === undefined
      ? laterMessage("content.object.image")
      : laterMessage("content.file", {
          name: facts.name,
          width: facts.width,
          height: facts.height,
          type: laterMessage(facts.type === "image/png" ? "content.file.png" : "content.file.jpeg"),
        });
  return {
    file,
    onReplace: () => void controller.pickImage(item),
    ...(facts?.smaller === true ? { note: laterMessage("content.image.small") } : {}),
  };
}

/** The alternative text fields: the text, or decoration. */
function altFields(controller: ContentController, alt: { text: string } | "decorative" | undefined) {
  const decorative = alt === "decorative";
  return {
    text: alt === undefined || decorative ? "" : alt.text,
    decorative,
    onText: (text: string) => controller.change({ alt: { text } as never }),
    onDecorative: (on: boolean) => controller.change({ alt: (on ? "decorative" : { text: "" }) as never }),
  };
}

/** A text box's fields. */
function textFields(
  controller: ContentController,
  item: Item,
  block: Extract<NonNullable<Item["added"]["object"]>, { text: unknown }>["text"],
  addFont: (() => void) | undefined,
) {
  const text = textOf(block);
  const known =
    controller.fontOf(item.id) ??
    ((["helvetica", "times", "courier"] as string[]).includes(block.fonts?.[0] ?? "") ? block.fonts?.[0] : undefined);
  const align = (block.align ?? "left") as Look["align"];
  const rgb = rgbOf(block.color) ?? rgbOfColour("ink");
  return textLook(controller, {
    fontId: known,
    size: block.size ?? 12,
    bold: block.runs.every((r) => r.bold === true),
    italic: block.runs.every((r) => r.italic === true),
    align,
    colour: colourIdOf(rgb),
    text,
    apply: (change) => void controller.changeText(change),
    addFont,
  });
}

/** The text fields over a look and what changes it: a text box's own, or the next new one's. */
export function textLook(
  controller: ContentController,
  look: {
    readonly fontId: string | undefined;
    readonly size: number;
    readonly bold: boolean;
    readonly italic: boolean;
    readonly align: Look["align"];
    readonly colour: ContentColour | undefined;
    readonly text: string;
    readonly apply: (change: Partial<Look>) => void;
    readonly addFont: (() => void) | undefined;
  },
) {
  return {
    font: {
      value: look.fontId ?? "file",
      options: fontOptions(controller, look.fontId),
      onChange: (id: string) => id !== "file" && look.apply({ fontId: id }),
    },
    size: choice(look.size, (size) => look.apply({ size })),
    bold: choice(look.bold, (bold) => look.apply({ bold })),
    italic: choice(look.italic, (italic) => look.apply({ italic })),
    align: choice(look.align, (align) => look.apply({ align })),
    colour: choice(look.colour, (id) => id !== undefined && look.apply({ color: rgbOfColour(id) })),
    fontNote: fontNote(controller, look.fontId, look.text),
    onInsertSymbol: () => controller.pickSymbol(true),
    ...(look.addFont === undefined ? {} : { onAddFont: look.addFont }),
  };
}
