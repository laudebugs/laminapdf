// The text calls' gate for one open document. The document worker runs one call at a time, so a
// text or link call that waits for the extract module to download would hold the first page's
// render behind it: those calls wait here until the document's first page is drawn.

/** How long text calls wait for the open document's first render before going ahead anyway, in ms. */
const FIRST_DRAW_WAIT = 2000;

/** Open once document `id`'s first render answers, or after `wait` ms, whichever comes first. */
export class FirstDraw {
  readonly #id: number;
  readonly #opened: Promise<void>;
  #open: () => void = () => undefined;
  readonly #timer: ReturnType<typeof setTimeout>;

  constructor(id: number, wait = FIRST_DRAW_WAIT) {
    this.#id = id;
    this.#opened = new Promise<void>((resolve) => {
      this.#open = resolve;
    });
    // One timer per open: a caller that never renders waits once, not on every call.
    this.#timer = setTimeout(() => this.#open(), wait);
  }

  /** A render of document `id` answered: a render of an earlier document doesn't count. */
  drawn(id: number): void {
    if (id !== this.#id) return;
    clearTimeout(this.#timer);
    this.#open();
  }

  /** Resolves once the gate is open; rejects with `signal`'s reason if it was aborted meanwhile. */
  async wait(signal?: AbortSignal): Promise<void> {
    await this.#opened;
    signal?.throwIfAborted();
  }
}
