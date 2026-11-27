// The fallback chain for text a font lacks characters of (A.12): the first font of an added text,
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

/** Characters no font draws a glyph for: spaces, line breaks and control characters. */
function drawsNothing(codePoint: number): boolean {
  return codePoint <= 0x20 || (codePoint >= 0x7f && codePoint <= 0xa0) || codePoint === 0x200b || codePoint === 0xfeff;
}

/**
 * The chain for `text` set in `primary`, from `choices` (the registry's list): a standard font
 * can't be a fallback (the core embeds every fallback), so only bundled and added fonts follow.
 * A character the primary font has needs nothing; one it lacks takes the first fallback that has
 * it, which then covers every later character it has too.
 */
export function fallbackChain(choices: readonly FontChoice[], primary: string, text: string): FallbackPlan {
  const first = choices.find((font) => font.id === primary);
  const fonts = [primary];
  const missing: string[] = [];
  const candidates = choices.filter((font) => font.source !== "standard" && font.id !== primary);
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
