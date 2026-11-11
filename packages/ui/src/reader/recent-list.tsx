// The home screen's Recent list (R01): each recent document as a button, its name in Newsreader
// over a line saying it is on this device, how many pages it has and the page the reader stopped
// at, the date it was last opened at the end. Linked documents are never listed (ADR 0011). A local
// document the reader can't read again without a picker says so; pressing it asks for the file.
// Clear list empties the list in this browser; it sits beside Continue reading when that is the
// only document. The document open when the app last closed comes first, under Continue reading,
// with its page. The list is hidden when empty: a first visit needs
// no empty state.

import { type ReactNode, useId } from "react";
import { Button as AriaButton } from "react-aria-components";
import { Button } from "../components/button.js";
import { countMessage, message } from "../messages.js";
import type { RecentFile } from "../platform/types.js";

/** The document to continue: a recent entry and the page its reader stopped at (0-based). */
export interface ContinueReading {
  readonly entry: RecentFile;
  readonly page: number;
  readonly pageCount: number;
}

/** Props of `RecentList`. */
export interface RecentListProps {
  /** Newest first. */
  entries: readonly RecentFile[];
  /** An entry was pressed. */
  onOpen: (entry: RecentFile) => void;
  /** Clear list was pressed. */
  onClear: () => void;
  /** The page (0-based) each entry's reader stopped at, by entry id, where one is kept. */
  stoppedAt?: ReadonlyMap<string, number>;
  /** The document open when the app last closed, shown first under Continue reading. */
  continueReading?: ContinueReading | undefined;
  /** The locale dates are written in. */
  locale?: string;
}

/**
 * The line under the name: where, how many pages, the page the reader stopped at (0-based,
 * `stopped`), and whether the file must be chosen again.
 */
export function recentMeta(entry: RecentFile, reopens: boolean, stopped?: number): string {
  const pages =
    entry.pages === undefined ? [] : [countMessage(entry.pages, { one: "recent.page", many: "recent.pages" })];
  const at = stopped === undefined ? [] : [message("recent.stoppedAt", { page: stopped + 1 })];
  return [message("recent.thisDevice"), ...pages, ...at, ...(reopens ? [] : [message("recent.chooseAgain")])].join(
    " · ",
  );
}

/** One row: the name over its meta line, the date at the end. */
function Row({
  entry,
  meta,
  date,
  onOpen,
}: {
  entry: RecentFile;
  meta: string;
  date: Intl.DateTimeFormat;
  onOpen: (entry: RecentFile) => void;
}): ReactNode {
  return (
    <AriaButton className="paper-recent-row" onPress={() => onOpen(entry)}>
      <span className="paper-recent-name">{entry.name}</span>
      <span className="paper-recent-meta">{meta}</span>
      <span className="paper-recent-date">{date.format(entry.lastOpened)}</span>
    </AriaButton>
  );
}

/** The document to continue, with its page of its page count; Clear list beside it when it is the only one. */
function ContinueSection({
  reading,
  date,
  onOpen,
  clear,
}: {
  reading: ContinueReading;
  date: Intl.DateTimeFormat;
  onOpen: (entry: RecentFile) => void;
  clear: ReactNode;
}): ReactNode {
  const { entry, page, pageCount } = reading;
  const heading = useId();
  const parts = [
    message("recent.thisDevice"),
    message("recent.continuePage", { page: page + 1, count: pageCount }),
    ...(entry.hasHandle === true ? [] : [message("recent.chooseAgain")]),
  ];
  return (
    <section className="paper-recent" aria-labelledby={heading}>
      <div className="paper-recent-head">
        <h2 id={heading} className="paper-recent-title">
          {message("recent.continue")}
        </h2>
        {clear}
      </div>
      <Row entry={entry} meta={parts.join(" · ")} date={date} onOpen={onOpen} />
    </section>
  );
}

/** The document to continue, the recent documents, and Clear list. */
export function RecentList({
  entries,
  onOpen,
  onClear,
  stoppedAt,
  continueReading,
  locale,
}: RecentListProps): ReactNode {
  const heading = useId();
  const rest = entries.filter((entry) => entry.id !== continueReading?.entry.id);
  if (rest.length === 0 && continueReading === undefined) return null;
  const date = new Intl.DateTimeFormat(locale, { dateStyle: "medium" });
  const clear = (
    <Button variant="ghost" onPress={onClear}>
      {message("recent.clear")}
    </Button>
  );
  return (
    <>
      {continueReading === undefined ? null : (
        <ContinueSection
          reading={continueReading}
          date={date}
          onOpen={onOpen}
          clear={rest.length === 0 ? clear : null}
        />
      )}
      {rest.length === 0 ? null : (
        <section className="paper-recent" aria-labelledby={heading}>
          <div className="paper-recent-head">
            <h2 id={heading} className="paper-recent-title">
              {message("recent.title")}
            </h2>
            {clear}
          </div>
          <ul className="paper-recent-list">
            {rest.map((entry) => (
              <li key={entry.id}>
                <Row
                  entry={entry}
                  meta={recentMeta(entry, entry.hasHandle === true, stoppedAt?.get(entry.id))}
                  date={date}
                  onOpen={onOpen}
                />
              </li>
            ))}
          </ul>
        </section>
      )}
    </>
  );
}
