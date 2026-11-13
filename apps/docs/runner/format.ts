// Format in "Edit and run" (todo Q4r): the reader's TypeScript through Prettier, loaded on the first
// press (its TypeScript parser is about 300 KB gzipped), never before. The options are the
// repository's Biome settings (biome.json: 2 spaces, 120 columns, double quotes, semicolons,
// trailing commas), at which Prettier leaves every runnable example exactly as it is written.

/** The formatted code and where the cursor goes in it. */
export interface Formatted {
  readonly code: string;
  readonly cursor: number;
}

/**
 * `code` formatted, with `cursor` (an offset into it) moved to the same place in the result.
 *
 * @throws Prettier's error when the code doesn't parse; its message names the line and column.
 */
export async function formatCode(code: string, cursor: number): Promise<Formatted> {
  const [prettier, typescript, estree] = await Promise.all([
    import("prettier/standalone"),
    import("prettier/plugins/typescript"),
    import("prettier/plugins/estree"),
  ]);
  const result = await prettier.formatWithCursor(code, {
    cursorOffset: cursor,
    parser: "typescript",
    plugins: [typescript, estree],
    printWidth: 120,
    tabWidth: 2,
    useTabs: false,
    semi: true,
    singleQuote: false,
    trailingComma: "all",
  });
  return { code: result.formatted, cursor: result.cursorOffset };
}

/** The first line of a formatter's error: Prettier adds a code frame after it. */
export function firstLine(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return message.split("\n", 1)[0] ?? message;
}
