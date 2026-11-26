// The comment summary's own words in the UI's language (E5k's `SummaryWords`): one template per
// phrase, and the names of the annotation kinds and review states, from the messages catalogue. A
// translation of the catalogue translates the summary; the comments' own text never changes.

import { type LaterMessageId, laterMessage } from "../later-messages.js";

/** The words a summary writes itself, as the SDK's `CommentSummaryWords`: templates, kind names and state names. */
export interface SummaryWords {
  readonly titled: string;
  readonly untitled: string;
  readonly noComments: string;
  readonly noneChosen: string;
  readonly count: string;
  readonly commentsOne: string;
  readonly commentsOther: string;
  readonly pagesOne: string;
  readonly pagesOther: string;
  readonly page: string;
  readonly pageEmpty: string;
  readonly figureOne: string;
  readonly figureMany: string;
  readonly status: string;
  readonly marked: string;
  readonly quote: string;
  readonly kinds: Readonly<Record<string, string>>;
  readonly states: Readonly<Record<string, string>>;
}

/** The annotation kinds the summary names, by PDF subtype. */
const KINDS = [
  "Text",
  "FreeText",
  "Line",
  "Square",
  "Circle",
  "Polygon",
  "PolyLine",
  "Highlight",
  "Underline",
  "Squiggly",
  "StrikeOut",
  "Caret",
  "Stamp",
  "Ink",
  "FileAttachment",
  "Sound",
  "Redact",
] as const;

/** The review states the summary names, by PDF name. */
const STATES = ["Accepted", "Rejected", "Cancelled", "Completed"] as const;

/** A template as written, its `{names}` left for the summary to fill. */
const template = (id: LaterMessageId) => laterMessage(id);

/** Every word the summary writes itself, in the language of the catalogue. */
export function summaryWords(): SummaryWords {
  return {
    titled: template("summary.words.titled"),
    untitled: template("summary.words.untitled"),
    noComments: template("summary.words.noComments"),
    noneChosen: template("summary.words.noneChosen"),
    count: template("summary.words.count"),
    commentsOne: template("summary.words.commentsOne"),
    commentsOther: template("summary.words.commentsOther"),
    pagesOne: template("summary.words.pagesOne"),
    pagesOther: template("summary.words.pagesOther"),
    page: template("summary.words.page"),
    pageEmpty: template("summary.words.pageEmpty"),
    figureOne: template("summary.words.figureOne"),
    figureMany: template("summary.words.figureMany"),
    status: template("summary.words.status"),
    marked: template("summary.words.marked"),
    quote: template("summary.words.quote"),
    kinds: Object.fromEntries(KINDS.map((kind) => [kind, template(`summary.kind.${kind}`)])),
    states: Object.fromEntries(STATES.map((state) => [state, template(`summary.state.${state}`)])),
  };
}
