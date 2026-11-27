// The characters the symbol picker offers (the full-editor plan's A.12), in groups, each with the English name the
// picker's search matches. Every character here has a font among the bundled ones (a test keeps
// it so), and an inserted symbol the first font lacks takes the fallback chain's next font
// (`fallbackChain`). Left out for want of a font: ⇐ ⇒ ⇔ ↩ ↪ ∆ ₪ ⅓ ⅔, which Noto Sans Math or Noto
// Sans would add (issue #839). Bold and italic faces for the fonts are issue #838.

/** A symbol: the character and the words that find it. */
export interface SymbolEntry {
  readonly character: string;
  readonly name: string;
}

/** A group of symbols the picker lists under one heading. */
export interface SymbolGroup {
  readonly id: "punctuation" | "arrows" | "math" | "currency" | "marks" | "shapes" | "fractions";
  readonly symbols: readonly SymbolEntry[];
}

const s = (character: string, name: string): SymbolEntry => ({ character, name });

export const SYMBOL_GROUPS: readonly SymbolGroup[] = [
  {
    id: "punctuation",
    symbols: [
      s("©", "copyright"),
      s("®", "registered trade mark"),
      s("™", "trade mark"),
      s("§", "section"),
      s("¶", "paragraph pilcrow"),
      s("†", "dagger"),
      s("‡", "double dagger"),
      s("•", "bullet"),
      s("…", "ellipsis"),
      s("–", "en dash"),
      s("—", "em dash"),
      s("«", "left guillemet"),
      s("»", "right guillemet"),
      s("‹", "single left guillemet"),
      s("›", "single right guillemet"),
      s("“", "left double quotation mark"),
      s("”", "right double quotation mark"),
      s("‘", "left single quotation mark"),
      s("’", "right single quotation mark"),
      s("¿", "inverted question mark"),
      s("¡", "inverted exclamation mark"),
      s("№", "numero"),
      s("°", "degree"),
      s("′", "prime"),
      s("″", "double prime"),
    ],
  },
  {
    id: "arrows",
    symbols: [
      s("←", "left arrow"),
      s("↑", "up arrow"),
      s("→", "right arrow"),
      s("↓", "down arrow"),
      s("↔", "left right arrow"),
      s("↕", "up down arrow"),
      s("↗", "up right arrow"),
      s("↘", "down right arrow"),
      s("➜", "heavy right arrow"),
    ],
  },
  {
    id: "math",
    symbols: [
      s("±", "plus minus"),
      s("×", "multiplication times"),
      s("÷", "division"),
      s("−", "minus"),
      s("≈", "approximately equal"),
      s("≠", "not equal"),
      s("≤", "less than or equal"),
      s("≥", "greater than or equal"),
      s("∞", "infinity"),
      s("√", "square root"),
      s("∑", "sum"),
      s("∏", "product"),
      s("∂", "partial differential"),
      s("∫", "integral"),
      s("π", "pi"),
      s("Ω", "omega ohm"),
      s("µ", "micro"),
      s("‰", "per mille"),
    ],
  },
  {
    id: "currency",
    symbols: [
      s("€", "euro"),
      s("£", "pound"),
      s("¥", "yen yuan"),
      s("¢", "cent"),
      s("¤", "currency"),
      s("₹", "indian rupee"),
      s("₩", "won"),
      s("₽", "ruble"),
      s("₺", "turkish lira"),
      s("₿", "bitcoin"),
      s("₫", "dong"),
    ],
  },
  {
    id: "marks",
    symbols: [
      s("✓", "check mark"),
      s("✔", "heavy check mark"),
      s("✗", "ballot x"),
      s("✘", "heavy ballot x"),
      s("★", "black star"),
      s("☆", "white star"),
      s("♥", "heart"),
      s("♦", "diamond"),
      s("♠", "spade"),
      s("♣", "club"),
      s("☎", "telephone"),
      s("✉", "envelope"),
      s("✂", "scissors"),
      s("☐", "ballot box"),
      s("☑", "ballot box with check"),
      s("☒", "ballot box with x"),
    ],
  },
  {
    id: "shapes",
    symbols: [
      s("●", "black circle"),
      s("○", "white circle"),
      s("■", "black square"),
      s("□", "white square"),
      s("▲", "black up triangle"),
      s("△", "white up triangle"),
      s("▼", "black down triangle"),
      s("◆", "black diamond"),
      s("◇", "white diamond"),
      s("◀", "black left triangle"),
      s("▶", "black right triangle"),
    ],
  },
  {
    id: "fractions",
    symbols: [
      s("½", "one half"),
      s("¼", "one quarter"),
      s("¾", "three quarters"),
      s("¹", "superscript one"),
      s("²", "superscript two"),
      s("³", "superscript three"),
      s("⁰", "superscript zero"),
      s("₀", "subscript zero"),
      s("₁", "subscript one"),
      s("₂", "subscript two"),
    ],
  },
];

/** The symbols whose character or name contains `query`, ignoring case, in their groups. */
export function findSymbols(query: string): readonly SymbolEntry[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return [];
  return SYMBOL_GROUPS.flatMap((group) =>
    group.symbols.filter((symbol) => symbol.character === needle || symbol.name.includes(needle)),
  );
}
