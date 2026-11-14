// Crop every page by half an inch on each side, whatever its size or rotation.
//
// Margins are measured on the page as it shows, so a page turned on its side loses the same
// strip from its visible top as an upright one. Cropping only hides content: the edit lists the
// pages whose hidden content stays in the file, where text extraction still finds it.
import { Document } from "@laminapdf/sdk";

/** The document with 36 points cropped from each side of every page. Needs `init()`. */
export function cropPages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const pages = doc.pages.map((page) => page.id);
  const edited = doc.cropPages(pages, { top: 36, right: 36, bottom: 36, left: 36 });
  for (const page of edited.croppedContentRemains ?? []) {
    console.warn(`page ${page} hides content that is still in the file`);
  }
  return doc.save();
}
