// Edit a document's title, authors and keywords, keeping its Info dictionary and XMP metadata in step.
// One edit sets the title and the authors, another removes the keywords, and the save stamps the
// time it happened. Before saving, the plan says whether the save is allowed (a certified file
// forbids metadata changes) and which PDF/A or PDF/UA claims the edit breaks.
import { Document } from "@laminapdf/sdk";

/**
 * The document with its metadata edited, saved incrementally.
 *
 * @param savedAt - the time of the save, as a PDF date (`D:20261110093000Z`) your app takes
 *   from its clock.
 */
export function editMetadata(bytes: Uint8Array, savedAt: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  console.log(`title before: ${doc.metadata().title ?? "(none)"}`);

  doc.setMetadata({ title: "Quarterly figures", authors: ["Ada Lovelace", "Charles Babbage"] }, { at: savedAt });
  doc.removeMetadata(["keywords"]);

  const options = { mode: "incremental", metadataDate: savedAt } as const;
  const plan = doc.plan(options);
  if (plan.refusal !== undefined) {
    throw new Error(`the save would be refused: ${plan.refusal}`);
  }
  for (const warning of plan.conformanceWarnings) {
    console.warn(`warning: ${warning.code}`);
  }
  return doc.save(options);
}
