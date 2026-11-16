// The Properties dialog's Apply as edit ops (O04), one list for the web reader and the desktop:
// the XMP tab's changes first, then the Description form's, the metadata set then removed, Info
// and XMP together. The ops are shapes, as `quarterTurn` is: @laminapdf/ui depends on neither the
// SDK nor the viewer, so an app hands them to its edit session as the SDK's `Op`.

import type { EditableField, MetadataChange } from "./properties-form.js";
import type { XmpChange, XmpValueData } from "./xmp-model.js";

/** Given with every metadata and XMP property op when the XMP metadata can't be read: Apply replaces it. */
type Replace = { readonly replaceMalformed?: true };

/** One op of the dialog's Apply, as the SDK's ops of these names take their arguments. */
export type PropertiesOp =
  | { readonly op: "setMetadata"; readonly args: { readonly patch: MetadataChange["patch"] } & Replace }
  | { readonly op: "removeMetadata"; readonly args: { readonly keys: readonly EditableField[] } & Replace }
  | {
      readonly op: "setXmpProperty";
      readonly args: {
        readonly namespace: string;
        readonly prefix: string;
        readonly name: string;
        readonly value: XmpValueData;
      } & Replace;
    }
  | { readonly op: "removeXmpProperty"; readonly args: { readonly namespace: string; readonly name: string } & Replace }
  | { readonly op: "replaceXmpPacket"; readonly args: { readonly xml: string } };

/**
 * The ops that make the dialog's changes one edit: the XMP tab's `xmp`, then the Description
 * tab's `change`, each op replacing the XMP metadata when `replaceMalformed` says it can't be read.
 */
export function propertiesOps(
  change: MetadataChange | undefined,
  xmp: readonly XmpChange[],
  options: { readonly replaceMalformed: boolean },
): PropertiesOp[] {
  const replace: Replace = options.replaceMalformed ? { replaceMalformed: true } : {};
  const ops: PropertiesOp[] = xmp.map((c) => xmpOp(c, replace));
  if (change === undefined) return ops;
  if (Object.keys(change.patch).length > 0) ops.push({ op: "setMetadata", args: { patch: change.patch, ...replace } });
  if (change.remove.length > 0) ops.push({ op: "removeMetadata", args: { keys: change.remove, ...replace } });
  return ops;
}

/** An XMP tab change as its op. A replaced packet is parsed whole, so it takes no `replaceMalformed`. */
function xmpOp(change: XmpChange, replace: Replace): PropertiesOp {
  switch (change.kind) {
    case "set": {
      const { namespace, prefix, name, value } = change;
      return { op: "setXmpProperty", args: { namespace, prefix, name, value, ...replace } };
    }
    case "remove":
      return { op: "removeXmpProperty", args: { namespace: change.namespace, name: change.name, ...replace } };
    case "replace":
      return { op: "replaceXmpPacket", args: { xml: change.xml } };
  }
}
