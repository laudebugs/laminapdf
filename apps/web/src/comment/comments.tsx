// The Comments panel in the right dock (Paper C04), over the comment controller: the document's
// threads read through its worker, again after every edit, undo or redo; the search, filters and
// sort kept here while the panel is open; and the thread open, the comment selected on the page.

import { useStoreState } from "@laminapdf/ui";
import {
  type CommentFilterSection,
  type CommentPageGroup,
  type CommentSort,
  CommentsPanel,
  CommentThreadDetail,
  type CommentThreadItem,
  laterMessage,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useState, useSyncExternalStore } from "react";
import type { CommentController } from "./controller.ts";
import { kindName } from "./ops.ts";
import {
  counts,
  KINDS,
  madeAt,
  NO_FILTER,
  STATUSES,
  shownThreads,
  type ThreadFilter,
  type ThreadKind,
  type ThreadStatus,
  type ThreadView,
  threadViews,
} from "./threads.ts";

/** A key that changes whenever the edit session makes, undoes or redoes a step. */
function useEditStep(viewer: Viewer): string {
  return useSyncExternalStore(
    (listener) => viewer.edits.subscribe(listener),
    () => {
      const { history } = viewer.edits.getState();
      return `${history.dropped}:${history.steps.length}:${history.position}`;
    },
  );
}

/** The document's threads, read again after every step. */
export function useThreads(controller: CommentController, viewer: Viewer): readonly ThreadView[] | undefined {
  const step = useEditStep(viewer);
  const [threads, setThreads] = useState<readonly ThreadView[] | undefined>(undefined);
  useEffect(() => {
    let current = true;
    controller.threads().then(
      (read) => {
        if (current) setThreads(threadViews(read));
      },
      () => {
        if (current) setThreads([]);
      },
    );
    return () => {
      current = false;
    };
    // `step` is the dependency that reads them again.
  }, [controller, step]);
  return threads;
}

/** A thread as the panel shows it. */
export function item(t: ThreadView): CommentThreadItem {
  return {
    id: t.id,
    kindLabel: kindName(t.subtype),
    colour: t.colour,
    author: t.author,
    when: madeAt(t.date),
    status: t.status,
    statusBy: t.statusBy,
    checked: t.checked,
    quoted: t.quoted,
    note: t.note,
    replies: t.replies.map((r) => ({ id: r.id, author: r.author, when: madeAt(r.date), text: r.text })),
  };
}

/** The shown threads by page, each page under its heading; one list for another sort. */
function grouped(shown: readonly ThreadView[], sort: CommentSort): CommentPageGroup[] {
  if (sort !== "page") return [{ title: undefined, threads: shown.map(item) }];
  const groups: { page: number; threads: CommentThreadItem[] }[] = [];
  for (const t of shown) {
    const last = groups.at(-1);
    if (last?.page === t.page) last.threads.push(item(t));
    else groups.push({ page: t.page, threads: [item(t)] });
  }
  return groups.map((g) => ({ title: laterMessage("comments.page", { page: g.page + 1 }), threads: g.threads }));
}

/** `set` with `value` added or taken out. */
function toggled<T>(set: ReadonlySet<T>, value: T, on: boolean): Set<T> {
  const out = new Set(set);
  if (on) out.add(value);
  else out.delete(value);
  return out;
}

/** The Filter menu's sections with their counts, and which are on. */
function filterSections(
  threads: readonly ThreadView[],
  filter: ThreadFilter,
  you: string | undefined,
): CommentFilterSection[] {
  const c = counts(threads);
  return [
    {
      id: "status",
      label: laterMessage("comments.filter.status"),
      options: STATUSES.map((s) => ({
        id: s,
        label: laterMessage(`comments.status.${s}`),
        count: c.statuses.get(s) ?? 0,
        on: filter.statuses.has(s),
      })),
    },
    {
      id: "author",
      label: laterMessage("comments.filter.author"),
      options: [...c.authors].map(([name, count]) => ({
        id: name,
        label:
          name === ""
            ? laterMessage("comments.filter.nobody")
            : name === you
              ? laterMessage("comments.filter.you", { name })
              : name,
        count,
        on: filter.authors.has(name),
      })),
    },
    {
      id: "kind",
      label: laterMessage("comments.filter.kind"),
      options: KINDS.map((k) => ({
        id: k,
        label: laterMessage(`comments.kind.${k}`),
        count: c.kinds.get(k) ?? 0,
        on: filter.kinds.has(k),
      })),
    },
  ];
}

/** `filter` with one choice turned on or off. */
function filterWith(filter: ThreadFilter, section: string, option: string, on: boolean): ThreadFilter {
  switch (section) {
    case "status":
      return { ...filter, statuses: toggled(filter.statuses, option as ThreadStatus, on) };
    case "author":
      return { ...filter, authors: toggled(filter.authors, option, on) };
    case "kind":
      return { ...filter, kinds: toggled(filter.kinds, option as ThreadKind, on) };
    default:
      return filter;
  }
}

/** The Comments panel over `controller`, in the right dock. */
export function CommentsDock({ controller, viewer }: { controller: CommentController; viewer: Viewer }): ReactNode {
  const threads = useThreads(controller, viewer);
  const selection = useStoreState(controller, (s) => s.selection);
  const focus = useStoreState(controller, (s) => s.panelFocus);
  const ticked = useStoreState(controller, (s) => s.ticked);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<CommentSort>("page");
  const [filter, setFilter] = useState<ThreadFilter>(NO_FILTER);
  const all = threads ?? [];
  const shown = useMemo(() => shownThreads(all, filter, query, sort), [all, filter, query, sort]);
  const expanded = selection[0]?.id;
  const pageOf = (id: string) => all.find((t) => t.id === id)?.page ?? 0;
  return (
    <CommentsPanel
      total={all.length}
      open={all.filter((t) => t.status === "open").length}
      groups={grouped(shown, sort)}
      query={query}
      onQuery={setQuery}
      sort={sort}
      onSort={setSort}
      filters={filterSections(all, filter, controller.author())}
      onFilter={(section, option, on) => setFilter((f) => filterWith(f, section, option, on))}
      onClearFilters={() => {
        setFilter(NO_FILTER);
        setQuery("");
      }}
      expanded={expanded}
      focus={focus}
      onGo={(id) => controller.goTo(pageOf(id), id)}
      actions={{
        onReply: (id, text) => void controller.addReply(id, text),
        onStatus: (id, status) => void controller.setStatus([id], status),
        onChecked: (id, checked) => void controller.setChecked(id, checked),
      }}
      selected={ticked}
      onSelect={(ids, on) => controller.tick(ids, on)}
      onSetStatus={(ids, status) => void controller.setStatus(ids, status)}
      onDelete={(ids) => void controller.deleteThreads(ids)}
      broken={all.some((t) => t.broken)}
      onClose={() => controller.togglePanel()}
    />
  );
}

/** The selected comment's replies, Reply and Status, as the phone's comment sheet shows them (C08). */
export function SelectedThread({ controller, viewer }: { controller: CommentController; viewer: Viewer }): ReactNode {
  const threads = useThreads(controller, viewer);
  const id = useStoreState(controller, (s) => s.selection[0]?.id);
  const thread = threads?.find((t) => t.id === id);
  if (thread === undefined) return null;
  return (
    <CommentThreadDetail
      thread={item(thread)}
      actions={{
        onReply: (target, text) => void controller.addReply(target, text),
        onStatus: (target, status) => void controller.setStatus([target], status),
        onChecked: (target, checked) => void controller.setChecked(target, checked),
      }}
    />
  );
}
