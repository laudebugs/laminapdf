// Reader mode's settings (R14): the text size, line spacing and line length the reading column
// takes, kept by the app. Apart from the column itself (reader-view.tsx), so an app reads and
// keeps them without loading the column, which waits until reader mode turns on (#380).

/** How far apart a paragraph's lines are. */
export type ReaderLineSpacing = "tight" | "normal" | "loose";

/** How wide the column is: about 55, 68 or 85 characters. */
export type ReaderLineLength = "narrow" | "medium" | "wide";

/** The reader's settings: global, kept by the app. */
export interface ReaderSettings {
  /** The text size in CSS px, from {@link READER_TEXT_SIZES}' first to its last. */
  readonly textSize: number;
  readonly lineSpacing: ReaderLineSpacing;
  readonly lineLength: ReaderLineLength;
}

/** The smallest and largest text size, in CSS px. */
export const READER_TEXT_SIZES = [14, 32] as const;

/** The settings Reset brings back. */
export const DEFAULT_READER_SETTINGS: ReaderSettings = { textSize: 19, lineSpacing: "normal", lineLength: "medium" };

/** `size` kept between the smallest and largest text size, in whole pixels. */
export function readerTextSize(size: number): number {
  const [least, most] = READER_TEXT_SIZES;
  return Math.min(most, Math.max(least, Math.round(size)));
}
