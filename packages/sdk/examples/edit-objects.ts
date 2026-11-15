// Move, resize, delete and reorder what a page already draws.
//
// Lists the first page's objects, then changes four of them in one edit: shrinks the first
// shape about its centre, deletes the second, moves the last image across the page and sends
// the third shape, a line across the page, under everything else. Each edit names its object as
// the list does, and only the objects it names are written again: the rest of the page keeps its
// bytes. An object that would draw differently in its new place in the stacking order (another
// clip or transparency there) is refused with `content-not-editable`.
import { Document } from "@laminapdf/sdk";

/** The document with four objects of its first page changed. Needs `init({ extract: true })`. */
export function editObjects(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const objects = doc.content.objects(0);
  for (const object of objects) {
    console.log(object.kind, object.bounds, object.readOnly ? `(read-only: ${object.readOnly})` : "");
  }
  const [square, second, line] = objects.filter((o) => o.kind === "path");
  const photo = objects.filter((o) => o.kind === "image").at(-1);
  if (!square?.bounds || !second || !line || !photo) {
    throw new Error("the page needs three shapes and an image");
  }
  // Page space: points from the top-left, y down. Half size, about the square's centre.
  const [x0, y0, x1, y1] = square.bounds;
  const [cx, cy] = [(x0 + x1) / 2, (y0 + y1) / 2];
  doc.content.edit(0, [
    { transform: { object: square.object, matrix: [0.5, 0, 0, 0.5, cx - cx * 0.5, cy - cy * 0.5] } },
    { delete: { object: second.object } },
    { transform: { object: photo.object, matrix: [1, 0, 0, 1, 300, 0] } },
    { reorder: { object: line.object, to: "back" } },
  ]);
  return doc.save();
}
