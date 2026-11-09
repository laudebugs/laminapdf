// The home screen's Recent list (R01): each recent document as a button, its name in Newsreader
// over a line saying it is on this device and how many pages it has, the date it was last opened
// at the end. Linked documents are never listed (ADR 0011). A local document the reader can't read again without a picker says so;
// pressing it asks for the file. Clear list empties the list in this browser. The list is hidden
// when empty: a first visit needs no empty state.

import type { ReactNode } from "react";
import { Button as AriaButton } from "react-aria-components";
import { Button } from "../components/button.js";
import { countMessage, message } from "../messages.js";
import type { RecentFile } from "../platform/types.js";

/** Props of `RecentList`. */
export interface RecentListProps {
  /** Newest first. */
  entries: readonly RecentFile[];
  /** An entry was pressed. */
  onOpen: (entry: RecentFile) => void;
  /** Clear list was pressed. */
  onClear: () => void;
  /** The locale dates are written in. */
  locale?: string;
}

/** The line under the name: where, how many pages, and whether the file must be chosen again. */
export function recentMeta(entry: RecentFile, reopens: boolean): string {
  const pages =
    entry.pages === undefined ? [] : [countMessage(entry.pages, { one: "recent.page", many: "recent.pages" })];
  return [message("recent.thisDevice"), ...pages, ...(reopens ? [] : [message("recent.chooseAgain")])].join(" · ");
}

/** The recent documents, and Clear list. */
export function RecentList({ entries, onOpen, onClear, locale }: RecentListProps): ReactNode {
  if (entries.length === 0) return null;
  const date = new Intl.DateTimeFormat(locale, { dateStyle: "medium" });
  return (
    <section className="paper-recent" aria-labelledby="paper-recent-heading">
      <div className="paper-recent-head">
        <h2 id="paper-recent-heading" className="paper-recent-title">
          {message("recent.title")}
        </h2>
        <Button variant="ghost" onPress={onClear}>
          {message("recent.clear")}
        </Button>
      </div>
      <ul className="paper-recent-list">
        {entries.map((entry) => {
          const reopens = entry.hasHandle === true;
          return (
            <li key={entry.id}>
              <AriaButton className="paper-recent-row" onPress={() => onOpen(entry)}>
                <span className="paper-recent-name">{entry.name}</span>
                <span className="paper-recent-meta">{recentMeta(entry, reopens)}</span>
                <span className="paper-recent-date">{date.format(entry.lastOpened)}</span>
              </AriaButton>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
