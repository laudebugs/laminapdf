import { describe, expect, it, vi } from "vitest";
import { type OnlineScope, onlineStore } from "./online.ts";

function fakeWindow(onLine: boolean) {
  const listeners = new Map<string, () => void>();
  let tick: (() => void) | undefined;
  const scope: OnlineScope & { navigator: { onLine: boolean } } = {
    navigator: { onLine },
    addEventListener: (type, l) => listeners.set(type, l),
    removeEventListener: (type) => listeners.delete(type),
    setInterval: (handler) => {
      tick = handler;
      return 1;
    },
    clearInterval: () => {
      tick = undefined;
    },
  };
  return { scope, fire: (type: string) => listeners.get(type)?.(), tick: () => tick?.() };
}

describe("onlineStore", () => {
  it("follows the online and offline events", () => {
    const { scope, fire } = fakeWindow(true);
    const store = onlineStore(scope);
    const listener = vi.fn();
    store.subscribe(listener);
    scope.navigator.onLine = false;
    fire("offline");
    expect(store.getState()).toBe(false);
    expect(listener).toHaveBeenCalledOnce();
  });

  it("notices coming back online without the event, but only while offline", () => {
    const { scope, tick } = fakeWindow(false);
    const store = onlineStore(scope);
    const listener = vi.fn();
    const stop = store.subscribe(listener);
    tick();
    expect(listener).not.toHaveBeenCalled();
    scope.navigator.onLine = true;
    tick();
    expect(listener).toHaveBeenCalledOnce();
    tick();
    expect(listener).toHaveBeenCalledOnce();
    stop();
  });
});
