// The web reader's localStorage persistence for page appearance choices (R10). The pure choice
// logic lives in `@laminapdf/ui` so the web and desktop readers share one implementation; this
// module re-exports it under the web reader's names and adds `localStorage` reading and saving.

import { type PageAppearanceChoices, parsePageAppearanceChoices } from "@laminapdf/ui";

export {
  choosePageAppearance as choose,
  type DocumentPlace,
  documentAppearanceKey as documentKey,
  isPageAppearanceName as isName,
  type PageAppearanceChoices,
  parsePageAppearanceChoices as parseChoices,
  resolvePageAppearance as resolve,
  undoPageAppearance as undo,
  withDocumentAppearance as withDocument,
} from "@laminapdf/ui";

/** Where the choices are kept, in this browser only. */
const STORAGE_KEY = "lamina.page-appearance";

export function readChoices(): PageAppearanceChoices {
  try {
    return parsePageAppearanceChoices(localStorage.getItem(STORAGE_KEY));
  } catch {
    return { documents: {} };
  }
}

export function saveChoices(choices: PageAppearanceChoices): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(choices));
  } catch {
    // Storage blocked: the choice lasts until the tab closes.
  }
}
