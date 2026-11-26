// The question when files of a split's names are in the folder the person chose (Paper O08), shared
// by both readers: Replace them, or Keep both under numbered names.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";

/** The question: how many files are taken, and the person's answer's taker. */
export interface FilesExist {
  readonly count: number;
  /** `true` to replace them, `false` to keep both, `undefined` to cancel. */
  readonly answer: (replace: boolean | undefined) => void;
}

/** The question when files of the parts' names are in the chosen folder: Replace, or Keep both. `ask` is undefined while nothing is asked. */
export function ExistsDialog({ ask }: { readonly ask: FilesExist | undefined }): ReactNode {
  const count = ask?.count ?? 0;
  return (
    <Dialog
      isOpen={ask !== undefined}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) ask?.answer(undefined);
      }}
      onEscape={() => ask?.answer(undefined)}
      title={laterCountMessage(count, { one: "documents.exists.one", many: "documents.exists" })}
      description={laterMessage("documents.exists.body")}
      actions={
        <>
          <Button onPress={() => ask?.answer(undefined)}>{laterMessage("organise.move.cancel")}</Button>
          <Button onPress={() => ask?.answer(true)}>{laterMessage("documents.exists.replace")}</Button>
          <Button variant="primary" autoFocus onPress={() => ask?.answer(false)}>
            {laterMessage("documents.exists.keep")}
          </Button>
        </>
      }
    />
  );
}
