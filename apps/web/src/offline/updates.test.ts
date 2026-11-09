import { describe, expect, it, vi } from "vitest";
import { AppUpdates, type WorkerContainer, type WorkerRegistration } from "./updates.ts";

/** A container and registration that the test drives. */
function fakes(options: { controlled: boolean; waiting: boolean }) {
  const listeners: Record<string, () => void> = {};
  const posted: unknown[] = [];
  const installingListeners: (() => void)[] = [];
  const installing = {
    state: "installing",
    addEventListener: (_: "statechange", l: () => void) => installingListeners.push(l),
  };
  const registration: WorkerRegistration & { waiting: { postMessage(m: unknown): void } | null } = {
    waiting: options.waiting ? { postMessage: (m: unknown) => posted.push(m) } : null,
    installing,
    addEventListener: (type, l) => {
      listeners[type] = l;
    },
  };
  const container: WorkerContainer = {
    register: async () => registration,
    controller: options.controlled ? {} : null,
    addEventListener: (type, l) => {
      listeners[type] = l;
    },
  };
  const install = () => {
    registration.waiting = { postMessage: (m: unknown) => posted.push(m) };
    listeners.updatefound?.();
    installing.state = "installed";
    for (const l of installingListeners) l();
  };
  return { container, listeners, posted, install };
}

describe("AppUpdates", () => {
  it("does nothing where the browser has no service workers", async () => {
    const updates = new AppUpdates(undefined, () => undefined);
    await updates.start();
    expect(updates.getState().ready).toBe(false);
  });

  it("offers a new version once it has installed, and never on the first install", async () => {
    const first = fakes({ controlled: false, waiting: false });
    const fresh = new AppUpdates(first.container, () => undefined);
    await fresh.start();
    first.install();
    expect(fresh.getState().ready).toBe(false);

    const later = fakes({ controlled: true, waiting: false });
    const updates = new AppUpdates(later.container, () => undefined);
    await updates.start();
    later.install();
    expect(updates.getState().ready).toBe(true);
  });

  it("offers a version that was already waiting when the page loaded", async () => {
    const { container } = fakes({ controlled: true, waiting: true });
    const updates = new AppUpdates(container, () => undefined);
    await updates.start();
    expect(updates.getState().ready).toBe(true);
  });

  it("reloads only after the user's Reload, once the new version has taken over", async () => {
    const { container, listeners, posted } = fakes({ controlled: true, waiting: true });
    const reload = vi.fn();
    const updates = new AppUpdates(container, reload);
    await updates.start();
    listeners.controllerchange?.();
    expect(reload).not.toHaveBeenCalled();
    updates.reload();
    expect(posted).toEqual(["skip-waiting"]);
    listeners.controllerchange?.();
    expect(reload).toHaveBeenCalledOnce();
  });
});
