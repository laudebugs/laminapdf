// Document properties with its Description form and XMP tab (O04), loaded with the rest of
// later.ts (#380): what they edit, read each time the dialog opens, and Apply's changes made one
// edit.

import type { Op } from "@laminapdf/sdk";
import { message } from "@laminapdf/ui";
import {
  PropertiesDialog,
  type PropertiesDialogProps,
  type PropertiesEditing,
  propertiesOps,
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
  const [editing, setEditing] = useState<Pick<PropertiesEditing, "metadata" | "rewritesWhole" | "xmp">>();
  useEffect(() => {
    if (!props.isOpen) return;
    let current = true;
    void engine.editableMetadata().then((editable) => {
      if (!current || editable === undefined) return;
      setEditing({
        metadata: { status: "ready", metadata: editable.metadata },
        xmp: editable.xmp,
        rewritesWhole: editable.rewritesWhole,
      });
    }, reportError);
    // Closing forgets the form's data, so the next open starts from what the session reads then.
    return () => {
      current = false;
      setEditing(undefined);
    };
  }, [engine, props.isOpen]);
  const edit: PropertiesEditing | undefined =
    editing === undefined
      ? undefined
      : {
          ...editing,
          onApply: async (change, xmp, options) => {
            // The dialog's ops are shapes of the SDK's (@laminapdf/ui doesn't depend on it).
            const ops = propertiesOps(change, xmp, options) as unknown as Op[];
            if (ops.length > 0) await engine.applyEdits(ops, message("properties.edit.step"));
          },
        };
  return <PropertiesDialog {...props} editing={edit} />;
}
