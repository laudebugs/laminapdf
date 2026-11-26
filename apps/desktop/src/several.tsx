// Several PDFs the OS opened together (Paper O07; E2u): the reader shows one document per tab, so
// it asks whether to merge them into one new file, which then opens, or to open each in its own tab.
// The main process holds the files under a request and tells the window their names only
// (src-tauri/src/several.rs); the answer names the request.

import { Button, Dialog, type ExternalStore, Notice, type ToastQueue, useStoreState } from "@laminapdf/ui";
import { laterMessage as word } from "@laminapdf/ui/editing";
import type { ReactNode } from "react";
import type { SeveralAsked } from "./engine/open.ts";

/** What the question shows. */
export interface SeveralState extends SeveralAsked {
  /** The merge is being made: nothing more can be chosen. */
  readonly busy: boolean;
  /** The last merge failed: the files wait for another choice. */
  readonly failed: boolean;
}

/** What the person chose. */
type Choice = "merge" | "each" | "cancel";

/** The question about files opened together, and its answer. */
export class SeveralAsk implements ExternalStore<SeveralState | undefined> {
  readonly #answer: (request: number, answer: Choice, suggested?: string) => Promise<boolean>;
  readonly #toasts: ToastQueue;
  #state: SeveralState | undefined;
  /** Questions that came while another was open, in order. */
  readonly #later: SeveralAsked[] = [];
  readonly #listeners = new Set<() => void>();

  /**
   * @param answer - Sends the answer to the main process; resolves to whether the request is done.
   * @param toasts - Where a failed answer is said.
   */
  constructor(answer: (request: number, choice: Choice, suggested?: string) => Promise<boolean>, toasts: ToastQueue) {
    this.#answer = answer;
    this.#toasts = toasts;
  }

  getState(): SeveralState | undefined {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Asks what to do with the files of `asked`, once the question before it is answered. */
  ask(asked: SeveralAsked): void {
    if (this.#state === undefined) this.#set({ ...asked, busy: false, failed: false });
    else this.#later.push(asked);
  }

  /**
   * Does what the person chose. A merge that fails leaves the question open, so they can open each
   * file instead; so does an answer that couldn't be sent.
   */
  async choose(choice: Choice): Promise<void> {
    const asked = this.#state;
    if (asked === undefined || asked.busy) return;
    this.#set({ ...asked, busy: true, failed: false });
    try {
      const first = asked.names[0]?.replace(/\.pdf$/i, "") ?? "document";
      const suggested = choice === "merge" ? word("documents.several.name", { name: first }) : undefined;
      const done = await this.#answer(asked.request, choice, suggested);
      // Cancelling the save dialog of a merge leaves the question as it was.
      if (done) this.#next();
      else this.#set({ ...asked, busy: false, failed: false });
    } catch {
      this.#set({ ...asked, busy: false, failed: true });
      if (choice !== "merge") this.#toasts.add({ message: word("documents.several.failed.open") });
    }
  }

  /** The next question, if one waits. */
  #next(): void {
    const next = this.#later.shift();
    this.#set(next === undefined ? undefined : { ...next, busy: false, failed: false });
  }

  #set(state: SeveralState | undefined): void {
    this.#state = state;
    for (const listener of [...this.#listeners]) listener();
  }
}

/** The question: Merge into one document, or Open each. */
export function SeveralFiles({ ask }: { readonly ask: SeveralAsk }): ReactNode {
  const state = useStoreState(ask, (s) => s);
  return (
    <Dialog
      isOpen={state !== undefined}
      onOpenChange={(isOpen) => {
        if (!isOpen) void ask.choose("cancel");
      }}
      title={word("documents.several.title", { count: state?.names.length ?? 0 })}
      description={word("documents.several.body.tabs")}
      closeLabel={word("organise.close")}
      actions={
        <>
          <Button isDisabled={state?.busy === true} onPress={() => void ask.choose("each")}>
            {word("documents.several.each")}
          </Button>
          <Button
            variant="primary"
            isDisabled={state?.busy === true}
            autoFocus
            onPress={() => void ask.choose("merge")}
          >
            {word("documents.several.merge")}
          </Button>
        </>
      }
    >
      <ol className="paper-documents-parts">
        {(state?.names ?? []).map((name, i) => (
          // The same name can come twice from different folders: its place in the order is its key.
          // biome-ignore lint/suspicious/noArrayIndexKey: the list never reorders
          <li key={i}>
            <span className="paper-documents-file-name">{name}</span>
          </li>
        ))}
      </ol>
      {state?.failed === true ? (
        <Notice tone="danger" title={word("documents.failed.merge")}>
          {word("documents.several.failed")}
        </Notice>
      ) : null}
    </Dialog>
  );
}
