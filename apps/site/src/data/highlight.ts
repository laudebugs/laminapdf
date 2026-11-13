// Code colouring for the home page's code panel, at build time, by the sites' shared rules
// (@laminapdf/site-kit/code-kinds): five kinds, which site.css colours with the --paper-code-*
// tokens. Here, a console's `$ ` prompts. The page ships no script for this.

import { type CodeToken, joined, tokenize } from "@laminapdf/site-kit/code-kinds";

/** The languages the panel shows: the two examples, and a console of `$ ` commands in bash. */
export type CodeLanguage = "ts" | "rust" | "console";

/** A console line's prompt, coloured as a comment: it is not part of the command. */
const PROMPT = "$ ";

/**
 * `code`'s lines as runs of text; neighbouring runs of the same kind are joined. In a console, a
 * line that starts with `$ ` is a prompt and a bash command; any other line is output, left plain.
 */
export async function highlight(code: string, lang: CodeLanguage): Promise<CodeToken[][]> {
  if (lang !== "console") return tokenize(code, lang);
  const lines = code.split("\n");
  const commands = await tokenize(
    lines.map((line) => (line.startsWith(PROMPT) ? line.slice(PROMPT.length) : "")).join("\n"),
    "bash",
  );
  return lines.map((line, index) => {
    if (!line.startsWith(PROMPT)) return line === "" ? [] : [{ text: line }];
    return joined([{ text: PROMPT, kind: "comment" }, ...(commands[index] ?? [])]);
  });
}
