import axe from "axe-core";
import { useState } from "react";
import { afterEach, describe, expect, it, onTestFinished, vi } from "vitest";
import { userEvent } from "vitest/browser";
import type { CommandRegistry } from "../../src/commands/registry.ts";
import type { CommandContext, CommandId } from "../../src/commands/types.ts";
import { CommandsProvider, RecentCommands, RegistryPalette } from "../../src/index.ts";
import { readerCommandsFor } from "../commands.tsx";
import { FakeViewer, fakeFile, fakePlatform } from "../fakes.ts";
import { cleanup, focused, renderIn, settled } from "./render.tsx";

afterEach(cleanup);

const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));
const OPEN = { phase: "open" as const, file: fakeFile(), pageCount: 24, mode: "select" as const };
const memory = () => new RecentCommands({ storage: { getItem: () => null, setItem: () => undefined } });

/** The palette over `registry`, with a page to focus and a field to type in; `shell.setOpen` opens it. */
function PaletteApp({
  registry,
  context,
  shell,
}: {
  registry: CommandRegistry;
  context: CommandContext;
  shell: { setOpen: (open: boolean) => void };
}) {
  const [open, setOpen] = useState(false);
  shell.setOpen = setOpen;
  return (
    <CommandsProvider registry={registry} context={context} keys="other">
      <main data-shortcut-scope="page" tabIndex={-1} aria-label="Page">
        <input aria-label="Page number" />
      </main>
      <RegistryPalette isOpen={open} onOpenChange={setOpen} recent={memory()} />
    </CommandsProvider>
  );
}

/** Every delegated command's handler, as spies, plus the palette's own. */
function setup(viewer = new FakeViewer({ currentPage: 2 })) {
  const calls: { id: string; args: unknown }[] = [];
  const reader = {
    getState: () => OPEN,
    subscribe: () => () => true,
    openFromPicker: vi.fn(async () => undefined),
  };
  let openPalette = () => {};
  const { registry, context: base } = readerCommandsFor({ reader, viewer });
  const actions = Object.fromEntries(
    registry
      .list()
      .filter((c) => c.delegated === true)
      .map((c) => [
        c.id,
        (args: unknown) => (c.id === "palette.open" ? openPalette() : calls.push({ id: c.id, args })),
      ]),
  ) as Partial<Record<CommandId, (args: unknown) => void>>;
  const context = {
    ...base,
    platform: fakePlatform({ flags: { canReopenRecent: true, canOpenUrl: true, canShare: true } }),
    actions,
  };
  const shell = { setOpen: (_open: boolean) => {} };
  openPalette = () => shell.setOpen(true);
  const app = <PaletteApp registry={registry} context={context} shell={shell} />;
  return { app, registry, viewer, reader, calls };
}

const palette = () => document.querySelector(".paper-palette") as HTMLElement | null;
const rows = () => [...document.querySelectorAll(".paper-palette-row")] as HTMLElement[];
const page = () => document.querySelector("main") as HTMLElement;

/** Opens the palette with Ctrl+K from the page, and types `text`. */
async function search(text: string) {
  page().focus();
  await userEvent.keyboard("{Control>}k{/Control}");
  await frame();
  expect(palette()).not.toBeNull();
  if (text !== "") await userEvent.keyboard(text);
  await frame();
}

/** What to type to run each command in one step: its title, or its title with arguments. */
const INLINE: Partial<Record<CommandId, string>> = {
  "nav.go-to-page": "go to 12",
  "view.set-zoom": "zoom 150",
  "file.open-url": "Open from URL https://example.com/a.pdf",
  "view.page-layout": "Page layout two pages",
  "view.reader-line-spacing": "Line spacing loose",
  "view.reader-line-length": "Line length wide",
  "view.page-appearance": "Page appearance sepia",
  "view.theme": "Theme dark",
  "view.accent": "Accent colour plum",
};

/** Sixteen or so search-and-run cycles per test: about 10 s each in Firefox on a busy runner. */
const EVERY_COMMAND_MS = 60_000;
/** The registry's commands are run in this many tests, so none nears its time on a loaded run. */
const PARTS = 4;

describe("RegistryPalette", () => {
  const ranInPart: string[][] = [];
  for (let part = 0; part < PARTS; part++) {
    it(`runs every registered command the toolbar and menus can reach, each from its first row (part ${part + 1} of ${PARTS})`, {
      timeout: EVERY_COMMAND_MS,
    }, async () => {
      const { app, registry, viewer, reader, calls } = setup();
      renderIn(app);
      // The password goes in the password prompt, never in the palette; the palette itself is open.
      const all = registry.list().filter((c) => c.id !== "file.unlock" && c.id !== "palette.open");
      const size = Math.ceil(all.length / PARTS);
      const mine = all.slice(part * size, (part + 1) * size);
      const ran: string[] = [];
      for (const command of mine) {
        const title = registry.title(command, { platform: fakePlatform() });
        await search(INLINE[command.id] ?? title);
        await userEvent.keyboard("{Enter}");
        await frame();
        expect(palette(), `${command.id} closes the palette`).toBeNull();
        ran.push(command.id);
      }
      ranInPart[part] = ran;
      // Each delegated command reached its handler, with parsed arguments where it takes them.
      const ids = new Set<string>(mine.map((c) => c.id));
      const expectArgs = (id: CommandId, value: unknown) => {
        if (ids.has(id)) expect(calls.find((c) => c.id === id)?.args, id).toBe(value);
      };
      expectArgs("view.page-layout", "two-up");
      expectArgs("view.reader-line-spacing", "loose");
      expectArgs("view.reader-line-length", "wide");
      expectArgs("file.open-url", "https://example.com/a.pdf");
      expect(new Set(calls.map((c) => c.id)).size).toBe(calls.length);
      if (ids.has("file.open")) expect(reader.openFromPicker).toHaveBeenCalledOnce();
      if (ids.has("nav.go-to-page")) expect(viewer.calls).toContainEqual({ method: "jumpToPage", value: 11 });
      if (ids.has("view.set-zoom")) expect(viewer.calls).toContainEqual({ method: "setZoom", value: 150 });
    });
  }

  it("ran all 71 commands across the parts", () => {
    expect(ranInPart.flat()).toHaveLength(71);
  });

  it("goes to a page and zooms in one step, and runs nothing for arguments that don't parse", async () => {
    const { app, viewer } = setup();
    renderIn(app);
    await search("go to 99");
    expect(rows()[0]?.textContent).toContain("There is no page 99. Enter a page from 1 to 24.");
    expect(rows()[0]?.getAttribute("aria-disabled")).toBe("true");
    await userEvent.keyboard("{Enter}");
    expect(viewer.calls).toEqual([]);
    await userEvent.keyboard("{Escape}");
    await search("zoom 150");
    expect(rows()[0]?.textContent).toContain("Zoom to: 150%");
    await userEvent.keyboard("{Enter}");
    expect(viewer.calls).toEqual([{ method: "setZoom", value: 150 }]);
  });

  it("takes a fit mode as Zoom to's argument, and shows no error row for a search that starts a title", async () => {
    const { app, viewer } = setup();
    renderIn(app);
    await search("zoom in");
    expect(rows()[0]?.textContent).toContain("Zoom in");
    expect(rows().some((r) => r.textContent?.includes("Type a zoom"))).toBe(false);
    await userEvent.keyboard("{Escape}");
    await search("zoom fit width");
    expect(rows()[0]?.textContent).toContain("Zoom to: Fit width");
    await userEvent.keyboard("{Enter}");
    expect(viewer.calls).toEqual([{ method: "setZoom", value: "fit-width" }]);
  });

  it("jumps to a page label in one step", async () => {
    const viewer = new FakeViewer();
    const { registry, context } = readerCommandsFor({
      reader: { getState: () => OPEN, subscribe: () => () => true },
      viewer,
      labels: [...Array.from({ length: 21 }, (_, n) => String(n + 1)), "A-1", "A-2", "A-3"],
      actions: {},
    });
    renderIn(
      <CommandsProvider registry={registry} context={context} keys="other" listen={false}>
        <RegistryPalette isOpen onOpenChange={() => {}} recent={memory()} />
      </CommandsProvider>,
    );
    await frame();
    await userEvent.keyboard("go to a-2");
    await frame();
    expect(rows()[0]?.textContent).toContain("Go to page: A-2");
    await userEvent.keyboard("{Enter}");
    expect(viewer.calls).toEqual([{ method: "jumpToPage", value: 22 }]);
  });

  it("asks for missing arguments on a page of its own, and says why a try didn't parse", async () => {
    const { app, viewer } = setup();
    renderIn(app);
    await search("Go to page");
    await userEvent.keyboard("{Enter}");
    await frame();
    expect(palette()?.textContent).toContain("A page number or label");
    expect(focused().getAttribute("aria-expanded")).toBe("false");
    await userEvent.keyboard("99{Enter}");
    await frame();
    expect(document.querySelector("[role=alert]")?.textContent).toBe("There is no page 99. Enter a page from 1 to 24.");
    expect(viewer.calls).toEqual([]);
    await userEvent.keyboard("{Backspace}{Backspace}12{Enter}");
    await frame();
    expect(palette()).toBeNull();
    expect(viewer.calls).toEqual([{ method: "jumpToPage", value: 11 }]);
  });

  it("shows a disabled command with why, each row's shortcut, and the result count", async () => {
    const { app } = setup(new FakeViewer({ currentPage: 23 }));
    renderIn(app);
    await search("next page");
    const next = rows().find((r) => r.textContent?.startsWith("Next page"));
    expect(next?.textContent).toContain("This is the last page.");
    expect(next?.textContent).toContain("PgDn");
    expect(document.querySelector(".paper-palette-footer [role=status]")?.textContent).toMatch(/^\d+ commands?$/);
  });

  it("lists recent commands first", async () => {
    const { app } = setup();
    renderIn(app);
    await search("Fit page");
    await userEvent.keyboard("{Enter}");
    await search("");
    expect(document.querySelector(".paper-palette-overline")?.textContent).toBe("Recent");
    expect(rows()[0]?.textContent).toContain("Fit page");
  });

  it("passes axe with the palette open", async () => {
    const { app } = setup();
    renderIn(app);
    await search("zoom");
    // As in the palette shell's own test: a row's group and chip describe it rather than name it,
    // which axe's experimental label-content-name-mismatch doesn't allow for; checked here instead.
    for (const row of rows()) {
      const described = (row.getAttribute("aria-describedby") ?? "")
        .split(" ")
        .map((id) => document.getElementById(id));
      for (const meta of row.querySelectorAll(".paper-palette-meta")) expect(described).toContain(meta);
    }
    await settled();
    const { violations } = await axe.run(document.body, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
      rules: { "label-content-name-mismatch": { enabled: false } },
    });
    const serious = violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)).toEqual([]);
  });
});

describe("the shortcut dispatcher", () => {
  it("runs a page key only while the page has focus, never while typing in a field", async () => {
    const { app, viewer } = setup();
    renderIn(app);
    page().focus();
    await userEvent.keyboard("{PageDown}");
    (document.querySelector("input[aria-label='Page number']") as HTMLInputElement).focus();
    await userEvent.keyboard("{PageDown}+");
    document.body.focus();
    await userEvent.keyboard("{PageDown}");
    expect(viewer.calls).toEqual([{ method: "goToPage", value: 3 }]);
  });

  it("runs a global key with a modifier even from a text field", async () => {
    const { app } = setup();
    renderIn(app);
    (document.querySelector("input[aria-label='Page number']") as HTMLInputElement).focus();
    await userEvent.keyboard("{Control>}k{/Control}");
    await frame();
    expect(palette()).not.toBeNull();
  });

  it("runs nothing behind a modal, such as the palette itself", async () => {
    const { app, viewer, reader } = setup();
    renderIn(app);
    await search("");
    await userEvent.keyboard("{Control>}o{/Control}{PageDown}");
    expect(reader.openFromPicker).not.toHaveBeenCalled();
    expect(viewer.calls).toEqual([]);
  });

  it("runs a key pressed in a modal that is leaving, as its exit plays", async () => {
    const { app } = setup();
    renderIn(app);
    // A dialog fading out keeps focus until it leaves the page; it no longer holds the keys.
    const leaving = document.createElement("div");
    leaving.setAttribute("data-exiting", "true");
    leaving.innerHTML = '<div role="dialog" aria-modal="true"><button type="button">Open</button></div>';
    document.body.append(leaving);
    onTestFinished(() => leaving.remove());
    (leaving.querySelector("button") as HTMLButtonElement).focus();
    await userEvent.keyboard("{Control>}k{/Control}");
    await frame();
    expect(palette()).not.toBeNull();
  });

  it("keeps the page's keys inside a selection on the page", async () => {
    const { registry, context } = readerCommandsFor({
      reader: { getState: () => OPEN, subscribe: () => () => true },
      viewer: new FakeViewer({ currentPage: 2 }),
    });
    const viewer = context.viewer as FakeViewer;
    renderIn(
      <CommandsProvider registry={registry} context={context} keys="other">
        <main data-shortcut-scope="page">
          <section data-shortcut-scope="selection" tabIndex={-1} aria-label="Selection" />
        </main>
      </CommandsProvider>,
    );
    (document.querySelector("[data-shortcut-scope=selection]") as HTMLElement).focus();
    await userEvent.keyboard("{PageDown}");
    expect(viewer.calls).toEqual([{ method: "goToPage", value: 3 }]);
  });

  it("leaves a key alone when its command can't run, so the browser keeps it", async () => {
    const { app, viewer } = setup(new FakeViewer({ currentPage: 23 }));
    renderIn(app);
    page().focus();
    const event = new KeyboardEvent("keydown", { key: "PageDown", bubbles: true, cancelable: true });
    page().dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
    expect(viewer.calls).toEqual([]);
  });
});
