// The Comments panel in the right dock (Paper C04), over the comment controller: the document's
// threads read through its worker, again after every edit, undo or redo; the search, filters and
// sort kept here while the panel is open; and the thread open, the comment selected on the page.

import { AnchoredMenu, type MenuEntry, type ModeRegistry, usePhone, useStoreState } from "@laminapdf/ui";
import {
  COMMENT_STATUSES,
  type CommentFilterSection,
  type CommentPageGroup,
  type CommentSort,
  type CommentStatus,
  CommentsPanel,
  type CommentThreadActions,
  CommentThreadDetail,
  type CommentThreadItem,
  CommentThreadSummary,
  laterMessage,
  type StatusChoice,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useState } from "react";
import { useEditStep } from "../edit-step.ts";
import { KEYS } from "../keys.ts";
import type { CommentController } from "./controller.ts";
import { kindName } from "./ops.ts";
import {
  counts,
  KINDS,
  madeAt,
  NO_FILTER,
  shownThreads,
  type ThreadFilter,
  type ThreadKind,
  type ThreadView,
  threadOf,
  threadViews,
} from "./threads.ts";

/** The document's threads, read again after every step. */
function useThreads(controller: CommentController, viewer: Viewer): readonly ThreadView[] | undefined {
  const step = useEditStep(viewer);
  const [threads, setThreads] = useState<readonly ThreadView[] | undefined>(undefined);
  useEffect(() => {
    let current = true;
    controller.threads().then(
      (read) => {
        if (!current) return;
        const views = threadViews(read);
        controller.threadsRead(views);
        setThreads(views);
      },
      // A read that fails keeps the last list; the first one shows none.
      () => {
        if (current) setThreads((last) => last ?? []);
      },
    );
    return () => {
      current = false;
    };
    // `step` is the dependency that reads them again.
  }, [controller, step]);
  return threads;
}

/** What a thread's Reply, Status and Mark as checked do, through `controller`. */
function threadActions(controller: CommentController): CommentThreadActions {
  return {
    onReply: (id, text) => void controller.addReply(id, text),
    onStatus: (id, status) => void controller.setStatus([id], status),
    onChecked: (id, checked) => void controller.setChecked(id, checked),
  };
}

/** A thread as the panel shows it. */
function item(t: ThreadView): CommentThreadItem {
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
      options: COMMENT_STATUSES.map((s) => ({
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
      return { ...filter, statuses: toggled(filter.statuses, option as CommentStatus, on) };
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
  const phone = usePhone();
  const ticked = useStoreState(controller, (s) => s.ticked);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<CommentSort>("page");
  const [filter, setFilter] = useState<ThreadFilter>(NO_FILTER);
  const all = threads ?? [];
  const shown = useMemo(() => shownThreads(all, filter, query, sort), [all, filter, query, sort]);
  // A reply selected on the page opens the thread it answers.
  const expanded = threadOf(all, selection[0]?.id)?.id;

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
      onGo={(id) => {
        const page = all.find((t) => t.id === id)?.page;
        if (page !== undefined) controller.goTo(page, id);
      }}
      actions={threadActions(controller)}
      onFocused={() => controller.focused()}
      headed={!phone}
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
  const thread = threadOf(threads ?? [], id);
  if (thread === undefined) return null;
  const shown = item(thread);
  return (
    <>
      <CommentThreadSummary thread={shown} />
      <CommentThreadDetail thread={shown} actions={threadActions(controller)} />
    </>
  );
}

/** What the selected comment's menu offers (C02's Shift+F10). */
type MenuChoice = "reply" | "accepted" | "rejected" | "done" | "show" | "delete";

/** The selected comment's menu, at its button on the page: Reply, a status, Show in Comments, Delete. */
export function AnnotationMenu({ controller }: { controller: CommentController }): ReactNode {
  const at = useStoreState(controller, (s) => s.menuAt);
  const id = useStoreState(controller, (s) => s.selection[0]?.id);
  const items: MenuEntry<MenuChoice>[] = [
    { id: "reply", label: laterMessage("comments.reply"), icon: "comment" },
    ...(["accepted", "rejected", "done"] as const).map((s) => ({ id: s, label: laterMessage(`comments.status.${s}`) })),
    { id: "show", label: laterMessage("comments.menu.show"), icon: "comments-list" },
    { id: "delete", label: laterMessage("comments.menu.delete"), icon: "delete", isDestructive: true },
  ];
  return (
    <AnchoredMenu
      at={at ?? null}
      open={at !== undefined}
      onClose={() => controller.closeMenu()}
      aria-label={laterMessage("comments.menu")}
      items={items}
      platform={KEYS}
      onAction={(choice) => {
        if (id === undefined) return;
        // After the menu has closed and put the focus back on the comment, so the field keeps it.
        if (choice === "reply") requestAnimationFrame(() => controller.focusPanel("reply"));
        else if (choice === "show") controller.showPanel();
        else if (choice === "delete") void controller.deleteThreads([id]);
        else void controller.setStatus([id], choice);
      }}
    />
  );
}

/** The Comments panel's command handlers (C04), over Comment mode's `on` and `withArgs`. */
export function panelActions(
  on: (run: (controller: CommentController) => void) => () => void,
  withArgs: (run: (controller: CommentController, args: unknown) => void) => (args: unknown) => void,
  modes: ModeRegistry,
) {
  return {
    // Read mode's More › Comments opens it in Comment mode.
    "comments.toggle-panel": () => {
      if (modes.getState().current !== "comment") modes.switchTo("comment");
      on((c) => c.togglePanel())();
    },
    "comments.search": on((c) => c.focusPanel("search")),
    "comments.filter": on((c) => c.focusPanel("filter")),
    "comments.sort": on((c) => c.focusPanel("sort")),
    "comments.delete-selected": on((c) => void c.deleteThreads([...c.getState().ticked])),
    "comment.go-to": on((c) => c.goToSelected()),
    "comment.reply": on((c) => c.focusPanel("reply")),
    "comment.set-status": withArgs((c, args) => {
      const id = c.selectedId();
      if (id !== undefined) void c.setStatus([id], args as StatusChoice);
    }),
  };
}
