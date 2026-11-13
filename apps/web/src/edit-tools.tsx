// The interaction modes of each edit session: one tool registry, shared by the edit runtime's
// pointer handling and the top bar's tool toggles (the Link tool, O12). Loaded with later.ts once
// a document is open.

import type { EditControls, KeyPlatform } from "@laminapdf/ui";
import { ToolRegistry, EditArea as UiEditArea } from "@laminapdf/ui/editing";
import type { ReactNode } from "react";

const registries = new WeakMap<EditControls, ToolRegistry>();

/** The tool registry of the edit session `edits` (a viewer's), made the first time it is asked for. */
export function toolsOf(edits: EditControls): ToolRegistry {
  let tools = registries.get(edits);
  if (tools === undefined) {
    tools = new ToolRegistry();
    registries.set(edits, tools);
  }
  return tools;
}

/** The top bar's edit area, with the tool toggles of its session. */
export function EditArea({ edits, keys }: { edits: EditControls; keys: KeyPlatform }): ReactNode {
  return <UiEditArea edits={edits} keys={keys} tools={toolsOf(edits)} />;
}
