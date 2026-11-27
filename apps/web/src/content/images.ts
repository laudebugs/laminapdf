// Pictures for Edit mode (Paper A02): reading a file the picker, a drop or a paste gave, and the
// size it is placed at. JPEG and PNG only (the core takes no others; the desktop converts what its
// OS can decode, a later todo), so a file of another kind is named to the person, not dropped.

import { imageType } from "../comment/stamp-library.ts";
import { MARGIN } from "./geometry.ts";

/** A picture read from a file. */
export interface ReadImage {
  readonly name: string;
  readonly bytes: Uint8Array;
  readonly type: "image/png" | "image/jpeg";
  /** Its size in pixels. */
  readonly width: number;
  readonly height: number;
}

/** CSS pixels to points: a picture 96 px wide is 72 pt wide. */
const POINTS_PER_PIXEL = 0.75;

/** Reads `file` as a picture; undefined when it is not a JPEG or PNG the browser can decode. */
export async function readImage(file: {
  readonly name: string;
  arrayBuffer(): Promise<ArrayBuffer>;
}): Promise<ReadImage | undefined> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const type = imageType(bytes);
  if (type === undefined) return undefined;
  try {
    const bitmap = await createImageBitmap(new Blob([bytes], { type }));
    const size = { width: bitmap.width, height: bitmap.height };
    bitmap.close();
    return { name: file.name, bytes, type, ...size };
  } catch {
    return undefined;
  }
}

/**
 * The size, in points, to place `image` at on a page `page` wide and high: its own size in CSS
 * pixels, scaled down to fit between the margins when it is bigger, which `smaller` says.
 */
export function fittedSize(
  image: { readonly width: number; readonly height: number },
  page: { readonly width: number; readonly height: number },
): { readonly width: number; readonly height: number; readonly smaller: boolean } {
  const width = image.width * POINTS_PER_PIXEL;
  const height = image.height * POINTS_PER_PIXEL;
  const room = { width: Math.max(page.width - 2 * MARGIN, 1), height: Math.max(page.height - 2 * MARGIN, 1) };
  const scale = Math.min(1, room.width / width, room.height / height);
  return { width: width * scale, height: height * scale, smaller: scale < 1 };
}

/** The name of a file without its extension: a picture's first alternative text. */
export function stemOf(name: string): string {
  return name.replace(/\.[^.]*$/, "");
}

/** The pictures among the files of a drop or a paste. */
export function pictureFiles(files: FileList | readonly File[] | null | undefined): File[] {
  return [...(files ?? [])].filter(
    (file) => file.type.startsWith("image/") || /\.(png|jpe?g|heic|gif|webp|tiff?|bmp)$/i.test(file.name),
  );
}
