// The Threads rail tab (O13), loaded once a document with threads is open (the ./dialogs entry, #380): the file's article threads, one expandable entry each with its parts
// as links, and Previous part / Next part. A thread joins the parts of an article across pages
// (ISO 32000-2 §12.4.3); the panel follows them and never edits them. A part whose page the file
// lacks is listed muted and skipped. The current part carries `aria-current="location"`.

import { type ReactNode, useState } from "react";
import { Button } from "../components/button.js";
import { laterMessage } from "../later-messages.js";
import type { ThreadData, ThreadPartRef } from "./threads.js";

/** Props of `ThreadsPanel`. */
export interface ThreadsPanelProps {
  readonly threads: readonly ThreadData[];
  /** The part shown last, when the reader followed one. */
  readonly current?: ThreadPartRef | undefined;
  /** A page's name for people: its label, else its number. */
  readonly pageName: (index: number) => string;
  /** Go to a part. */
  readonly onGo: (part: ThreadPartRef) => void;
  /** The part after or before the current one, skipping missing pages; `undefined` at an end. */
  readonly step: (direction: 1 | -1) => ThreadPartRef | undefined;
}

/** The thread's title, or "Untitled thread". */
export function threadTitle(thread: ThreadData): string {
  const title = thread.title?.trim();
  return title === undefined || title === "" ? laterMessage("threads.untitled") : title;
}

/** "3 parts · pages 3, 5, 6": the thread's summary line. */
function summary(thread: ThreadData, pageName: (index: number) => string): string {
  const pages = [...new Set(thread.parts.flatMap((p) => (p.page === undefined ? [] : [p.page])))];
  const parts =
    thread.parts.length === 1
      ? laterMessage("threads.parts.one")
      : laterMessage("threads.parts", { count: thread.parts.length });
  if (pages.length === 0) return parts;
  return laterMessage(pages.length === 1 ? "threads.summary.one" : "threads.summary", {
    parts,
    pages: pages.map(pageName).join(", "),
  });
}

/** One thread: its title and summary as a disclosure, then its parts. */
function Thread({
  thread,
  index,
  props,
  open,
  onToggle,
}: {
  thread: ThreadData;
  index: number;
  props: ThreadsPanelProps;
  open: boolean;
  onToggle: () => void;
}): ReactNode {
  const id = `paper-thread-${index}`;
  return (
    <li className="paper-thread">
      <button type="button" className="paper-thread-toggle" aria-expanded={open} aria-controls={id} onClick={onToggle}>
        <span className="paper-thread-title">{threadTitle(thread)}</span>
        <span className="paper-thread-summary">{summary(thread, props.pageName)}</span>
      </button>
      <ol id={id} className="paper-thread-parts" hidden={!open}>
        {thread.parts.map((part, p) => {
          const label =
            part.page === undefined
              ? laterMessage("threads.part.missing", { part: p + 1 })
              : laterMessage("threads.part", { part: p + 1, page: props.pageName(part.page) });
          const current = props.current?.thread === index && props.current.part === p;
          return (
            // Parts never move, so their index is their identity.
            // biome-ignore lint/suspicious/noArrayIndexKey: a part's place in its thread is what names it
            <li key={p}>
              {part.page === undefined ? (
                <span className="paper-thread-part" data-missing="">
                  {label}
                </span>
              ) : (
                <button
                  type="button"
                  className="paper-thread-part"
                  aria-current={current ? "location" : undefined}
                  onClick={() => props.onGo({ thread: index, part: p })}
                >
                  {label}
                </button>
              )}
            </li>
          );
        })}
      </ol>
    </li>
  );
}

/** The file's article threads and the way through them. */
export function ThreadsPanel(props: ThreadsPanelProps): ReactNode {
  const [open, setOpen] = useState<ReadonlySet<number>>(() => new Set([props.current?.thread ?? 0]));
  const previous = props.step(-1);
  const next = props.step(1);
  const go = (part: ThreadPartRef | undefined) => {
    if (part === undefined) return;
    setOpen((was) => new Set(was).add(part.thread));
    props.onGo(part);
  };
  return (
    <section className="paper-threads" aria-label={laterMessage("threads.title")}>
      <p className="paper-threads-note">{laterMessage("threads.note")}</p>
      <ul className="paper-threads-list">
        {props.threads.map((thread, index) => (
          <Thread
            // biome-ignore lint/suspicious/noArrayIndexKey: the file's thread order names each thread
            key={index}
            thread={thread}
            index={index}
            props={props}
            open={open.has(index)}
            onToggle={() =>
              setOpen((was) => {
                const next = new Set(was);
                if (!next.delete(index)) next.add(index);
                return next;
              })
            }
          />
        ))}
      </ul>
      <div className="paper-threads-footer">
        <Button size={26} isDisabled={previous === undefined} onPress={() => go(previous)}>
          {laterMessage("threads.previous")}
        </Button>
        <Button size={26} isDisabled={next === undefined} onPress={() => go(next)}>
          {laterMessage("threads.next")}
        </Button>
      </div>
    </section>
  );
}
