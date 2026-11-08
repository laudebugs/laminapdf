import { afterEach, describe, expect, it, vi } from "vitest";
import { shellActions } from "../src/shell/actions.ts";
import {
  clampRailWidth,
  RAIL_DEFAULT_WIDTH,
  RAIL_MAX_WIDTH,
  RAIL_MIN_WIDTH,
  ShellLayout,
} from "../src/shell/layout.ts";
import { ModeRegistry } from "../src/shell/modes.ts";
import { fakePlatform } from "./fakes.ts";

afterEach(() => {
  vi.useRealTimers();
});

describe("ShellLayout", () => {
  it("starts with only the canvas: the rail closed, no focus mode, no inspector", () => {
    const state = new ShellLayout().getState();
    expect(state).toEqual({
      rail: { open: false, width: RAIL_DEFAULT_WIDTH, tab: undefined },
      focus: false,
      inspector: undefined,
    });
  });

  it("keeps the rail's state once it rests, and reads it back on the next visit", async () => {
    vi.useFakeTimers();
    const { storage } = fakePlatform();
    const set = vi.spyOn(storage, "set");
    const first = new ShellLayout(storage);
    first.toggleRail();
    for (const width of [240, 260, 280, 300]) first.setRailWidth(width);
    first.setRailTab("outline");
    await vi.runAllTimersAsync();
    expect(set).toHaveBeenCalledTimes(1);
    const next = new ShellLayout(storage);
    await next.load();
    expect(next.getState().rail).toEqual({ open: true, width: 300, tab: "outline" });
  });

  it("ignores a kept state it didn't write", async () => {
    const { storage } = fakePlatform();
    await storage.set("shell.rail", { open: "yes" });
    const layout = new ShellLayout(storage);
    await layout.load();
    expect(layout.getState().rail.open).toBe(false);
  });

  it("keeps a change the user made while the kept state was still loading", async () => {
    const { storage } = fakePlatform();
    await storage.set("shell.rail", { open: false, width: 300 });
    const layout = new ShellLayout(storage);
    const loading = layout.load();
    layout.toggleRail();
    await loading;
    expect(layout.getState().rail.open).toBe(true);
  });

  it("never keeps focus mode", async () => {
    const { storage } = fakePlatform();
    const first = new ShellLayout(storage);
    first.toggleFocus();
    const next = new ShellLayout(storage);
    await next.load();
    expect([first.getState().focus, next.getState().focus]).toEqual([true, false]);
  });

  it("keeps the rail's width within its limits", () => {
    expect([clampRailWidth(10), clampRailWidth(10_000), clampRailWidth(Number.NaN), clampRailWidth(250.4)]).toEqual([
      RAIL_MIN_WIDTH,
      RAIL_MAX_WIDTH,
      RAIL_DEFAULT_WIDTH,
      250,
    ]);
  });

  it("closes a selection's inspector on deselect and leaves a panel opened on purpose", () => {
    const layout = new ShellLayout();
    layout.openInspector({ id: "highlight", selection: true });
    layout.deselect();
    const afterSelection = layout.getState().inspector;
    layout.openInspector({ id: "repairs", selection: false });
    layout.deselect();
    expect([afterSelection, layout.getState().inspector]).toEqual([undefined, { id: "repairs", selection: false }]);
  });

  it("clears a selection before it leaves focus mode on Escape, whichever came first", () => {
    const layout = new ShellLayout();
    layout.openInspector({ id: "highlight", selection: true });
    layout.toggleFocus();
    layout.escapeFocus();
    const first = [layout.getState().inspector, layout.getState().focus];
    layout.escapeFocus();
    expect([first, layout.getState().focus]).toEqual([[undefined, true], false]);
  });

  it("tells the app when it closes a selection's inspector, so the app clears its selection", () => {
    const layout = new ShellLayout();
    const cleared = vi.fn();
    layout.onDeselect(cleared);
    layout.deselect();
    layout.openInspector({ id: "highlight", selection: true });
    layout.deselect();
    expect(cleared).toHaveBeenCalledTimes(1);
  });

  it("tells its listeners about a change, and not about a call that changes nothing", () => {
    const layout = new ShellLayout();
    const listener = vi.fn();
    layout.subscribe(listener);
    layout.setRailWidth(RAIL_DEFAULT_WIDTH);
    layout.leaveFocus();
    layout.closeInspector();
    layout.toggleFocus();
    expect(listener).toHaveBeenCalledTimes(1);
  });
});

describe("ModeRegistry", () => {
  it("makes the first mode registered the current one", () => {
    const modes = new ModeRegistry();
    modes.register({ id: "read", title: "Read" });
    modes.register({ id: "comment", title: "Comment" });
    expect(modes.getState().current).toBe("read");
  });

  it("switches: the old mode leaves, clearing what only it could select, then the new one enters", () => {
    const modes = new ModeRegistry();
    const calls: string[] = [];
    modes.register({ id: "read", title: "Read", leave: () => calls.push("leave read") });
    modes.register({ id: "comment", title: "Comment", enter: () => calls.push(`enter ${modes.current()?.id}`) });
    modes.switchTo("comment");
    expect(calls).toEqual(["leave read", "enter comment"]);
  });

  it("ignores a switch to the current mode or to one that isn't registered", () => {
    const modes = new ModeRegistry();
    const leave = vi.fn();
    modes.register({ id: "read", title: "Read", leave });
    modes.switchTo("read");
    modes.switchTo("edit");
    expect([modes.getState().current, leave.mock.calls.length]).toEqual(["read", 0]);
  });

  it("refuses a second mode with the same id", () => {
    const modes = new ModeRegistry();
    modes.register({ id: "read", title: "Read" });
    expect(() => modes.register({ id: "read", title: "Read again" })).toThrow(/already registered/);
  });

  it("moves to the next mode when the current one is removed", () => {
    const modes = new ModeRegistry();
    const removeRead = modes.register({ id: "read", title: "Read" });
    modes.register({ id: "comment", title: "Comment" });
    removeRead();
    expect(modes.getState().current).toBe("comment");
  });
});

describe("shellActions", () => {
  it("toggles the rail, focus mode and each mode", () => {
    const layout = new ShellLayout();
    const modes = new ModeRegistry();
    modes.register({ id: "read", title: "Read" });
    modes.register({ id: "comment", title: "Comment" });
    const actions = shellActions(layout, modes, { rail: true });
    actions["view.toggle-sidebar"]?.(undefined);
    actions["view.focus-mode"]?.(undefined);
    actions["mode.comment"]?.(undefined);
    expect([layout.getState().rail.open, layout.getState().focus, modes.getState().current]).toEqual([
      true,
      true,
      "comment",
    ]);
  });

  it("leaves the sidebar command without a handler when the shell has no rail tabs", () => {
    const actions = shellActions(new ShellLayout(), new ModeRegistry(), { rail: false });
    expect(Object.keys(actions)).toEqual(["view.focus-mode"]);
  });
});
