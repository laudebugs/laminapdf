// The Organise mode in the reader (Paper O06): @laminapdf/ui's mode, which the desktop reader
// runs too, over the document worker's engine and viewer, with the reader's own part: the dialogs
// that read or write other files (Insert from a file, Replace, Merge, Split and Extract;
// documents.tsx) and saving's copies for the certification banner's Save a copy. Loaded with
// later.ts once a document is open (#380).

import type { CommandRegistry, KeyPlatform, ModeRegistry, Platform, ToastQueue } from "@laminapdf/ui";
import {
  ModeArea,
  OrganiseController as SharedController,
  OrganiseMode as SharedMode,
  organiseActions as sharedActions,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import type { ReactNode } from "react";
import { DocumentsController, DocumentsDialogs, documentsActions } from "./documents.tsx";
import type { SaveFlow } from "./save-flow.ts";
import type { Engine } from "./session.ts";

export { ModeArea };

/** The Organise mode's state and actions over one open document, with the reader's dialogs across files. */
export class OrganiseController extends SharedController {
  /** Insert from a file, Replace, Merge, Split and Extract (documents.tsx). */
  readonly documents: DocumentsController;

  constructor(
    engine: Engine,
    viewer: Viewer,
    toasts: ToastQueue,
    platform: Platform,
    copies: Pick<SaveFlow, "copyWith" | "copyTitle">,
  ) {
    super(engine, viewer, toasts, copies);
    this.documents = new DocumentsController({ engine, organise: this, toasts, platform });
  }
}

/** The handlers the page commands run while Organise is the mode. */
export function organiseActions(controller: OrganiseController) {
  return { ...sharedActions(controller), ...documentsActions(controller.documents) } as const;
}

/** Props of `OrganiseMode`. */
export interface OrganiseModeProps {
  readonly controller: OrganiseController;
  /** The open file's name, which the files Split and Merge write are named from. */
  readonly fileName: string;
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly modes: ModeRegistry;
  readonly registry: CommandRegistry;
  readonly keys: KeyPlatform;
}

/** The Organise mode: registered while a document is open, its grid over the viewer while current. */
export function OrganiseMode({ fileName, ...props }: OrganiseModeProps): ReactNode {
  const { controller, engine } = props;
  return (
    <SharedMode
      {...props}
      dialogs={
        <DocumentsDialogs documents={controller.documents} organise={controller} engine={engine} fileName={fileName} />
      }
    />
  );
}
