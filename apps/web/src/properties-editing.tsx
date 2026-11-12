// Document properties with its Description form (O04), loaded with the rest of later.ts (#380):
// what the form edits, read each time the dialog opens, and Apply's change made one edit.

import type { MetadataPatch, Op } from "@laminapdf/sdk";
import { message } from "@laminapdf/ui";
import {
  type MetadataChange,
  type MetadataState,
  PropertiesDialog,
  type PropertiesDialogProps,
  type PropertiesEditing,
} from "@laminapdf/ui/dialogs";
import { type ReactNode, useEffect, useState } from "react";
import type { Engine } from "./session.ts";

/** Props of `EditablePropertiesDialog`: the dialog's own, less `editing`, and the engine. */
interface EditablePropertiesDialogProps extends Omit<PropertiesDialogProps, "editing"> {
  readonly engine: Engine;
}

/** The properties dialog, with the form when the open document can be edited. */
export function EditablePropertiesDialog({ engine, ...props }: EditablePropertiesDialogProps): ReactNode {
  // Undefined until the session says whether the document can be edited, and when it can't.
  const [editing, setEditing] = useState<{ metadata: MetadataState; rewritesWhole: boolean }>();
  useEffect(() => {
    if (!props.isOpen) return;
    let current = true;
    setEditing(undefined);
    void engine.editableMetadata().then((editable) => {
      if (!current || editable === undefined) return;
      setEditing({ metadata: { status: "ready", metadata: editable.metadata }, rewritesWhole: editable.rewritesWhole });
    }, reportError);
    return () => {
      current = false;
    };
  }, [engine, props.isOpen]);
  const edit: PropertiesEditing | undefined =
    editing === undefined
      ? undefined
      : {
          ...editing,
          onApply: async (change, options) => {
            const ops = metadataOps(change, options);
            if (ops.length > 0) await engine.applyEdits(ops, message("properties.edit.step"));
          },
        };
  return <PropertiesDialog {...props} editing={edit} />;
}

/** The form's change as ops: `patch` set and `remove` removed, Info and XMP together. */
function metadataOps(change: MetadataChange, options: { readonly replaceMalformed: boolean }): Op[] {
  const replace = options.replaceMalformed ? { replaceMalformed: true } : {};
  const { authors, ...rest } = change.patch;
  const patch: MetadataPatch = { ...rest, ...(authors === undefined ? {} : { authors: [...authors] }) };
  const ops: Op[] = [];
  if (Object.keys(patch).length > 0) ops.push({ op: "setMetadata", args: { patch, ...replace } });
  if (change.remove.length > 0) ops.push({ op: "removeMetadata", args: { keys: [...change.remove], ...replace } });
  return ops;
}
