// A name for a new file in a folder the person chose (Split's parts, an export's files): the name
// asked for, or that name with " (2)", " (3)"… before its extension, the first not taken there.

import type { PlatformFolder } from "@laminapdf/ui";

/** `name`, or `name` with ` (2)`, ` (3)`… before its extension: the first that isn't in `folder`. */
export async function freeName(folder: Pick<PlatformFolder, "existing">, name: string): Promise<string> {
  if ((await folder.existing([name])).length === 0) return name;
  const dot = name.lastIndexOf(".");
  const [stem, extension] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ""];
  for (let n = 2; ; n++) {
    const candidate = `${stem} (${n})${extension}`;
    if ((await folder.existing([candidate])).length === 0) return candidate;
  }
}
