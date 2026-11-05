import { describe, expect, it } from "vitest";
import { aliases, iconFiles, readRepo } from "./helpers.ts";

/** Every row of §5's icon tables: its file name and its markup, or the icon it repeats. */
function tableRows(): { file: string; markup?: string; same?: string }[] {
  const md = readRepo("docs/design/paper/design-language.md");
  const section = md.slice(md.indexOf("## 5. Iconography"), md.indexOf("## 6. Layout"));
  return section
    .split("\n")
    .filter((line) => /^\| [^|]+ \| `[a-z0-9-]+` \| /.test(line))
    .map((line) => {
      const cells = line.split(" | ");
      const file = /`([a-z0-9-]+)`/.exec(cells[1] ?? "")?.[1] ?? "";
      const same = /^the `([a-z0-9-]+)` icon$/.exec(cells[2] ?? "")?.[1];
      const markup = /^`(<.*>)`$/.exec(cells[2]?.replace(/ \|$/, "") ?? "")?.[1];
      return { file, ...(markup === undefined ? {} : { markup }), ...(same === undefined ? {} : { same }) };
    });
}

const inner = (text: string) => /^<svg [^>]*>(.*)<\/svg>$/s.exec(text.trim())?.[1];

describe("the design language's §5 tables", () => {
  const rows = tableRows();
  const files = new Map(iconFiles().map(([name, text]) => [name, inner(text)]));

  it("list every icon file once, with its markup exactly", () => {
    const drawn = rows.filter((r) => r.markup !== undefined);
    expect(drawn.map((r) => r.file).sort()).toEqual([...files.keys()].sort());
    for (const row of drawn) expect(row.markup, row.file).toBe(files.get(row.file));
  });

  it("list every alias as the icon it repeats", () => {
    const repeats = Object.fromEntries(rows.filter((r) => r.same !== undefined).map((r) => [r.file, r.same]));
    const { "chevron-left": _documentedAsBack, ...rest } = aliases();
    expect(repeats).toEqual(rest);
  });
});
