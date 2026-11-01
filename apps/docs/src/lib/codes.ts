// fixtures/codes.json, read once for the site: the groups the code table shows and the entry each
// code's page names. A group added to codes.json (any key matching the schema's group pattern, read
// from fixtures/codes.schema.json) appears without a change here.
import codes from "../../../../fixtures/codes.json";
import schema from "../../../../fixtures/codes.schema.json";

/** One code's entry in codes.json. */
export interface CodeEntry {
  readonly code: string;
  readonly kind: string;
  readonly summary: string;
  readonly hint: string;
  readonly since: string;
}

/** One group of codes (`warnings`, `textErrors`, …), with its table heading. */
export interface CodeGroup {
  readonly key: string;
  readonly title: string;
  readonly entries: readonly CodeEntry[];
}

const patterns = Object.keys(schema.patternProperties);
if (patterns.length !== 1) throw new Error("codes.schema.json: expected one group pattern in patternProperties");
const GROUP = new RegExp(patterns[0] ?? "");

/** `textWarnings` → "Text warnings", `warnings` → "Warnings". */
function title(key: string): string {
  const words = key.replace(/([A-Z])/g, " $1").toLowerCase();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** Every group of codes, in the file's order. */
export const CODE_GROUPS: readonly CodeGroup[] = Object.entries(codes as Record<string, unknown>)
  .filter(([key, value]) => GROUP.test(key) && Array.isArray(value))
  .map(([key, value]) => ({ key, title: title(key), entries: value as CodeEntry[] }));

/** The entry of `code`, or `undefined` when codes.json doesn't list it. */
export function codeEntry(code: string): CodeEntry | undefined {
  for (const group of CODE_GROUPS) {
    const entry = group.entries.find((e) => e.code === code);
    if (entry !== undefined) return entry;
  }
  return undefined;
}
