// What the inspector shows of the objects the file already had (Paper A05): "Image", "n shapes"
// or "Text block", their position and size, Arrange, Delete, and for one picture Replace, Crop and
// alternative text. Apart from the DOM. Each field is one `EditContent` op through the controller.

import type { ContentInspectorProps } from "@laminapdf/ui/editing";
import { laterMessage } from "@laminapdf/ui/editing";
import type { ContentController } from "./controller.ts";
import { type Original, type Picked, pickKind } from "./original.ts";

/** What the inspector needs besides the pick. */
export interface OriginalInspectorOptions {
  readonly keepRatio: boolean;
  readonly locked: string | undefined;
  /** The file claims PDF/UA, so a picture needs alternative text. */
  readonly needsAlt: boolean;
  /** The alternative text this session gave the picture, if any. */
  readonly altText: string;
}

/** The reason an object can't be changed, in words. */
function refusalText(reason: string): string {
  const known = new Set(["unsupported", "clipping-path"]);
  const why = known.has(reason)
    ? laterMessage(`content.original.readOnly.${reason as "unsupported"}`)
    : laterMessage("content.original.readOnly.other", { reason });
  return laterMessage("content.original.readOnly", { reason: why });
}

/** The crop's insets of one picture, as the inspector's fields hold them. */
function cropFields(
  controller: ContentController,
  image: Original,
): NonNullable<ContentInspectorProps["image"]>["crop"] {
  const [left, top, right, bottom] = controller.originals.cropOf(image) ?? [0, 0, 0, 0];
  const insets = { left, top, right, bottom };
  return {
    ...insets,
    onChange: (edge, points) => {
      const next = { ...insets, [edge]: points };
      void controller.originals.crop([next.left, next.top, next.right, next.bottom]);
    },
  };
}

/** The picture-only parts: Replace, Crop and alternative text. */
function pictureProps(
  controller: ContentController,
  image: Original,
  options: OriginalInspectorOptions,
): Pick<ContentInspectorProps, "image" | "alt"> {
  return {
    image: { onReplace: () => void controller.pickImage("picked"), crop: cropFields(controller, image) },
    ...(options.needsAlt
      ? {
          alt: {
            text: options.altText,
            decorative: false,
            help: laterMessage("content.original.alt.help"),
            ...(options.altText === "" ? { error: laterMessage("content.original.alt.error") } : {}),
            onText: (text: string) => void controller.setAlt(text),
          },
        }
      : {}),
  };
}

/** The name of the action that deletes the pick: "Delete image", "Delete shapes". */
export function deleteLabelOf(pick: Picked): string {
  const kind = pickKind(pick);
  return laterMessage(
    kind === "shape" && pick.objects.length > 1 ? "content.original.delete.shapes" : `content.original.delete.${kind}`,
  );
}

/** The inspector's props for the pick. */
export function originalInspectorProps(
  controller: ContentController,
  pick: Picked,
  options: OriginalInspectorOptions,
): ContentInspectorProps {
  const originals = controller.originals;
  const kind = pickKind(pick);
  const refusal = pick.objects.find((o) => o.readOnly !== undefined)?.readOnly;
  const one = pick.objects.length === 1 ? pick.objects[0] : undefined;
  const box = originals.box(pick);
  const notes = [
    ...(kind === "text" ? [laterMessage("content.original.note.text")] : []),
    ...(kind === "image" ? [laterMessage("content.original.note.shared")] : []),
    laterMessage("content.original.history"),
  ];
  const locked = refusal === undefined ? options.locked : refusalText(refusal);
  return {
    title: originals.title(pick),
    context: laterMessage("content.original.context", { page: pick.page + 1 }),
    geometry: {
      ...box,
      keepRatio: options.keepRatio,
      textBox: false,
      fixed: false,
      onChange: (change) => void originals.setBox(change),
      onKeepRatio: (keep) => controller.setKeepRatio(keep),
    },
    arrange: {
      onFront: () => controller.arrangeSelection("front"),
      onForward: () => controller.arrangeSelection("forward"),
      onBackward: () => controller.arrangeSelection("backward"),
      onBack: () => controller.arrangeSelection("back"),
    },
    deleteLabel: deleteLabelOf(pick),
    onDelete: () => controller.deleteSelection(),
    notes,
    ...(locked === undefined ? {} : { locked }),
    ...(one !== undefined && one.kind === "image" ? pictureProps(controller, one, options) : {}),
  };
}
