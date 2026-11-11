// The History rail tab (Paper O01, design language §8.10): a heading with the step count, the
// notice of a partial undo until the next edit, then an ordered list of buttons: "Opened file"
// first, one row per step (label, page, time), the current point tinted (`aria-current="step"`),
// undone steps struck through. Choosing a row undoes or redoes to that point in one call. Up and
// Down move between rows, Home and End to the ends, Enter or Space goes to the step. The list holds
// up to 500 steps, so only the rows in view (and a few either side) are in the DOM; each row says
// where it sits in the whole list.

import {
  type FocusEvent,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { Notice } from "../components/notice.js";
import { countMessage, message } from "../messages.js";
import { useStoreState } from "../state/hooks.js";
import type { EditControls, EditStepSlice } from "./edit-controls.js";
import { revertedText } from "./outcome.js";

/** Rows drawn above and below the ones in view. */
const OVERSCAN = 8;
/** A row's height before one is measured, in CSS px: two lines and their padding. */
const ROW_GUESS = 48;

/** Props of `HistoryPanel`. */
export interface HistoryPanelProps {
  /** The open document's edit session. */
  readonly edits: EditControls;
  /** When the document was opened (ms since the epoch), for the first row's time. */
  readonly openedAt?: number;
  /** A page's name in a row ("5", or its label), from its 0-based index; its number by default. */
  readonly pageName?: (index: number) => string;
  /** A time as rows show it; hours and minutes in the reader's locale by default. */
  readonly formatTime?: (at: number) => string;
  /** The heading's level; 2 by default. */
  readonly headingLevel?: 2 | 3;
  /** Called after a row is chosen (a rail over the page closes). */
  readonly onNavigate?: () => void;
}

const TIME = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });

/** One row: the file as opened (`step` undefined), or the state after a step. */
interface Row {
  readonly index: number;
  readonly step: EditStepSlice | undefined;
}

/** The pages part of a row's meta line: "Page 5", "Pages 1, 3". */
function pagesText(pages: readonly number[], name: (index: number) => string): string | undefined {
  if (pages.length === 0) return undefined;
  if (pages.length === 1) return message("edit.history.page", { page: name(pages[0] as number) });
  return message("edit.history.pages", { pages: pages.map(name).join(", ") });
}

/** The edit session's history, as a list to go back and forward through. */
export function HistoryPanel(props: HistoryPanelProps): ReactNode {
  const { edits, headingLevel = 2 } = props;
  const history = useStoreState(edits, (s) => s.history);
  const notice = useStoreState(edits, (s) => s.notice);
  const busy = useStoreState(edits, (s) => s.busy);
  const Heading = headingLevel === 2 ? "h2" : "h3";
  const count = history.steps.length;
  return (
    <section className="paper-history" aria-busy={busy || undefined}>
      <header className="paper-history-header">
        <Heading className="paper-history-heading">{message("edit.history")}</Heading>
        <span className="paper-history-count">
          {countMessage(count, { one: "edit.history.one", many: "edit.history.many" })}
        </span>
      </header>
      {notice === undefined ? null : <Notice title={revertedText(notice)} tone="warning" />}
      <HistoryList {...props} />
      <p className="paper-history-footer">{message("edit.history.footer")}</p>
    </section>
  );
}

/** The rows in view: from `first` up to (not including) `last`. */
function useWindow(
  scroller: RefObject<HTMLDivElement | null>,
  total: number,
  rowHeight: number,
): { first: number; last: number } {
  const [view, setView] = useState({ top: 0, height: rowHeight * 12 });
  useEffect(() => {
    const element = scroller.current;
    if (element === null) return;
    const update = () => setView({ top: element.scrollTop, height: element.clientHeight || rowHeight * 12 });
    update();
    element.addEventListener("scroll", update, { passive: true });
    const resize = typeof ResizeObserver === "function" ? new ResizeObserver(update) : undefined;
    resize?.observe(element);
    return () => {
      element.removeEventListener("scroll", update);
      resize?.disconnect();
    };
  }, [scroller, rowHeight]);
  const first = Math.max(0, Math.floor(view.top / rowHeight) - OVERSCAN);
  const last = Math.min(total, Math.ceil((view.top + view.height) / rowHeight) + OVERSCAN);
  return { first, last };
}

/** The rows' height, measured once one is drawn; a guess until then. */
function useRowHeight(list: RefObject<HTMLOListElement | null>): number {
  const [rowHeight, setRowHeight] = useState(ROW_GUESS);
  useLayoutEffect(() => {
    const row = list.current?.querySelector<HTMLElement>("li");
    if (row !== null && row !== undefined && row.offsetHeight > 0 && row.offsetHeight !== rowHeight) {
      setRowHeight(row.offsetHeight);
    }
  });
  return rowHeight;
}

/** What the arrow, Home, End and page keys do to the focused row `focus` of `total`. */
function keyTarget(key: string, focus: number, total: number): number | undefined {
  const keys: Record<string, number | undefined> = {
    ArrowDown: focus + 1,
    ArrowUp: focus - 1,
    Home: 0,
    End: total - 1,
    PageDown: focus + 10,
    PageUp: focus - 10,
  };
  const to = keys[key];
  return to === undefined ? undefined : Math.max(0, Math.min(total - 1, to));
}

/**
 * The row that takes Tab (one at a time), moved by the keys and focused once drawn; it follows the
 * current row while focus is elsewhere, and the current row stays in view.
 */
function useRovingFocus(
  refs: { readonly list: RefObject<HTMLOListElement | null>; readonly scroller: RefObject<HTMLDivElement | null> },
  rows: { readonly total: number; readonly position: number; readonly height: number; readonly drawn: string },
) {
  const [focus, setFocus] = useState(rows.position);
  const focusing = useRef(false);
  useEffect(() => {
    if (!focusing.current) setFocus(rows.position);
    reveal(refs.scroller.current, rows.position, rows.height);
  }, [refs, rows.position, rows.height]);
  // `drawn` names the rows in the DOM: a row the keys reached is focused once it is one of them.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `rows.drawn` changes when the window of rows does
  useLayoutEffect(() => {
    if (!focusing.current) return;
    refs.list.current?.querySelector<HTMLButtonElement>(`[data-row="${focus}"]`)?.focus();
  }, [refs, focus, rows.drawn]);
  const onKeyDown = (event: KeyboardEvent<HTMLOListElement>) => {
    const to = keyTarget(event.key, focus, rows.total);
    if (to === undefined) return;
    event.preventDefault();
    focusing.current = true;
    setFocus(to);
    reveal(refs.scroller.current, to, rows.height);
  };
  const onFocus = () => {
    focusing.current = true;
  };
  const onBlur = (event: FocusEvent<HTMLOListElement>) => {
    if (!event.currentTarget.contains(event.relatedTarget as Node | null)) focusing.current = false;
  };
  return { focus, setFocus, onKeyDown, onFocus, onBlur };
}

/** The list itself, virtualised, with a roving focus. */
function HistoryList(props: HistoryPanelProps): ReactNode {
  const { edits } = props;
  const history = useStoreState(edits, (s) => s.history);
  const name = props.pageName ?? ((index: number) => String(index + 1));
  const time = props.formatTime ?? ((at: number) => TIME.format(at));
  const scroller = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLOListElement>(null);
  const [refs] = useState(() => ({ list, scroller }));
  const rowHeight = useRowHeight(list);
  const total = history.steps.length + 1;
  const { first, last } = useWindow(scroller, total, rowHeight);
  const { focus, setFocus, onKeyDown, onFocus, onBlur } = useRovingFocus(refs, {
    total,
    position: history.position,
    height: rowHeight,
    drawn: `${first}-${last}`,
  });
  const rows: Row[] = [];
  for (let index = first; index < last; index++) rows.push({ index, step: history.steps[index - 1] });
  const choose = (index: number) => {
    setFocus(index);
    // A failure leaves the history as it was; the session tells its listeners, who say so.
    void edits.goTo(index).catch(() => undefined);
    props.onNavigate?.();
  };
  // The roving Tab stop, kept on a drawn row: when scrolling takes the focused row out of the
  // DOM, the first drawn row takes Tab, so the list is never left without one.
  const tabbable = focus >= first && focus < last ? focus : first;
  return (
    <div ref={scroller} className="paper-history-scroll">
      <ol
        ref={list}
        className="paper-history-list"
        aria-label={message("edit.history.list")}
        onKeyDown={onKeyDown}
        onFocus={onFocus}
        onBlur={onBlur}
        // The rows out of view take their room as padding, sized from the measured row height at run
        // time. React sets it through the CSSOM (element.style), as ThumbnailStrip sizes its
        // spacer, which the CSP's style-src allows; a stylesheet can't know the numbers.
        style={{ paddingBlockStart: first * rowHeight, paddingBlockEnd: (total - last) * rowHeight }}
      >
        {rows.map((row) => (
          <HistoryRow
            key={row.step?.id ?? "opened"}
            row={row}
            total={total}
            dropped={history.dropped > 0}
            tabbable={row.index === tabbable}
            meta={metaOf(row, history.position, history.dropped > 0, { name, time, openedAt: props.openedAt })}
            onChoose={choose}
            onFocusRow={setFocus}
          />
        ))}
      </ol>
    </div>
  );
}

/** Scrolls `scroller` so row `index` is in view. */
function reveal(scroller: HTMLDivElement | null, index: number, rowHeight: number): void {
  if (scroller === null) return;
  const top = index * rowHeight;
  const bottom = top + rowHeight;
  if (top < scroller.scrollTop) scroller.scrollTop = top;
  else if (bottom > scroller.scrollTop + scroller.clientHeight) scroller.scrollTop = bottom - scroller.clientHeight;
}

/** What a row shows of where it stands: done or undone, the current one, and its meta line. */
interface RowMeta {
  readonly text: string;
  readonly current: boolean;
  readonly undone: boolean;
  /** Its caption warns: the step was restored only in part. */
  readonly partial: boolean;
}

/**
 * A row's state and meta line: the opened file's "No changes" (only its time once the oldest
 * steps were dropped, as the row then stands for them); else its pages, then "Undone", "Partly
 * undone" (an undo that left part of it alone) or "Partly redone", then the time.
 */
function metaOf(
  row: Row,
  position: number,
  dropped: boolean,
  format: {
    readonly name: (index: number) => string;
    readonly time: (at: number) => string;
    readonly openedAt: number | undefined;
  },
): RowMeta {
  const current = row.index === position;
  const undone = row.index > position;
  const join = (parts: readonly (string | undefined)[]) => parts.filter((p) => p !== undefined).join(" · ");
  const { step } = row;
  if (step === undefined) {
    const time = format.openedAt === undefined ? undefined : format.time(format.openedAt);
    const text = dropped ? "" : join([message("edit.history.noChanges"), time]);
    return { text, current, undone, partial: false };
  }
  const partial = (undone && step.partial === "undo") || (!undone && step.partial === "redo");
  const state = partial
    ? message(undone ? "edit.history.partial" : "edit.history.partialRedo")
    : undone
      ? message("edit.history.undone")
      : undefined;
  return { text: join([pagesText(step.pages, format.name), state, format.time(step.at)]), current, undone, partial };
}

function HistoryRow({
  row,
  total,
  dropped,
  tabbable,
  meta,
  onChoose,
  onFocusRow,
}: {
  row: Row;
  total: number;
  dropped: boolean;
  tabbable: boolean;
  meta: RowMeta;
  onChoose: (index: number) => void;
  onFocusRow: (index: number) => void;
}): ReactNode {
  const { current, undone } = meta;
  const label = row.step?.label ?? (dropped ? message("edit.history.dropped") : message("edit.history.opened"));
  return (
    <li className="paper-history-item" aria-setsize={total} aria-posinset={row.index + 1}>
      <button
        type="button"
        className="paper-history-row"
        data-row={row.index}
        data-current={current || undefined}
        data-undone={undone || undefined}
        aria-current={current ? "step" : undefined}
        tabIndex={tabbable ? 0 : -1}
        onClick={() => onChoose(row.index)}
        onFocus={() => onFocusRow(row.index)}
      >
        <span className="paper-history-label">{label}</span>
        {meta.text === "" ? null : (
          <span className="paper-history-meta" data-partial={meta.partial || undefined}>
            {meta.text}
          </span>
        )}
      </button>
    </li>
  );
}
