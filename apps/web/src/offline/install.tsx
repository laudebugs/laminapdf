// Installing the reader (R15): the browser's own install prompt, deferred when it offers one
// (`beforeinstallprompt`, Chromium), so the reader can show it from More › Install Lamina PDF…
// rather than the browser's banner; gone once the reader is installed. Where the browser fires no
// such event (Firefox, Safari) there is no item: their own menus install sites.

import { type CommandId, type ExternalStore, InstallDialog, useStoreState } from "@laminapdf/ui";
import { type ReactNode, useState } from "react";

/** The event's own interface (not in TypeScript's DOM library). */
interface InstallPromptEvent extends Event {
  prompt(): Promise<void>;
  readonly userChoice: Promise<{ readonly outcome: "accepted" | "dismissed" }>;
}

/** The parts of a window this uses; tests pass a fake. */
export interface InstallScope {
  addEventListener(type: "beforeinstallprompt" | "appinstalled", listener: (event: Event) => void): void;
}

/** The deferred prompt, as a store: whether the browser offers an install now, and showing it. */
export class InstallPrompt implements ExternalStore<boolean> {
  #event: InstallPromptEvent | undefined;
  readonly #listeners = new Set<() => void>();

  constructor(scope: InstallScope) {
    scope.addEventListener("beforeinstallprompt", (event) => {
      // Kept for More › Install Lamina PDF…, instead of the browser's own banner.
      event.preventDefault();
      this.#set(event as InstallPromptEvent);
    });
    scope.addEventListener("appinstalled", () => this.#set(undefined));
  }

  getState = (): boolean => this.#event !== undefined;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Shows the browser's prompt; an event can prompt once, so it is used up whatever the answer. */
  async prompt(): Promise<void> {
    const event = this.#event;
    if (event === undefined) return;
    await event.prompt();
    const { outcome } = await event.userChoice;
    // Dismissed: the browser fires the event again when it would offer the install again.
    if (outcome === "accepted" || this.#event === event) this.#set(undefined);
  }

  #set(event: InstallPromptEvent | undefined): void {
    this.#event = event;
    for (const listener of this.#listeners) listener();
  }
}

let pagePrompt: InstallPrompt | undefined;

/** The page's one install prompt; main.tsx makes it at startup, so no early offer is missed. */
export function installPrompt(): InstallPrompt {
  pagePrompt ??= new InstallPrompt(window);
  return pagePrompt;
}

/** Whether the More menu offers Install, its dialog, and the `app.install` handler. */
export function useInstall(prompt: InstallPrompt = installPrompt()): {
  available: boolean;
  dialog: ReactNode;
  actions: Readonly<Partial<Record<CommandId, () => void>>>;
} {
  const available = useStoreState(prompt, (s) => s);
  const [open, setOpen] = useState(false);
  return {
    available,
    dialog: (
      <InstallDialog
        isOpen={open && available}
        onInstall={() => {
          setOpen(false);
          void prompt.prompt().catch(reportError);
        }}
        onClose={() => setOpen(false)}
      />
    ),
    // Without an offer the command has no handler, so it stays out of the palette.
    actions: available ? { "app.install": () => setOpen(true) } : {},
  };
}
