// Several PDFs the OS opened together (Paper O07; E2u): the reader shows one document per tab, so
// it asks whether to merge them into one new file, which then opens, or to open each in its own tab.
// The main process holds the files under a request and tells the window their names only
// (src-tauri/src/several.rs); the answer names the request.

import { Button, Dialog, type ExternalStore, Notice, type ToastQueue, useStoreState } from "@laminapdf/ui";
import { laterCountMessage, laterMessage as word } from "@laminapdf/ui/editing";
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

  /** Asks what to do with the files of `asked`. */
  ask(asked: SeveralAsked): void {
    this.#set({ ...asked, busy: false, failed: false });
  }

  /** Does what the person chose. A merge that fails leaves the question open, so they can open each instead. */
  async choose(choice: Choice): Promise<void> {
    const asked = this.#state;
    if (asked === undefined || asked.busy) return;
    if (choice !== "merge") {
      this.#set(undefined);
      await this.#answer(asked.request, choice).catch(() =>
        this.#toasts.add({ message: word("documents.failed.merge") }),
      );
      return;
    }
    this.#set({ ...asked, busy: true, failed: false });
    try {
      const first = asked.names[0]?.replace(/\.pdf$/i, "") ?? "document";
      const done = await this.#answer(asked.request, "merge", word("documents.several.name", { name: first }));
      // Cancelling the save dialog leaves the question as it was.
      this.#set(done ? undefined : { ...asked, busy: false, failed: false });
    } catch {
      this.#set({ ...asked, busy: false, failed: true });
    }
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
      title={laterCountMessage(state?.names.length ?? 0, {
        one: "documents.several.title",
        many: "documents.several.title",
      })}
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
