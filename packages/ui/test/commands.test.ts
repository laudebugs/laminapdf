import { describe, expect, it, vi } from "vitest";
import {
  parseChoice,
  parsePage,
  parsePageRanges,
  parseUrl,
  parseZoom,
  parseZoomPercent,
} from "../src/commands/args.ts";
import { matchText, RecentCommands, rankCommands } from "../src/commands/rank.ts";
import { readerCommands } from "../src/commands/reader-commands.ts";
import { CommandError, CommandRegistry } from "../src/commands/registry.ts";
import { bindingKey, pressKey, shortcutProblems } from "../src/commands/shortcuts.ts";
import type { AnyCommand, CommandContext } from "../src/commands/types.ts";
import { readerCommandsFor } from "./commands.tsx";
import { FakeViewer, fakeFile, fakePlatform } from "./fakes.ts";

const value = <T>(result: { ok: boolean; value?: T }) => (result.ok ? result.value : undefined);
const error = (result: { ok: boolean; error?: string }) => (result.ok ? undefined : result.error);

describe("parsePage", () => {
  it("reads a number from 1 to the page count as a 0-based page", () => {
    expect(value(parsePage(" 12 ", 24))).toBe(11);
    expect(value(parsePage("1", 24))).toBe(0);
  });

  it("reads a page label first, without regard to case", () => {
    const labels = ["i", "ii", "1", "2", "A-1"];
    expect(value(parsePage("a-1", 5, labels))).toBe(4);
    expect(value(parsePage("1", 5, labels))).toBe(2);
  });

  it("names the range for a page that doesn't exist, and asks for something when empty", () => {
    expect(error(parsePage("25", 24))).toBe("There is no page 25. Enter a page from 1 to 24.");
    expect(error(parsePage("0", 24))).toBe("There is no page 0. Enter a page from 1 to 24.");
    expect(error(parsePage("twelve", 24))).toContain("There is no page twelve");
    expect(error(parsePage("  ", 24))).toBe("Type a page number or label.");
  });
});

describe("parsePageRanges", () => {
  it("reads numbers and ranges in the order written, spaces ignored", () => {
    expect(value(parsePageRanges("4-8", 24))).toEqual([3, 4, 5, 6, 7]);
    expect(value(parsePageRanges("4, 6, 9-12", 24))).toEqual([3, 5, 8, 9, 10, 11]);
    expect(value(parsePageRanges("2-2", 24))).toEqual([1]);
  });

  it("refuses anything else, a page past the end and a backwards range", () => {
    expect(error(parsePageRanges("4-", 24))).toBe("Write the pages as 4-8 or 4, 6, 9-12.");
    expect(error(parsePageRanges("a-b", 24))).toBe("Write the pages as 4-8 or 4, 6, 9-12.");
    expect(error(parsePageRanges("", 24))).toBe("Write the pages as 4-8 or 4, 6, 9-12.");
    expect(error(parsePageRanges("4,", 24))).toBe("Write the pages as 4-8 or 4, 6, 9-12.");
    expect(error(parsePageRanges("20-30", 24))).toBe("There is no page 30. Use pages from 1 to 24.");
    expect(error(parsePageRanges("0", 24))).toBe("There is no page 0. Use pages from 1 to 24.");
    expect(error(parsePageRanges("8-4", 24))).toBe("8-4 runs backwards. Write it as 4-8.");
  });
});

describe("parseZoomPercent", () => {
  it("reads 10 to 800, with or without %", () => {
    expect(value(parseZoomPercent("150"))).toBe(150);
    expect(value(parseZoomPercent("150%"))).toBe(150);
    expect(value(parseZoomPercent("12.5 %"))).toBe(12.5);
    expect(value(parseZoomPercent("10"))).toBe(10);
    expect(value(parseZoomPercent("800"))).toBe(800);
  });

  it("refuses a zoom out of range or not a number", () => {
    for (const text of ["9", "801", "", "fit", "-50", "1e3"]) {
      expect(error(parseZoomPercent(text))).toBe(
        "Type a zoom from 10% to 800%, such as 150, or fit width, fit page or actual size.",
      );
    }
  });
});

describe("parseZoom", () => {
  it("takes a percentage or a fit mode by name", () => {
    expect(value(parseZoom("150%"))).toBe(150);
    expect(value(parseZoom("fit width"))).toBe("fit-width");
    expect(value(parseZoom("Fit-Page"))).toBe("fit-page");
    expect(value(parseZoom("actual size"))).toBe("actual-size");
    expect(error(parseZoom("fit"))).toContain("fit width, fit page or actual size");
  });
});

describe("parseUrl and parseChoice", () => {
  it("takes web addresses only", () => {
    expect(value(parseUrl(" https://example.com/a.pdf "))).toBe("https://example.com/a.pdf");
    expect(error(parseUrl("file:///etc/passwd"))).toContain("https://");
    expect(error(parseUrl("example.com"))).toContain("https://");
  });

  it("takes a choice by its label or its value", () => {
    const label = (v: "single" | "two-up") => (v === "single" ? "Single page" : "Two pages");
    expect(value(parseChoice("two pages", ["single", "two-up"], label))).toBe("two-up");
    expect(value(parseChoice("Two-Up", ["single", "two-up"], label))).toBe("two-up");
    expect(error(parseChoice("three", ["single", "two-up"], label))).toBe("Type one of: Single page, Two pages.");
  });
});

/** A context with a document open in `viewer`. */
function opened(viewer = new FakeViewer(), extra: Partial<CommandContext> = {}): CommandContext {
  return readerCommandsFor({
    reader: {
      getState: () => ({ phase: "open", file: fakeFile(), pageCount: 24, mode: "select" }),
      subscribe: () => () => true,
    },
    viewer,
    ...extra,
  }).context;
}

describe("CommandRegistry", () => {
  const command = (over: Partial<AnyCommand> = {}): AnyCommand => ({
    id: "test.do-thing",
    title: "Do the thing",
    group: "app",
    home: ["view-menu"],
    run: vi.fn(),
    ...over,
  });

  it("refuses an id twice, and an id that isn't namespace.verb-object", () => {
    const registry = new CommandRegistry();
    registry.register(command());
    expect(() => registry.register(command())).toThrow("already registered");
    expect(() => registry.register(command({ id: "Bad.Id" }))).toThrow("kebab-case");
    expect(() => registry.register(command({ id: "a.b.c" as never }))).toThrow("kebab-case");
  });

  it("hides a command the platform never can do, and disables one it could", () => {
    const registry = new CommandRegistry();
    registry.register(command({ requires: "canReopenRecent" }));
    const never = readerCommandsFor({ platform: fakePlatform() }).context;
    const could = readerCommandsFor({ platform: fakePlatform({ reasons: { canReopenRecent: "Install it" } }) }).context;
    const can = readerCommandsFor({ platform: fakePlatform({ flags: { canReopenRecent: true } }) }).context;
    expect(registry.availability("test.do-thing", never)).toEqual({ state: "hidden" });
    expect(registry.availability("test.do-thing", could)).toEqual({ state: "disabled", reason: "Install it" });
    expect(registry.availability("test.do-thing", can)).toEqual({ state: "available" });
  });

  it("hides a delegated command until the shell gives its handler, which it then runs", async () => {
    const registry = new CommandRegistry();
    registry.register(command({ delegated: true, run: (ctx, args) => ctx.actions?.["test.do-thing"]?.(args) }));
    const handler = vi.fn();
    expect(registry.availability("test.do-thing", opened()).state).toBe("hidden");
    await registry.run("test.do-thing", opened(undefined, { actions: { "test.do-thing": handler } }));
    expect(handler).toHaveBeenCalledOnce();
  });

  it("runs nothing when the command is disabled or its arguments don't parse", async () => {
    const registry = new CommandRegistry();
    for (const c of readerCommands()) registry.register(c);
    const viewer = new FakeViewer({ currentPage: 23 });
    await expect(registry.run("nav.next-page", opened(viewer))).rejects.toThrow("This is the last page.");
    await expect(registry.run("nav.go-to-page", opened(viewer), { text: "40" })).rejects.toBeInstanceOf(CommandError);
    await expect(registry.run("nav.go-to-page", opened(viewer))).rejects.toThrow("A page number or label");
    await expect(registry.run("test.missing" as never, opened(viewer))).rejects.toThrow("there is no command");
    expect(viewer.calls).toEqual([]);
    await registry.run("nav.go-to-page", opened(viewer), { text: "12" });
    expect(viewer.calls).toEqual([{ method: "jumpToPage", value: 11 }]);
  });

  it("tells subscribers when a command is added or removed", () => {
    const registry = new CommandRegistry();
    const listener = vi.fn();
    registry.subscribe(listener);
    const remove = registry.register(command());
    remove();
    expect(listener).toHaveBeenCalledTimes(2);
    expect(registry.list()).toEqual([]);
  });
});

describe("the reader's commands", () => {
  const commands = readerCommands();

  it("are ADR 0024's 44 and reader mode's 7, each with a visible home besides the palette", () => {
    expect(commands).toHaveLength(51);
    expect(commands.filter((c) => c.home.length === 0).map((c) => c.id)).toEqual([]);
  });

  it("take no key the browser or OS keeps, no unexplained override, and no shared keys, in either shell", () => {
    expect(shortcutProblems(commands, "web")).toEqual([]);
    expect(shortcutProblems(commands, "desktop")).toEqual([]);
  });

  it("drive the viewer: steps, limits and zoom modes", async () => {
    const registry = new CommandRegistry();
    for (const c of commands) registry.register(c);
    const viewer = new FakeViewer({ currentPage: 2, percent: 110 });
    const ctx = opened(viewer);
    for (const id of ["nav.next-page", "nav.previous-page", "nav.first-page", "nav.last-page"] as const) {
      await registry.run(id, ctx);
    }
    await registry.run("view.zoom-in", ctx);
    await registry.run("view.fit-width", ctx);
    await registry.run("view.set-zoom", ctx, { text: "150%" });
    await registry.run("view.rotate-display", ctx, { text: "counter-clockwise" });
    expect(viewer.calls).toEqual([
      { method: "goToPage", value: 3 },
      { method: "goToPage", value: 1 },
      { method: "jumpToPage", value: 0 },
      { method: "jumpToPage", value: 23 },
      { method: "setZoom", value: 125 },
      { method: "setZoom", value: "fit-width" },
      { method: "setZoom", value: 150 },
      { method: "rotate", value: -90 },
    ]);
    expect(registry.availability("view.zoom-in", opened(new FakeViewer({ percent: 800 })))).toEqual({
      state: "disabled",
      reason: "The zoom is already at its largest.",
    });
  });

  it("ask for a document first when none is open", () => {
    const registry = new CommandRegistry();
    for (const c of commands) registry.register(c);
    const { context } = readerCommandsFor({
      reader: { getState: () => ({ phase: "empty" }), subscribe: () => () => true },
    });
    expect(registry.availability("nav.next-page", context)).toEqual({
      state: "disabled",
      reason: "Open a document first.",
    });
  });

  it("leave Open from URL to a platform that can open URLs", () => {
    const registry = new CommandRegistry();
    for (const c of commands) registry.register(c);
    const actions = { "file.open-url": () => undefined };
    const without = readerCommandsFor({ platform: fakePlatform(), actions }).context;
    const able = readerCommandsFor({ platform: fakePlatform({ flags: { canOpenUrl: true } }), actions }).context;
    expect(registry.availability("file.open-url", without).state).toBe("hidden");
    expect(registry.availability("file.open-url", able).state).toBe("available");
  });

  it("name the save command for the platform", () => {
    const save = commands.find((c) => c.id === "file.save-copy");
    const title = (flags: { canSaveAsDialog: boolean }) =>
      typeof save?.title === "function"
        ? save.title(readerCommandsFor({ platform: fakePlatform({ flags }) }).context)
        : "";
    expect(title({ canSaveAsDialog: false })).toBe("Download a copy");
    expect(title({ canSaveAsDialog: true })).toBe("Save a copy…");
  });
});

describe("shortcut keys", () => {
  const press = (
    key: string,
    mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {},
    code?: string,
  ) =>
    pressKey({
      key,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      metaKey: false,
      ...mods,
      ...(code === undefined ? {} : { code }),
    });

  it("read Mod as ⌘ on macOS and Ctrl elsewhere", () => {
    expect(bindingKey("Mod+K", "mac")).toBe(press("k", { metaKey: true }));
    expect(bindingKey("Mod+K", "other")).toBe(press("k", { ctrlKey: true }));
    expect(bindingKey("Mod+Shift+G", "other")).toBe(press("G", { ctrlKey: true, shiftKey: true }));
  });

  it("treat a symbol's Shift as part of it, and = as the + key", () => {
    expect(bindingKey("?", "other")).toBe(press("?", { shiftKey: true }));
    expect(bindingKey("+", "other")).toBe(press("+", { shiftKey: true }));
    expect(bindingKey("Mod++", "mac")).toBe(press("=", { metaKey: true }));
  });

  it("read a ⌥ letter on macOS from its code, since ⌥ changes the character", () => {
    expect(bindingKey("Alt+Mod+I", "mac")).toBe(press("ˆ", { altKey: true, metaKey: true }, "KeyI"));
  });

  it("report a never-bind key, an unexplained override, Ctrl+Alt+letter and shared keys", () => {
    const bound = (id: string, mac: string, other = mac, scope: "global" | "page" = "global"): AnyCommand => ({
      id: id as never,
      title: id,
      group: "app",
      home: ["view-menu"],
      shortcuts: [{ mac, other, scope }],
      run: () => undefined,
    });
    const close = shortcutProblems([bound("a.close", "Mod+W")], "web");
    expect(close.filter((p) => p.includes("may never be bound"))).toHaveLength(2);
    expect(shortcutProblems([bound("a.close", "Mod+W")], "desktop")).toEqual([]);
    expect(shortcutProblems([bound("a.undo", "Mod+Z")], "web")[0]).toContain("without a reason");
    expect(shortcutProblems([bound("a.euro", "Alt+Mod+E", "Ctrl+Alt+E")], "web")[0]).toContain("Ctrl+Alt+letter");
    expect(shortcutProblems([bound("a.one", "J", "J", "page"), bound("a.two", "J", "J", "page")], "web")).toHaveLength(
      2,
    );
    expect(
      shortcutProblems([bound("a.one", "J", "J", "page"), bound("a.two", "J", "J", "global")], "web"),
    ).toHaveLength(2);
  });
});

describe("rankCommands", () => {
  const items = [
    { id: "view.zoom-in", title: "Zoom in", keywords: ["bigger"] },
    { id: "view.fit-page", title: "Fit page", keywords: ["zoom"] },
    { id: "nav.next-page", title: "Next page" },
    { id: "view.rotate-display", title: "Rotate view", keywords: ["turn"] },
  ];

  it("ranks a title that starts with the query, then a word start, then letters in order", () => {
    expect(rankCommands("page", items).map((r) => r.item.id)).toEqual(["view.fit-page", "nav.next-page"]);
    expect(rankCommands("zoom", items).map((r) => r.item.id)).toEqual(["view.zoom-in", "view.fit-page"]);
    expect(rankCommands("nxpg", items).map((r) => r.item.id)).toEqual(["nav.next-page"]);
    expect(rankCommands("turn", items).map((r) => r.item.id)).toEqual(["view.rotate-display"]);
  });

  it("highlights the matched parts of the title, and puts recent commands first among equals", () => {
    expect(matchText("pa", "Fit page")?.ranges).toEqual([[4, 6]]);
    expect(rankCommands("", items, ["nav.next-page"])[0]?.item.id).toBe("nav.next-page");
  });

  it("ranks 44 commands quickly enough for every keystroke", () => {
    const many = readerCommands().map((c) => ({ id: c.id, title: String(c.title), keywords: c.keywords ?? [] }));
    const start = performance.now();
    for (const query of ["z", "zo", "zoo", "zoom", "zoom 1", "go to 12", "pr", "pri"]) rankCommands(query, many);
    // Eight keystrokes; the palette's budget is 16 ms each, rendering included.
    expect(performance.now() - start).toBeLessThan(16);
  });
});

describe("RecentCommands", () => {
  it("keeps the newest first, without repeats, up to its maximum", () => {
    const stored = new Map<string, string>();
    const storage = {
      getItem: (k: string) => stored.get(k) ?? null,
      setItem: (k: string, v: string) => stored.set(k, v),
    };
    const recent = new RecentCommands({ storage, max: 2 });
    for (const id of ["a.one", "a.two", "a.one", "a.three"]) recent.add(id);
    expect(recent.list()).toEqual(["a.three", "a.one"]);
    expect(new RecentCommands({ storage, max: 2 }).list()).toEqual(["a.three", "a.one"]);
  });

  it("keeps working in memory when storage throws or holds something else", () => {
    const broken = {
      getItem: () => "{not json",
      setItem: () => {
        throw new Error("quota");
      },
    };
    const recent = new RecentCommands({ storage: broken });
    expect(recent.list()).toEqual([]);
    recent.add("a.one");
    expect(recent.list()).toEqual(["a.one"]);
  });
});
