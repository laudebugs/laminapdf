// The find bar (R05, design language §7.30): in the secondary-toolbar slot, `role="search"`. A
// 300 px field searches as you type, with the count inside it ("3 of 9", "40+" while counting,
// "No matches"), then Previous and Next, Match case and Whole words, a note on the pages searched,
// and Close. Previous and Next run the registry's find commands, so Enter, Shift+Enter, F3,
// Shift+F3 and the palette do the same; Escape in the bar closes it. The count is a polite live
// region, announced at most once a second while results stream in. Focus starts in the field and
// goes back to where it was when the bar closes. Find comes in as a shape, like the viewer: the
// viewer's `viewer.find` fits.

import { type ReactNode, useEffect, useId, useRef, useState } from "react";
import { Input } from "react-aria-components";
import { useCommand } from "../commands/react.js";
import { IconButton } from "../components/button.js";
import { ToggleButton } from "../components/toggle.js";
import { message } from "../messages.js";
import { type ExternalStore, useStoreState } from "../state/hooks.js";

/** How find compares text. */
export interface FindOptionsSlice {
  readonly matchCase?: boolean;
  readonly wholeWords?: boolean;
}

/** The part of find's state the bar shows. */
export interface FindSlice {
  readonly query: string;
  readonly options: FindOptionsSlice;
  readonly total: number;
  readonly current: number | undefined;
  readonly searched: number;
  readonly pageCount: number;
  readonly done: boolean;
  readonly pagesWithoutText: number;
}

/** Find as the bar drives it: `viewer.find` fits. */
export interface FindControls extends ExternalStore<FindSlice> {
  find(query: string, options?: FindOptionsSlice): void;
}

/** How long typing pauses before a search starts, in ms. */
const DEBOUNCE = 150;
/** The shortest time between two announcements of the count, in ms. */
const ANNOUNCE_EVERY = 1000;

/** The count inside the field: "3 of 9", "40+" while counting, "No matches"; empty while idle. */
export function findCount(state: FindSlice): string {
  if (state.query.trim() === "") return "";
  if (state.total === 0) return state.done ? message("find.none") : message("find.counting");
  if (!state.done) return message("find.more", { total: state.total });
  return state.current === undefined
    ? message("find.total", { total: state.total })
    : message("find.of", { current: state.current + 1, total: state.total });
}

/** The note after the toggles: the pages searched so far, and those with no text. */
export function findNote(state: FindSlice): string {
  if (state.query.trim() === "") return "";
  const searched = state.done
    ? message("find.searchedAll", { pages: state.pageCount })
    : message("find.searching", { searched: state.searched, pages: state.pageCount });
  if (state.pagesWithoutText === 0) return searched;
  const noText =
    state.pagesWithoutText === 1
      ? message("find.noTextOne")
      : message("find.noText", { pages: state.pagesWithoutText });
  return `${searched} · ${noText}`;
}

/** The count, announced politely and at most once a second. */
function useAnnounced(text: string): string {
  const [said, setSaid] = useState(text);
  const last = useRef(0);
  useEffect(() => {
    const wait = Math.max(0, last.current + ANNOUNCE_EVERY - Date.now());
    const timer = setTimeout(() => {
      last.current = Date.now();
      setSaid(text);
    }, wait);
    return () => clearTimeout(timer);
  }, [text]);
  return said;
}

/** Props of `FindBar`. */
export interface FindBarProps {
  readonly find: FindControls;
  /** Close was pressed, or Escape in the bar: the app closes find and unmounts the bar. */
  readonly onClose: () => void;
}

/** The find bar. Mount it while find is open; closing unmounts it. */
export function FindBar({ find, onClose }: FindBarProps): ReactNode {
  const state = useStoreState(find, (s) => s);
  const [draft, setDraft] = useState(state.query);
  const field = useRef<HTMLInputElement>(null);
  const counted = useId();
  const next = useCommand("find.next");
  const previous = useCommand("find.previous");
  // Focus starts in the field, and returns where it was when the bar goes.
  useEffect(() => {
    const before = document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
    field.current?.focus();
    field.current?.select();
    return () => {
      if (before?.isConnected === true) before.focus();
    };
  }, []);
  // Searches as you type, once typing pauses.
  useEffect(() => {
    if (draft === find.getState().query) return;
    const timer = setTimeout(() => find.find(draft, find.getState().options), DEBOUNCE);
    return () => clearTimeout(timer);
  }, [draft, find]);
  const count = findCount(state);
  const announced = useAnnounced(count);
  const none = state.done && state.total === 0;
  // A toggle searches again at once, with the text typed so far.
  const toggle = (key: "matchCase" | "wholeWords") => {
    const options = { ...state.options, [key]: state.options[key] !== true };
    find.find(draft, options);
  };
  return (
    <div
      role="search"
      className="paper-find-bar"
      aria-label={message("find.label")}
      data-shortcut-scope="find-field"
      onKeyDown={(e) => {
        if (e.key !== "Escape") return;
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }}
    >
      <span className="paper-find-field">
        <Input
          ref={field}
          className="paper-find-input"
          aria-label={message("find.label")}
          aria-describedby={counted}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          spellCheck={false}
          autoComplete="off"
        />
        <span id={counted} className="paper-find-count" aria-hidden="true">
          {count}
        </span>
      </span>
      <IconButton
        icon="chevron-up"
        label={message("command.find.previous")}
        isDisabled={state.total === 0}
        onPress={() => previous.run()}
      />
      <IconButton
        icon="chevron-down"
        label={message("command.find.next")}
        isDisabled={state.total === 0}
        onPress={() => next.run()}
      />
      <span className="paper-find-divider" aria-hidden="true" />
      <ToggleButton isSelected={state.options.matchCase === true} onChange={() => toggle("matchCase")}>
        {message("find.matchCase")}
      </ToggleButton>
      <ToggleButton isSelected={state.options.wholeWords === true} onChange={() => toggle("wholeWords")}>
        {message("find.wholeWords")}
      </ToggleButton>
      <span className="paper-find-note" data-none={none || undefined}>
        {findNote(state)}
      </span>
      <IconButton icon="close" label={message("find.close")} onPress={onClose} />
      <span className="paper-visually-hidden" role="status" aria-live="polite">
        {announced}
      </span>
    </div>
  );
}
