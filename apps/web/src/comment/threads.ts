// The Comments panel's threads (Paper C04): each comment's status, kind and words read once from
// the document's threads, then filtered by status, author, kind and page, searched over the note,
// the replies, the quoted text and the author, and sorted. Pure, so the panel and its tests agree.

import type { CommentThread } from "@laminapdf/sdk";

/** A thread's status as the panel names it: open (no status set), or the latest review state. */
export type ThreadStatus = "open" | "accepted" | "rejected" | "cancelled" | "done";

/** The statuses, in the filter menu's order. */
export const STATUSES: readonly ThreadStatus[] = ["open", "accepted", "rejected", "cancelled", "done"];

/** The filter menu's kinds of comment (C04). */
export type ThreadKind = "markup" | "notes" | "drawings" | "stamps";

/** The kinds, in the filter menu's order. */
export const KINDS: readonly ThreadKind[] = ["markup", "notes", "drawings", "stamps"];

/** How the threads are ordered. */
export type ThreadSort = "page" | "newest" | "author";

/** One reply, as the panel shows it. */
export interface ReplyView {
  readonly id: string;
  readonly author: string | undefined;
  /** When it was written: `/M`, else `/CreationDate`, as the PDF writes dates. */
  readonly date: string | undefined;
  readonly text: string;
}

/** One thread, as the panel shows and filters it. */
export interface ThreadView {
  /** The comment's annotation ID, which selecting and the ops use. */
  readonly id: string;
  /** Its page, from 0. */
  readonly page: number;
  readonly subtype: string;
  readonly kind: ThreadKind;
  /** Its colour (`/C`) as CSS, for the type mark; none when it has none. */
  readonly colour: string | undefined;
  readonly author: string | undefined;
  readonly date: string | undefined;
  readonly note: string;
  /** For text markup, the text it marks. */
  readonly quoted: string | undefined;
  readonly status: ThreadStatus;
  /** Who set the status, when one is set. */
  readonly statusBy: string | undefined;
  /** Whether the latest Marked state is Marked (C04's Mark as checked). */
  readonly checked: boolean;
  readonly replies: readonly ReplyView[];
  /** Whether the thread is broken: its parent is missing, or its replies loop. */
  readonly broken: boolean;
}

/** What the filters keep; an empty set keeps every value. */
export interface ThreadFilter {
  readonly statuses: ReadonlySet<ThreadStatus>;
  readonly authors: ReadonlySet<string>;
  readonly kinds: ReadonlySet<ThreadKind>;
  /** The pages kept, from 0, both ends included; every page when undefined. */
  readonly pages: readonly [number, number] | undefined;
}

/** No filter: every thread shown. */
export const NO_FILTER: ThreadFilter = { statuses: new Set(), authors: new Set(), kinds: new Set(), pages: undefined };

const TEXT_MARKUP = new Set(["Highlight", "Underline", "StrikeOut", "Squiggly", "Caret"]);
const NOTES = new Set(["Text", "FreeText"]);
const STAMPS = new Set(["Stamp", "FileAttachment", "Sound"]);

/** The filter menu's kind of `subtype`. */
export function kindOf(subtype: string): ThreadKind {
  if (TEXT_MARKUP.has(subtype)) return "markup";
  if (NOTES.has(subtype)) return "notes";
  if (STAMPS.has(subtype)) return "stamps";
  return "drawings";
}

/** The status the latest review state `review` gives (`Completed` is Done, C04). */
function statusOf(review: string | undefined): ThreadStatus {
  switch (review) {
    case "Accepted":
      return "accepted";
    case "Rejected":
      return "rejected";
    case "Cancelled":
      return "cancelled";
    case "Completed":
      return "done";
    default:
      return "open";
  }
}

/** A PDF colour (`/C`: gray, RGB or CMYK, each 0 to 1) as CSS; none for no colour. */
function cssColour(c: readonly number[]): string | undefined {
  const pct = (v: number | undefined) => `${Math.round(Math.min(1, Math.max(0, v ?? 0)) * 100)}%`;
  if (c.length === 1) return `rgb(${pct(c[0])} ${pct(c[0])} ${pct(c[0])})`;
  if (c.length === 3) return `rgb(${pct(c[0])} ${pct(c[1])} ${pct(c[2])})`;
  if (c.length === 4) {
    const [cy = 0, m = 0, y = 0, k = 0] = c;
    return `rgb(${pct((1 - cy) * (1 - k))} ${pct((1 - m) * (1 - k))} ${pct((1 - y) * (1 - k))})`;
  }
  return undefined;
}

/** The threads `threads` as the panel shows them, in document order. */
export function threadViews(threads: readonly CommentThread[]): ThreadView[] {
  return threads.map((t) => {
    const status = statusOf(t.review);
    // Who set the status: the latest Review state annotation's author.
    const set = [...t.states]
      .reverse()
      .find((s) => s.annotation.state?.model === "Review" && s.annotation.state.state === t.review);
    return {
      id: t.comment.id,
      page: t.page,
      subtype: t.comment.subtype,
      kind: kindOf(t.comment.subtype),
      colour: cssColour(t.comment.color),
      author: t.comment.author ?? undefined,
      date: t.comment.modified ?? t.comment.created ?? undefined,
      note: t.comment.contents ?? "",
      quoted: t.quoted ?? undefined,
      status,
      statusBy: status === "open" ? undefined : (set?.annotation.author ?? undefined),
      checked: t.marked === "Marked",
      replies: t.replies.map((r) => ({
        id: r.annotation.id,
        author: r.annotation.author ?? undefined,
        date: r.annotation.modified ?? r.annotation.created ?? undefined,
        text: r.annotation.contents ?? "",
      })),
      broken: t.broken !== undefined,
    };
  });
}

/** Whether `thread` holds `query` in its note, replies, quoted text or author, ignoring case. */
export function matches(thread: ThreadView, query: string): boolean {
  const q = query.trim().toLocaleLowerCase();
  if (q === "") return true;
  const words = [thread.note, thread.quoted ?? "", thread.author ?? "", ...thread.replies.map((r) => r.text)];
  return words.some((w) => w.toLocaleLowerCase().includes(q));
}

/** Whether `filter` keeps `thread`. */
function kept(thread: ThreadView, filter: ThreadFilter): boolean {
  const { statuses, authors, kinds, pages } = filter;
  return (
    (statuses.size === 0 || statuses.has(thread.status)) &&
    (authors.size === 0 || authors.has(thread.author ?? "")) &&
    (kinds.size === 0 || kinds.has(thread.kind)) &&
    (pages === undefined || (thread.page >= pages[0] && thread.page <= pages[1]))
  );
}

/** The threads `filter` keeps that hold `query`, in `sort`'s order. */
export function shownThreads(
  threads: readonly ThreadView[],
  filter: ThreadFilter,
  query: string,
  sort: ThreadSort,
): ThreadView[] {
  const shown = threads.filter((t) => kept(t, filter) && matches(t, query));
  if (sort === "page") return shown;
  // A stable sort: ties keep document order.
  const by = (t: ThreadView) => (sort === "newest" ? (t.date ?? "") : (t.author ?? "").toLocaleLowerCase());
  return shown.sort((a, b) => {
    const [x, y] = [by(a), by(b)];
    if (x === y) return 0;
    return sort === "newest" ? (x < y ? 1 : -1) : x < y ? -1 : 1;
  });
}

/** How many of `threads` have each status, author and kind: the filter menu's counts. */
export function counts(threads: readonly ThreadView[]): {
  readonly statuses: ReadonlyMap<ThreadStatus, number>;
  readonly authors: ReadonlyMap<string, number>;
  readonly kinds: ReadonlyMap<ThreadKind, number>;
} {
  const statuses = new Map<ThreadStatus, number>();
  const authors = new Map<string, number>();
  const kinds = new Map<ThreadKind, number>();
  const add = <K>(map: Map<K, number>, key: K) => map.set(key, (map.get(key) ?? 0) + 1);
  for (const t of threads) {
    add(statuses, t.status);
    add(authors, t.author ?? "");
    add(kinds, t.kind);
  }
  return { statuses, authors, kinds };
}

/** Whether `filter` filters anything. */
export function filtering(filter: ThreadFilter): boolean {
  return filter.statuses.size + filter.authors.size + filter.kinds.size > 0 || filter.pages !== undefined;
}

/** The time an annotation was made, in the reader's words: "29 Oct 2026, 09:12". */
export function madeAt(date: string | undefined): string | undefined {
  const m = date?.match(/^D:(\d{4})(\d{2})?(\d{2})?(\d{2})?(\d{2})?(\d{2})?/);
  if (m === null || m === undefined) return undefined;
  const [, y, mo = "01", d = "01", h = "00", mi = "00", s = "00"] = m;
  const when = new Date(Date.UTC(Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s)));
  return when.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}
