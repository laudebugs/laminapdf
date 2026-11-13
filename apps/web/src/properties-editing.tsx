// Document properties with its Description form and XMP tab (O04), loaded with the rest of
// later.ts (#380): what they edit, read each time the dialog opens, and Apply's changes made one
// edit.

import type { MetadataPatch, Op, XmpValue } from "@laminapdf/sdk";
import { message } from "@laminapdf/ui";
import {
  type MetadataChange,
  PropertiesDialog,
  type PropertiesDialogProps,
  type PropertiesEditing,
  type XmpChange,
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
            const ops = [
              ...xmp.map((c) => xmpOp(c, options)),
              ...(change === undefined ? [] : metadataOps(change, options)),
            ];
            if (ops.length > 0) await engine.applyEdits(ops, message("properties.edit.step"));
          },
        };
  return <PropertiesDialog {...props} editing={edit} />;
}

/** An XMP tab change as its op. */
function xmpOp(change: XmpChange, options: { readonly replaceMalformed: boolean }): Op {
  const replace = options.replaceMalformed ? { replaceMalformed: true } : {};
  switch (change.kind) {
    case "set": {
      const { namespace, prefix, name } = change;
      // The tab's values are the SDK's XmpValue, written without its readonly arrays.
      const value = structuredClone(change.value) as XmpValue;
      return { op: "setXmpProperty", args: { namespace, prefix, name, value, ...replace } };
    }
    case "remove":
      return { op: "removeXmpProperty", args: { namespace: change.namespace, name: change.name, ...replace } };
    case "replace":
      return { op: "replaceXmpPacket", args: { xml: change.xml } };
  }
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
