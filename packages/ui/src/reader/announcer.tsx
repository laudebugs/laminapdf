// Spoken status for what changes without focus moving (WCAG 4.1.3): the page in view, the zoom,
// a document opening. One polite and one assertive live region, both always in the document, so a
// screen reader has heard them exist before anything is put in them. Polite messages wait until
// their kind has been quiet for a moment, so scrolling through twenty pages says the last one.

import { type ReactNode, useSyncExternalStore } from "react";

/** How long a polite message waits for a newer one of its kind, in ms. */
export const ANNOUNCE_DELAY = 700;

/** What the two live regions say now. */
export interface Announcements {
  readonly polite: string;
  readonly assertive: string;
}

/** The app's announcer: what to say politely (after a pause) or at once. */
export class Announcer {
  #state: Announcements = { polite: "", assertive: "" };
  readonly #listeners = new Set<() => void>();
  readonly #timers = new Map<string, ReturnType<typeof setTimeout>>();
  readonly #delay: number;
  /** The kind and time of the last polite message said, to join one of another kind due with it. */
  #last: { kind: string; at: number } | undefined;

  constructor(delay = ANNOUNCE_DELAY) {
    this.#delay = delay;
  }

  /**
   * Says `text` politely once nothing newer of the same `kind` ("page", "zoom") has come for a
   * moment; a newer one replaces it. Said within a moment of another kind's, the two are joined.
   */
  polite(text: string, kind = "status"): void {
    const pending = this.#timers.get(kind);
    if (pending !== undefined) clearTimeout(pending);
    this.#timers.set(
      kind,
      setTimeout(() => {
        this.#timers.delete(kind);
        // Another kind said a moment ago may not have been read yet: say both.
        const last = this.#last;
        const now = Date.now();
        const joined =
          last !== undefined && last.kind !== kind && now - last.at < this.#delay
            ? `${this.#state.polite.replace(/\u200b$/, "")}. ${text}`
            : text;
        this.#last = { kind, at: now };
        this.#say("polite", joined);
      }, this.#delay),
    );
  }

  /** Says `text` at once, interrupting: errors only. */
  assertive(text: string): void {
    this.#say("assertive", text);
  }

  /** Drops what is waiting (a document closed). */
  cancel(): void {
    for (const timer of this.#timers.values()) clearTimeout(timer);
    this.#timers.clear();
  }

  /** What the two regions say now. */
  getState(): Announcements {
    return this.#state;
  }

  /** Calls `listener` after each change; returns a function that stops it. */
  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  #say(region: keyof Announcements, text: string): void {
    // The same words again would not be read: a zero-width space at the end makes them new.
    const now = this.#state[region];
    const next = now === text ? `${text}\u200b` : text;
    this.#state = { ...this.#state, [region]: next };
    for (const listener of this.#listeners) listener();
  }
}

/** The two live regions. Render them once, for the app's life. */
export function LiveRegions({ announcer }: { announcer: Announcer }): ReactNode {
  const state = useSyncExternalStore(
    (listener) => announcer.subscribe(listener),
    () => announcer.getState(),
  );
  return (
    <>
      <div className="paper-visually-hidden" aria-live="polite" aria-atomic="true">
        {state.polite}
      </div>
      <div className="paper-visually-hidden" aria-live="assertive" aria-atomic="true">
        {state.assertive}
      </div>
    </>
  );
}
