// The canvas half of making a signature image draws a signature's strokes, a typed name
// or an uploaded picture into a PNG. capture.ts holds the parts that need no DOM.

import {
  type Drawing,
  inkCss,
  removeWhite,
  type Stroke,
  TYPED_STYLES,
  type TypedStyle,
  typedText,
  uploadProblem,
} from "./capture.ts";

/** A signature's picture: PNG bytes and their size in pixels. */
export interface Picture {
  readonly bytes: Uint8Array;
  readonly width: number;
  readonly height: number;
}

/** The pen width on the pad and in the PNG, in pixels. */
export const PEN = 2.4;
/** The scale a PNG is drawn at, so it stays sharp when placed at the size of a field. */
const SCALE = 2;

function canvasOf(width: number, height: number): [HTMLCanvasElement, CanvasRenderingContext2D] {
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(width * SCALE));
  canvas.height = Math.max(1, Math.round(height * SCALE));
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("no canvas");
  context.scale(SCALE, SCALE);
  return [canvas, context];
}

async function pngOf(canvas: HTMLCanvasElement, width: number, height: number): Promise<Picture> {
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (blob === null) throw new Error("no picture");
  return { bytes: new Uint8Array(await blob.arrayBuffer()), width, height };
}

/** Draws `strokes` on `context` in `ink`, a smooth line through each stroke's points. */
export function drawStrokes(
  context: CanvasRenderingContext2D,
  strokes: readonly Stroke[],
  ink: readonly [number, number, number],
): void {
  context.strokeStyle = inkCss(ink);
  context.fillStyle = inkCss(ink);
  context.lineWidth = PEN;
  context.lineCap = "round";
  context.lineJoin = "round";
  for (const stroke of strokes) {
    const first = stroke[0];
    if (first === undefined) continue;
    context.beginPath();
    context.moveTo(first[0], first[1]);
    if (stroke.length === 1) {
      // A dot: a zero-length line draws nothing, so fill a circle.
      context.arc(first[0], first[1], PEN / 2, 0, Math.PI * 2);
      context.fill();
      continue;
    }
    for (const [x, y] of stroke.slice(1)) context.lineTo(x, y);
    context.stroke();
  }
}

/** A drawn signature as a transparent PNG the size of its cropped drawing. */
export async function drawingToPng(drawing: Drawing, ink: readonly [number, number, number]): Promise<Picture> {
  const [canvas, context] = canvasOf(drawing.width, drawing.height);
  drawStrokes(context, drawing.strokes, ink);
  return pngOf(canvas, drawing.width, drawing.height);
}

/** A typed signature as a transparent PNG; undefined for a blank name. */
export async function typedToPng(
  name: string,
  style: TypedStyle,
  ink: readonly [number, number, number],
): Promise<Picture | undefined> {
  const text = typedText(name, style);
  if (text === "") return undefined;
  const font = `${style.italic ? "italic " : ""}${style.size}px ${style.family}`;
  const [measure, mc] = canvasOf(1, 1);
  mc.font = font;
  mc.letterSpacing = `${style.spacing}px`;
  const width = Math.ceil(mc.measureText(text).width) + 24;
  const height = Math.ceil(style.size * 1.5);
  measure.remove();
  const [canvas, context] = canvasOf(width, height);
  context.font = font;
  context.letterSpacing = `${style.spacing}px`;
  context.fillStyle = inkCss(ink);
  context.textBaseline = "alphabetic";
  context.fillText(text, 12, style.size * 1.05);
  return pngOf(canvas, width, height);
}

/** An uploaded PNG or JPEG as a PNG with its white background removed; undefined when it can't be drawn. */
export async function imageToPng(bytes: Uint8Array): Promise<Picture | undefined> {
  if (uploadProblem(bytes) !== undefined) return undefined;
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(new Blob([new Uint8Array(bytes)]));
  } catch {
    return undefined;
  }
  // A long edge of 1,200 pixels is ample for a signature and keeps the library small.
  const shrink = Math.min(1, 1200 / Math.max(bitmap.width, bitmap.height));
  const width = Math.max(1, Math.round(bitmap.width * shrink));
  const height = Math.max(1, Math.round(bitmap.height * shrink));
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (context === null) return undefined;
  context.drawImage(bitmap, 0, 0, width, height);
  bitmap.close();
  const pixels = context.getImageData(0, 0, width, height);
  removeWhite(pixels.data);
  context.putImageData(pixels, 0, 0);
  return pngOf(canvas, width, height);
}

/** The preview of a typed name in every style, for the Type tab's radio cards. */
export function typedPreviews(name: string): readonly { style: TypedStyle; text: string }[] {
  return TYPED_STYLES.map((style) => ({ style, text: typedText(name, style) }));
}
