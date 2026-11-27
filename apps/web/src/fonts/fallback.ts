// The fallback chain for text a font lacks characters of (the full-editor plan's A.12): the first font of an added text,
// then the fonts that have what it lacks, in the order the picker lists them. The core draws each
// character in the first font of the chain that has it, and refuses a character no font has
// (`content-glyph-missing`), so this works the chain out before the op is made and says which
// characters nothing covers.

import { covers } from "./ranges.ts";
import type { FontChoice } from "./registry.ts";

/** The fonts an added text needs, and what none of them draws. */
export interface FallbackPlan {
  /** The font IDs in order: the primary font first, then each fallback by when it was first needed. */
  readonly fonts: readonly string[];
  /** The characters no font in the registry has, each once, in the order they occur. */
  readonly missing: readonly string[];
}

/**
 * Characters the core asks no font for: control characters and hard line breaks, and the
 * default-ignorable ones (zero-width joiners and spaces, bidi controls, variation selectors,
 * soft hyphens, tags), which `needs_glyph` in the text layer skips. A combining mark isn't one: the
 * core wants a font that has it, so it is reported missing here too. The plain space is skipped
 * because every font has it and the core keeps it in the font it is in.
 */
function drawsNothing(codePoint: number): boolean {
  return (
    codePoint <= 0x20 ||
    (codePoint >= 0x7f && codePoint <= 0x9f) ||
    codePoint === 0x2028 ||
    codePoint === 0x2029 ||
    IGNORABLE.some(([first, last]) => codePoint >= first && codePoint <= last)
  );
}

/** The Unicode `Default_Ignorable_Code_Point` ranges. */
const IGNORABLE: readonly (readonly [number, number])[] = [
  [0x00ad, 0x00ad],
  [0x034f, 0x034f],
  [0x061c, 0x061c],
  [0x115f, 0x1160],
  [0x17b4, 0x17b5],
  [0x180b, 0x180f],
  [0x200b, 0x200f],
  [0x202a, 0x202e],
  [0x2060, 0x206f],
  [0x3164, 0x3164],
  [0xfe00, 0xfe0f],
  [0xfeff, 0xfeff],
  [0xffa0, 0xffa0],
  [0xfff0, 0xfff8],
  [0x1bca0, 0x1bca3],
  [0x1d173, 0x1d17a],
  [0xe0000, 0xe0fff],
];

/**
 * The chain for `text` set in `primary`, from `choices` (the registry's list): a standard font
 * can't be a fallback (the core embeds every fallback), so only bundled and added fonts follow,
 * the preferred ones (`FontChoice.preferred`) before the others, each group in the order the picker
 * lists them. A character the primary font has needs nothing; one it lacks takes the first fallback that has
 * it, which then covers every later character it has too.
 */
export function fallbackChain(choices: readonly FontChoice[], primary: string, text: string): FallbackPlan {
  const first = choices.find((font) => font.id === primary);
  const fonts = [primary];
  const missing: string[] = [];
  const others = choices.filter((font) => font.source !== "standard" && font.id !== primary);
  const candidates = [...others.filter((font) => font.preferred), ...others.filter((font) => !font.preferred)];
  for (const character of new Set([...text])) {
    const codePoint = character.codePointAt(0) ?? 0;
    if (drawsNothing(codePoint) || (first && covers(first.coverage, codePoint))) continue;
    // A font already in the chain that has it needs no new one.
    const chosen =
      candidates.find((font) => fonts.includes(font.id) && covers(font.coverage, codePoint)) ??
      candidates.find((font) => covers(font.coverage, codePoint));
    if (!chosen) {
      if (!missing.includes(character)) missing.push(character);
    } else if (!fonts.includes(chosen.id)) {
      fonts.push(chosen.id);
    }
  }
  return { fonts, missing };
}
