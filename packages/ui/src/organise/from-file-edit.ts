// The edit Insert pages from a file or Replace pages makes (Paper O07), as both readers apply it:
// the op over a source the engine holds, the pages it moves the selection to, and the words that
// name the step and its toast.

import { laterCountMessage, laterMessage } from "../later-messages.js";
import type { FromFileChoice } from "./from-file.js";
import type { MergeChoice } from "./merge.js";

/** What the edit session is asked to do for one Insert or Replace. */
export interface FromFileEdit {
  readonly ops: readonly object[];
  /** The pages (0-based) of the open document the step touches, which stay selected. */
  readonly indices: readonly number[];
  /** The step's name in the history. */
  readonly label: string;
  /** The toast once it is done. */
  readonly done: () => string;
}

/**
 * The edit for `choice`, over `source` (its ID, and the IDs of its pages in the source's page
 * order), made on the open document's `pages` (their IDs, in page order) with the pages at
 * `selected` selected.
 */
export function fromFileEdit(
  choice: FromFileChoice,
  source: { readonly id: string; readonly pages: readonly string[] },
  pages: readonly string[],
  selected: readonly number[],
): FromFileEdit {
  const chosen = choice.pages.flatMap((i) => (source.pages[i] === undefined ? [] : [source.pages[i] as string]));
  const carry = { annotations: choice.annotations, fields: choice.linkFields ? "link" : "rename" };
  if (choice.where === "replace") {
    const targets = selected.flatMap((i) => (pages[i] === undefined ? [] : [pages[i] as string]));
    const label = laterCountMessage(targets.length, {
      one: "documents.step.replace.one",
      many: "documents.step.replace",
    });
    return {
      ops: [{ op: "replacePages", args: { targets, source: source.id, pages: chosen, carry } }],
      indices: selected,
      label,
      done: () => label,
    };
  }
  const at =
    choice.where === "start" || choice.where === "end" ? choice.where : { [choice.where]: pages[choice.page] ?? "" };
  const label = laterCountMessage(chosen.length, {
    one: "documents.step.insert.one",
    many: "documents.step.insert",
  });
  return {
    ops: [{ op: "insertPagesFrom", args: { source: source.id, pages: chosen, at, carry } }],
    indices: [],
    label,
    done: () => laterCountMessage(chosen.length, { one: "documents.done.insert.one", many: "documents.done.insert" }),
  };
}

/** The toast when the file couldn't be read as a source. */
export function sourceUnreadable(name: string): string {
  return laterMessage("documents.failed.read", { name });
}

/** What the edit session is asked to do for Merge documents into the open document. */
export interface MergeEdit {
  readonly ops: readonly object[];
  /** The step's name in the history. */
  readonly label: string;
  /** The toast once it is done: how many documents, and the fields renamed. */
  readonly done: (result: { readonly fields?: readonly { readonly outcome: string }[] | undefined }) => string;
}

/**
 * The edit that adds each of `sources` (its ID in the edit session and its title, which names
 * its bookmark) after the open document's last page, in order.
 */
export function mergeEdit(
  choice: Pick<MergeChoice, "bookmarks" | "fields">,
  sources: readonly { readonly source: string; readonly title: string }[],
): MergeEdit {
  const options = { outline: choice.bookmarks, fields: choice.fields } as const;
  return {
    ops: [{ op: "mergeDocuments", args: { sources, at: "end", options } }],
    label: laterCountMessage(sources.length, { one: "documents.step.merge.one", many: "documents.step.merge" }),
    done: (result) => {
      const renamed = result.fields?.filter((f) => f.outcome === "renamed").length ?? 0;
      const merged = laterMessage("documents.done.merge", { count: sources.length + 1 });
      return renamed === 0
        ? merged
        : `${merged} ${laterCountMessage(renamed, { one: "documents.done.renamed.one", many: "documents.done.renamed" })}`;
    },
  };
}
