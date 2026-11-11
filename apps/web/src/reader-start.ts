// Where reader mode's column starts: the first block at or below the top of the page view.
import type { DocumentReflow } from "./session.ts";

type Block = DocumentReflow["blocks"][number];

/** The first block at or after `point` (page space) in reading order: where the column starts. */
export function blockAt(blocks: readonly Block[], point: { page: number; y: number } | undefined): number {
  if (point === undefined || blocks.length === 0) return 0;
  const index = blocks.findIndex(
    // A crop with no box has no place on its page: it starts the column only from a later page.
    (b) => b.page > point.page || (b.page === point.page && b.bbox !== null && b.bbox[1] + b.bbox[3] > point.y),
  );
  return index === -1 ? blocks.length - 1 : index;
}
