// An import is an edit that also answers a summary: the edit goes the way every edit goes (the
// client's, the session's or the viewer's own bookkeeping), and the summary rides beside it.

import type { EditResult, ImportedComments } from "../protocol.js";

/**
 * Runs `call`, which makes an import and passes its answer through `pass`, and gives the edit with
 * the summary `pass` saw.
 */
export async function withSummary(
  call: (pass: (done: ImportedComments) => EditResult) => Promise<EditResult>,
): Promise<ImportedComments> {
  let summary: ImportedComments["summary"] | undefined;
  const edit = await call((done) => {
    summary = done.summary;
    return done.edit;
  });
  if (summary === undefined) throw new Error("the import came back without its summary");
  return { edit, summary };
}
