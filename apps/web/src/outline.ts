// What the reader keeps of a page as the viewer opened it: its size, label and turn. The outline's
// own model, its places and its jumps live in `@laminapdf/ui`, which the desktop shares.

import type { OpenedDocument } from "@laminapdf/viewer";

/** A page's facts (size and label), as the viewer opened them. */
export type PageInfo = OpenedDocument["pages"][number];
