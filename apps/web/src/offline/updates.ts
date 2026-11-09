// The service worker's lifecycle as the reader shows it (R15): registered once the page has
// loaded, in production builds only; a new version that has installed waits until the user
// chooses Reload, which tells it to take over and reloads the page once it has. Nothing here
// updates the reader on its own.

import type { ExternalStore } from "@laminapdf/ui";

/** What the reader shows about updates. */
export interface UpdateState {
  /** A new version has installed and waits for the user's Reload. */
  readonly ready: boolean;
}

/** The parts of `navigator.serviceWorker` used here; tests pass a fake. */
export interface WorkerContainer {
  register(url: string): Promise<WorkerRegistration>;
  readonly controller: unknown;
  addEventListener(type: "controllerchange", listener: () => void): void;
}

/** The parts of a `ServiceWorkerRegistration` used here. */
export interface WorkerRegistration {
  /** The version in charge before this page loaded, if any (a hard reload leaves the page uncontrolled). */
  readonly active?: unknown;
  readonly waiting: { postMessage(message: unknown): void } | null;
  readonly installing: {
    readonly state: string;
    addEventListener(type: "statechange", listener: () => void): void;
  } | null;
  addEventListener(type: "updatefound", listener: () => void): void;
}

/** The reader's update flow, as a store for React. */
export class AppUpdates implements ExternalStore<UpdateState> {
  #state: UpdateState = { ready: false };
  readonly #listeners = new Set<() => void>();
  #registration: WorkerRegistration | undefined;
  #reloading = false;
  /** Another tab's Reload put a newer version in charge: this page runs the old one until it reloads. */
  #superseded = false;
  readonly #container: WorkerContainer | undefined;
  readonly #reloadPage: () => void;

  /**
   * Registers `/sw.js` with `container` (absent where the browser has no service workers) and
   * watches for a waiting version; `reloadPage` reloads once the new version has taken over.
   */
  constructor(container: WorkerContainer | undefined, reloadPage: () => void) {
    this.#container = container;
    this.#reloadPage = reloadPage;
  }

  getState = (): UpdateState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Registers the worker; a failure leaves the reader working online, as it did before. */
  async start(): Promise<void> {
    const container = this.#container;
    if (container === undefined) return;
    let hadController = container.controller !== null;
    container.addEventListener("controllerchange", () => {
      // Only a takeover the user asked for reloads the page; the first install's claim doesn't.
      if (this.#reloading) this.#reloadPage();
      // A takeover asked for in another tab: this page keeps the old version until its Reload.
      else if (hadController) {
        this.#superseded = true;
        this.#set({ ready: true });
      }
      hadController = true;
    });
    const registration = await container.register("/sw.js");
    this.#registration = registration;
    // A version was already in charge (a hard reload left this page uncontrolled): a later claim
    // is another tab's update, not the first install.
    if (registration.active !== null && registration.active !== undefined) hadController = true;
    // A version that installed while no page was watching (a reload, a second tab) already waits.
    if (registration.waiting !== null && container.controller !== null) this.#set({ ready: true });
    registration.addEventListener("updatefound", () => {
      const installing = registration.installing;
      installing?.addEventListener("statechange", () => {
        // With no controller this is the first install, which needs no reload.
        if (installing.state === "installed" && container.controller !== null) this.#set({ ready: true });
      });
    });
  }

  /**
   * The user chose Reload: the waiting version takes over, then the page reloads into it; when
   * another tab already put it in charge, the page just reloads.
   */
  reload(): void {
    if (this.#superseded) {
      this.#reloadPage();
      return;
    }
    const waiting = this.#registration?.waiting;
    if (waiting === null || waiting === undefined) return;
    this.#reloading = true;
    waiting.postMessage("skip-waiting");
  }

  #set(state: UpdateState): void {
    this.#state = state;
    for (const listener of this.#listeners) listener();
  }
}
