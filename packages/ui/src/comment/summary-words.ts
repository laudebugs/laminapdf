// The comment summary's own words in the UI's language (the SDK's summary words): one template per
// phrase, and the names of the annotation kinds and review states, from the messages catalogue. A
// translation of the catalogue translates the summary; the comments' own text never changes.

import { laterMessage } from "../later-messages.js";
import { SUMMARY_KINDS } from "./kind-name.js";

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

/** The review states the summary names, by PDF name. */
const STATES = ["Accepted", "Rejected", "Cancelled", "Completed"] as const;

/** Every word the summary writes itself, in the language of the catalogue. */
export function summaryWords(): SummaryWords {
  return {
    titled: laterMessage("summary.words.titled"),
    untitled: laterMessage("summary.words.untitled"),
    noComments: laterMessage("summary.words.noComments"),
    noneChosen: laterMessage("summary.words.noneChosen"),
    count: laterMessage("summary.words.count"),
    commentsOne: laterMessage("summary.words.commentsOne"),
    commentsOther: laterMessage("summary.words.commentsOther"),
    pagesOne: laterMessage("summary.words.pagesOne"),
    pagesOther: laterMessage("summary.words.pagesOther"),
    page: laterMessage("summary.words.page"),
    pageEmpty: laterMessage("summary.words.pageEmpty"),
    figureOne: laterMessage("summary.words.figureOne"),
    figureMany: laterMessage("summary.words.figureMany"),
    status: laterMessage("summary.words.status"),
    marked: laterMessage("summary.words.marked"),
    quote: laterMessage("summary.words.quote"),
    kinds: Object.fromEntries(SUMMARY_KINDS.map((kind) => [kind, laterMessage(`summary.kind.${kind}`)])),
    states: Object.fromEntries(STATES.map((state) => [state, laterMessage(`summary.state.${state}`)])),
  };
}
