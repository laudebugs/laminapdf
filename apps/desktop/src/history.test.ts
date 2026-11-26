// Desktop reading history: command handlers, Back/Forward labels, and mouse Back/Forward guards.

import { afterEach, describe, expect, it, vi } from "vitest";
import { handleHistoryMouseUp, historyActions, historyTargetLabel } from "./history.ts";

class FakeElement {
  readonly #matches: ReadonlySet<string>;
  constructor(matches: readonly string[] = []) {
    this.#matches = new Set(matches);
  }
  closest(selector: string): FakeElement | null {
    for (const part of selector.split(",").map((s) => s.trim())) {
      if (this.#matches.has(part)) return this;
    }
    return null;
  }
}

class FakeInput extends FakeElement {
  readonly type: string;
  constructor(type = "text") {
    super();
    this.type = type;
  }
}

class FakeTextArea extends FakeElement {}
class FakeSelect extends FakeElement {}

function stubDom(modals: readonly FakeElement[] = []): void {
  vi.stubGlobal("Element", FakeElement);
  vi.stubGlobal("HTMLInputElement", FakeInput);
  vi.stubGlobal("HTMLTextAreaElement", FakeTextArea);
  vi.stubGlobal("HTMLSelectElement", FakeSelect);
  vi.stubGlobal("document", { querySelectorAll: () => modals });
}

function fakeHistory(moved = true) {
  return {
    back: vi.fn(() => moved),
    forward: vi.fn(() => moved),
  };
}

function mouseUp(button: number, target: unknown = null, defaultPrevented = false) {
  const preventDefault = vi.fn();
  const event = { button, target, defaultPrevented, preventDefault } as unknown as MouseEvent;
  return { event, preventDefault };
}

describe("desktop reading history", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("registers nav.back and nav.forward handlers even before a document is open", () => {
    const empty = historyActions(undefined);
    expect(Object.keys(empty)).toEqual(["nav.back", "nav.forward"]);
    expect(() => {
      empty["nav.back"]?.(undefined);
      empty["nav.forward"]?.(undefined);
    }).not.toThrow();

    const history = fakeHistory(true);
    const wired = historyActions(history);
    wired["nav.back"]?.(undefined);
    wired["nav.forward"]?.(undefined);
    expect(history.back).toHaveBeenCalledTimes(1);
    expect(history.forward).toHaveBeenCalledTimes(1);
  });

  it("formats target history entries as 1-based page numbers", () => {
    expect(historyTargetLabel(undefined)).toBeUndefined();
    expect(historyTargetLabel({ id: 1, point: { page: 0, x: 0, y: 0 }, zoom: "fit-width" })).toBe("1");
    expect(historyTargetLabel({ id: 2, point: { page: 4, x: 0.2, y: 0.5 }, zoom: 1.5 })).toBe("5");
  });

  it("steps back and forward on mouse buttons 3 and 4 and always prevents default", () => {
    stubDom();
    const history = fakeHistory(false);

    const back = mouseUp(3, new FakeElement());
    handleHistoryMouseUp(back.event, history);
    expect(history.back).toHaveBeenCalledTimes(1);
    expect(back.preventDefault).toHaveBeenCalledTimes(1);

    const forward = mouseUp(4, new FakeElement());
    handleHistoryMouseUp(forward.event, history);
    expect(history.forward).toHaveBeenCalledTimes(1);
    expect(forward.preventDefault).toHaveBeenCalledTimes(1);

    // Other mouse buttons and already-handled events are left alone.
    const left = mouseUp(0, new FakeElement());
    handleHistoryMouseUp(left.event, history);
    const handled = mouseUp(3, new FakeElement(), true);
    handleHistoryMouseUp(handled.event, history);
    expect(history.back).toHaveBeenCalledTimes(1);
    expect(history.forward).toHaveBeenCalledTimes(1);
    expect(left.preventDefault).not.toHaveBeenCalled();
    expect(handled.preventDefault).not.toHaveBeenCalled();
  });

  it("ignores mouse Back/Forward inside editable fields, dialogs, alertdialogs, or while a modal is open", () => {
    stubDom();
    const history = fakeHistory(true);

    handleHistoryMouseUp(mouseUp(3, new FakeInput("text")).event, history);
    handleHistoryMouseUp(mouseUp(4, new FakeElement(['[role="alertdialog"]'])).event, history);
    handleHistoryMouseUp(mouseUp(3, new FakeElement(['[role="dialog"]'])).event, history);
    expect(history.back).not.toHaveBeenCalled();
    expect(history.forward).not.toHaveBeenCalled();

    // Even a click outside the dialog is blocked while an active aria-modal exists.
    stubDom([new FakeElement()]);
    const onScrim = mouseUp(3, new FakeElement());
    handleHistoryMouseUp(onScrim.event, history);
    expect(history.back).not.toHaveBeenCalled();
    expect(onScrim.preventDefault).not.toHaveBeenCalled();

    // An exiting modal no longer blocks navigation.
    stubDom([new FakeElement(["[data-exiting]"])]);
    handleHistoryMouseUp(mouseUp(3, new FakeElement()).event, history);
    expect(history.back).toHaveBeenCalledTimes(1);
  });
});
