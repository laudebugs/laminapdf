// Find's Results tab (R05, design language §7.30): every match in page order, grouped under a
// heading per page ("Page 3 · Repairs backlog": the page's label and its outline section), each a
// link with the text around the match and the match itself in weight 600. The current match has
// the accent tint and an inset accent ring and is `aria-current="true"`, so it isn't marked by
// colour alone, and it scrolls into the list's view as Next and Previous move it. While pages are
// searched the list grows; with no match, it says so and offers to turn off the option that may
// be hiding one. Find comes in as a shape, like the find bar: the viewer's `viewer.find` fits.

import { type MouseEvent, type ReactNode, useEffect, useMemo, useRef } from "react";
import { Button } from "../components/button.js";
import { EmptyState } from "../components/empty-state.js";
import { message } from "../messages.js";
import { useStoreState } from "../state/hooks.js";
import type { FindControls, FindOptionsSlice } from "./find-bar.js";

/** One match, as find lists it. */
export interface FindResultItem {
  /** Its position among every match, 0-based, in document order. */
  readonly ordinal: number;
  /** Its page, 0-based. */
  readonly page: number;
  /** The text before the match, the match, and the text after, as the page has them. */
  readonly snippet: { readonly before: string; readonly match: string; readonly after: string };
}

/** Find as the Results tab drives it: `viewer.find` fits. */
export interface FindResultsControls extends FindControls {
  /** Every match found so far in document order; the same array until the state changes. */
  results(): readonly FindResultItem[];
  /** Makes match `ordinal` current and scrolls it into view. */
  goTo(ordinal: number): void;
}

/** Props of `FindResults`. */
export interface FindResultsProps {
  readonly find: FindResultsControls;
  /** The heading of page `page`'s group (0-based), such as "Page iv · Methods". */
  readonly heading: (page: number) => string;
  /** A result was chosen, after find went to it (to close a rail that covers the page, say). */
  readonly onNavigate?: () => void;
}

/** One page's matches. */
interface Group {
  readonly page: number;
  readonly items: readonly FindResultItem[];
}

/** `results`, already in document order, split by page. */
function byPage(results: readonly FindResultItem[]): Group[] {
  const groups: { page: number; items: FindResultItem[] }[] = [];
  for (const item of results) {
    const last = groups.at(-1);
    if (last?.page === item.page) last.items.push(item);
    else groups.push({ page: item.page, items: [item] });
  }
  return groups;
}

/** The option to turn off when nothing matched: Whole words first, as it hides more. */
function hidingOption(options: FindOptionsSlice): "wholeWords" | "matchCase" | undefined {
  if (options.wholeWords === true) return "wholeWords";
  if (options.matchCase === true) return "matchCase";
  return undefined;
}

/** What the tab shows when no match was found. */
function NoMatches({ find }: { find: FindResultsControls }): ReactNode {
  const { query, options } = find.getState();
  const option = hidingOption(options);
  const action =
    option === undefined ? undefined : (
      <Button variant="ghost" onPress={() => find.find(query, { ...options, [option]: false })}>
        {message(option === "wholeWords" ? "find.results.turnOffWholeWords" : "find.results.turnOffMatchCase")}
      </Button>
    );
  return (
    <EmptyState
      title={message("find.results.none", { query })}
      headingLevel={3}
      icon="find"
      {...(action === undefined ? {} : { actions: action })}
    />
  );
}

/** Every match of find's query, by page, each a link that goes to it. */
export function FindResults({ find, heading, onNavigate }: FindResultsProps): ReactNode {
  const state = useStoreState(find, (s) => s);
  const results = find.results();
  const groups = useMemo(() => byPage(results), [results]);
  const list = useRef<HTMLDivElement>(null);
  // The current match stays in the list's view as Next and Previous move it.
  const current = state.current;
  useEffect(() => {
    if (current === undefined) return;
    list.current?.querySelector(`[data-ordinal="${current}"]`)?.scrollIntoView?.({ block: "nearest" });
  }, [current]);
  if (state.query.trim() === "") return null;
  if (state.done && state.total === 0) return <NoMatches find={find} />;
  const go = (e: MouseEvent, ordinal: number) => {
    e.preventDefault();
    find.goTo(ordinal);
    onNavigate?.();
  };
  return (
    <div ref={list} className="paper-find-results">
      {groups.map(({ page, items }) => (
        <section key={page} className="paper-find-results-group" aria-label={heading(page)}>
          <h3 className="paper-find-results-heading">{heading(page)}</h3>
          <ul className="paper-find-results-list">
            {items.map(({ ordinal, snippet }) => (
              <li key={ordinal}>
                <a
                  className="paper-find-result"
                  href={`#page=${page + 1}`}
                  data-ordinal={ordinal}
                  aria-current={ordinal === current ? "true" : undefined}
                  onClick={(e) => go(e, ordinal)}
                >
                  {snippet.before}
                  <strong className="paper-find-result-match">{snippet.match}</strong>
                  {snippet.after}
                </a>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
