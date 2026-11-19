// The colour swatches a Comment tool offers: fill colours for highlights and notes, line colours
// for the rest, named and coloured by their tokens.

import { FILL_COLOURS, LINE_COLOURS, laterMessage } from "@laminapdf/ui/editing";

/** The swatches of a colour family. */
export function swatches(family: "fill" | "line") {
  return family === "fill"
    ? FILL_COLOURS.map((id) => ({ id, label: laterMessage(`colour.${id}`), color: `annotation-${id}` as const }))
    : LINE_COLOURS.map((id) => ({ id, label: laterMessage(`colour.${id}`), color: `content-line-${id}` as const }));
}
