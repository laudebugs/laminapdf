// The Comments panel (Paper C04): the right dock's list of every comment as threads, grouped by
// page. A header with the counts and Close; a search field, the Filter menu (status, author, type,
// as checkbox items with counts) whose choices show as chips, and the sort. Each thread's header
// goes to its comment; the thread open shows its replies, Reply, Mark as checked and Status.
// Ticking threads shows a footer to set their status or delete them, after a question.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useState } from "react";
import {
  Button as AriaButton,
  Checkbox as AriaCheckbox,
  Menu as AriaMenu,
  Popover as AriaPopover,
  Header,
  MenuItem,
  MenuSection,
  MenuTrigger,
  Text,
} from "react-aria-components";
import { Button, IconButton } from "../components/button.js";
import { Checkbox } from "../components/choice.js";
import { EmptyState } from "../components/empty-state.js";
import { Select, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { POPOVER_LAYER } from "../overlays/popover.js";
import { QuestionDialog } from "../overlays/question-dialog.js";
import {
  type CommentThreadActions,
  CommentThreadDetail,
  CommentThreadHeading,
  type CommentThreadItem,
  CommentThreadSummary,
  ReplyCount,
  STATUS_CHOICES,
  type StatusChoice,
} from "./comment-thread.js";

/** How the threads are ordered. */
export type CommentSort = "page" | "newest" | "author";

/** The threads of one page, or of every page when the sort isn't by page. */
export interface CommentPageGroup {
  /** Its heading, "Page 3"; none for a list that isn't grouped. */
  readonly title: string | undefined;
  readonly threads: readonly CommentThreadItem[];
}

/** One choice of the Filter menu, with how many threads it matches. */
export interface CommentFilterOption {
  readonly id: string;
  readonly label: string;
  readonly count: number;
  readonly on: boolean;
}

/** One group of the Filter menu. */
export interface CommentFilterSection {
  readonly id: string;
  readonly label: string;
  readonly options: readonly CommentFilterOption[];
}

/** Props of `CommentsPanel`. */
export interface CommentsPanelProps {
  /** Every thread in the file, and how many are open. */
  readonly total: number;
  readonly open: number;
  /** The groups shown, after the search and the filters. */
  readonly groups: readonly CommentPageGroup[];
  readonly query: string;
  readonly onQuery: (query: string) => void;
  readonly sort: CommentSort;
  readonly onSort: (sort: CommentSort) => void;
  readonly filters: readonly CommentFilterSection[];
  /** Turns filter choice `option` of `section` on or off. */
  readonly onFilter: (section: string, option: string, on: boolean) => void;
  readonly onClearFilters: () => void;
  /** The thread shown open: the comment selected on the page. */
  readonly expanded: string | undefined;
  /** Whether the open thread's Reply field takes the focus (Reply asked for it). */
  readonly focusReply: boolean;
  /** Goes to a thread's comment: selects it and scrolls the page to it. */
  readonly onGo: (id: string) => void;
  readonly actions: CommentThreadActions;
  /** The threads ticked for a bulk action. */
  readonly selected: ReadonlySet<string>;
  readonly onSelect: (ids: readonly string[], on: boolean) => void;
  readonly onSetStatus: (ids: readonly string[], status: StatusChoice) => void;
  readonly onDelete: (ids: readonly string[]) => void;
  /** Whether some thread's parent is missing or its replies loop: the notice shows. */
  readonly broken: boolean;
  readonly onClose: () => void;
}

/** The Filter menu: checkbox items with counts, by section; choosing toggles one. */
function FilterMenu({ filters, onFilter }: Pick<CommentsPanelProps, "filters" | "onFilter">): ReactNode {
  const on = filters.flatMap((s) => s.options.filter((o) => o.on).map((o) => `${s.id}:${o.id}`));
  return (
    <MenuTrigger>
      <AriaButton className="paper-button" data-variant="secondary" data-size="32">
        <Icon name="filter" size={16} />
        {on.length === 0
          ? laterMessage("comments.filter")
          : laterMessage("comments.filter.active", { count: on.length })}
        <Icon name="chevron-down" size={16} />
      </AriaButton>
      <AriaPopover className="paper-popover paper-menu-popover" placement="bottom start" style={POPOVER_LAYER}>
        <AriaMenu
          className="paper-menu"
          aria-label={laterMessage("comments.filter")}
          selectionMode="multiple"
          selectedKeys={on}
          onAction={(key) => {
            const [section = "", option = ""] = String(key).split(/:(.*)/s);
            const now = on.includes(String(key));
            onFilter(section, option, !now);
          }}
        >
          {filters.map((section) => (
            <MenuSection key={section.id} id={section.id}>
              <Header className="paper-menu-reason">{section.label}</Header>
              {section.options.map((o) => (
                <MenuItem key={o.id} id={`${section.id}:${o.id}`} textValue={o.label} className="paper-menu-item">
                  <Text slot="label" className="paper-menu-label">
                    {o.label}
                  </Text>
                  <span className="paper-comments-filter-count">{o.count}</span>
                </MenuItem>
              ))}
            </MenuSection>
          ))}
        </AriaMenu>
      </AriaPopover>
    </MenuTrigger>
  );
}

/** The filters on, as removable chips, with Clear. */
function FilterChips({ filters, onFilter, onClearFilters }: CommentsPanelProps): ReactNode {
  const on = filters.flatMap((s) => s.options.filter((o) => o.on).map((o) => ({ section: s.id, option: o })));
  if (on.length === 0) return null;
  return (
    <div className="paper-comments-chips">
      {on.map(({ section, option }) => (
        <AriaButton
          key={`${section}:${option.id}`}
          className="paper-comments-chip"
          aria-label={laterMessage("comments.filter.remove", { filter: option.label })}
          onPress={() => onFilter(section, option.id, false)}
        >
          {option.label}
          <Icon name="close" size={16} />
        </AriaButton>
      ))}
      <Button variant="ghost" size={32} onPress={onClearFilters}>
        {laterMessage("comments.filter.clear")}
      </Button>
    </div>
  );
}

/** One thread in the list: a tick, the header that goes to it, and its body, open or not. */
function ThreadEntry({ thread, props }: { thread: CommentThreadItem; props: CommentsPanelProps }): ReactNode {
  const open = props.expanded === thread.id;
  return (
    <li className="paper-comment-thread" data-open={open || undefined}>
      <div className="paper-comment-thread-head">
        <AriaCheckbox
          className="paper-comment-thread-tick"
          aria-label={laterMessage("comments.select", { thread: `${thread.kindLabel}, ${thread.author ?? ""}` })}
          isSelected={props.selected.has(thread.id)}
          onChange={(on) => props.onSelect([thread.id], on)}
        >
          <span className="paper-checkbox-box" aria-hidden="true">
            <Icon name="check" size={16} />
          </span>
        </AriaCheckbox>
        <AriaButton className="paper-comment-thread-go" onPress={() => props.onGo(thread.id)}>
          <CommentThreadHeading thread={thread} />
        </AriaButton>
      </div>
      <div className="paper-comment-thread-body">
        <CommentThreadSummary thread={thread} />
        {open ? (
          <CommentThreadDetail thread={thread} actions={props.actions} focusReply={props.focusReply} />
        ) : (
          <ReplyCount count={thread.replies.length} />
        )}
      </div>
    </li>
  );
}

/** The footer while threads are ticked: how many, Set status and Delete n…. */
function BulkBar({ ids, props }: { ids: readonly string[]; props: CommentsPanelProps }): ReactNode {
  const [asking, setAsking] = useState(false);
  if (ids.length === 0) return null;
  const count = ids.length;
  return (
    <div className="paper-comments-bulk" role="toolbar" aria-label={laterMessage("comments.selected", { count })}>
      <span className="paper-comments-bulk-count">
        {laterCountMessage(count, { one: "comments.selected.one", many: "comments.selected" })}
      </span>
      <Select
        label={laterMessage("comments.selected.status")}
        options={STATUS_CHOICES.map((s) => ({ value: s, label: laterMessage(`comments.status.${s}`) }))}
        value="none"
        onChange={(s) => props.onSetStatus(ids, s)}
      />
      <Button variant="danger" size={32} onPress={() => setAsking(true)}>
        {laterMessage("comments.delete", { count })}
      </Button>
      <QuestionDialog
        open={asking}
        title={laterCountMessage(count, { one: "comments.delete.title.one", many: "comments.delete.title" })}
        description={laterMessage("comments.delete.description")}
        no={laterMessage("comments.delete.no")}
        yes={laterMessage("comments.delete.yes", { count })}
        yesVariant="danger-filled"
        onAnswer={(yes) => {
          setAsking(false);
          if (yes) props.onDelete(ids);
        }}
      />
    </div>
  );
}

/** The Comments panel (C04). */
export function CommentsPanel(props: CommentsPanelProps): ReactNode {
  const { total, groups } = props;
  const shown = groups.flatMap((g) => g.threads);
  const filtered = props.query.trim() !== "" || props.filters.some((s) => s.options.some((o) => o.on));
  const ticked = shown.filter((t) => props.selected.has(t.id)).map((t) => t.id);
  const all = shown.length > 0 && ticked.length === shown.length;
  return (
    <section className="paper-comments" aria-labelledby="paper-comments-title">
      <header className="paper-comments-header">
        <div>
          <h2 id="paper-comments-title" className="paper-comments-title">
            {laterMessage("comments.title")}
          </h2>
          <p className="paper-comments-count" aria-live="polite">
            {filtered
              ? laterMessage("comments.shown", { shown: shown.length, count: total })
              : laterCountMessage(total, { one: "comments.count.one", many: "comments.count" }, { open: props.open })}
          </p>
        </div>
        <IconButton icon="close" label={laterMessage("comments.close")} onPress={props.onClose} />
      </header>
      {total === 0 ? (
        <EmptyState title={laterMessage("comments.empty.title")} headingLevel={3}>
          {laterMessage("comments.empty.body")}
        </EmptyState>
      ) : (
        <>
          <div className="paper-comments-tools">
            <TextInput label={laterMessage("comments.search")} value={props.query} onChange={props.onQuery} />
            <div className="paper-comments-tools-row">
              <FilterMenu filters={props.filters} onFilter={props.onFilter} />
              <Select
                label={laterMessage("comments.sort")}
                options={(["page", "newest", "author"] as const).map((s) => ({
                  value: s,
                  label: laterMessage(`comments.sort.${s}`),
                }))}
                value={props.sort}
                onChange={props.onSort}
              />
            </div>
            <FilterChips {...props} />
          </div>
          {props.broken ? (
            <Notice tone="neutral" title={laterMessage("comments.broken.title")}>
              {laterMessage("comments.broken")}
            </Notice>
          ) : null}
          {shown.length === 0 ? (
            <EmptyState
              title={laterMessage("comments.none.title")}
              headingLevel={3}
              actions={
                <Button variant="ghost" onPress={props.onClearFilters}>
                  {laterMessage("comments.filter.clear")}
                </Button>
              }
            />
          ) : (
            <>
              <Checkbox
                isSelected={all}
                isIndeterminate={ticked.length > 0 && !all}
                onChange={(on) =>
                  props.onSelect(
                    shown.map((t) => t.id),
                    on,
                  )
                }
              >
                {laterMessage("comments.select.all")}
              </Checkbox>
              <fieldset className="paper-comments-list" aria-label={laterMessage("comments.threads")}>
                {groups.map((g, i) => (
                  <div key={g.title ?? i} className="paper-comments-page">
                    {g.title === undefined ? null : <h3 className="paper-comments-page-title">{g.title}</h3>}
                    <ul className="paper-comments-threads" aria-label={g.title ?? laterMessage("comments.threads")}>
                      {g.threads.map((t) => (
                        <ThreadEntry key={t.id} thread={t} props={props} />
                      ))}
                    </ul>
                  </div>
                ))}
              </fieldset>
            </>
          )}
          <BulkBar ids={ticked} props={props} />
        </>
      )}
    </section>
  );
}
