// The home screen's Recent list (R01; C7b): each recent document as a button, its name in
// Newsreader over a line saying where it is and how many pages it has, the date it was last
// opened at the end. A local document the reader can't read again without a picker says so;
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
  /** Whether a local entry without a kept handle can still reopen (from a cached copy). */
  canReopen?: (entry: RecentFile) => boolean;
  /** An entry was pressed. */
  onOpen: (entry: RecentFile) => void;
  /** Clear list was pressed. */
  onClear: () => void;
  /** The locale dates are written in. */
  locale?: string;
}

/** Where `entry` is: the linked document's host, or this device. */
function place(entry: RecentFile): string {
  if (entry.url === undefined) return message("recent.thisDevice");
  try {
    return new URL(entry.url).host;
  } catch {
    return entry.url;
  }
}

/** The line under the name: where, how many pages, and whether the file must be chosen again. */
export function recentMeta(entry: RecentFile, reopens: boolean): string {
  const pages =
    entry.pages === undefined ? [] : [countMessage(entry.pages, { one: "recent.page", many: "recent.pages" })];
  return [place(entry), ...pages, ...(reopens ? [] : [message("recent.chooseAgain")])].join(" · ");
}

/** The recent documents, and Clear list. */
export function RecentList({ entries, canReopen, onOpen, onClear, locale }: RecentListProps): ReactNode {
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
          const reopens = entry.url !== undefined || entry.hasHandle === true || canReopen?.(entry) === true;
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
