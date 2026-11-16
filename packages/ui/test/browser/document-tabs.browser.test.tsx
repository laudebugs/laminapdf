// The window's document tabs: one Tab stop that the arrow keys move along, Enter shows the focused
// tab, Delete closes it, the close buttons and the strip's menu reach every tab, dragging a tab
// across the strip reorders it, a marker and the grabbing cursor
// show while it is dragged, a release anywhere else is handed to the shell, and Escape cancels. A
// tab whose file is missing carries a named warning mark, and the menu says "Not found" or each
// tab's page; the restore notice counts what came back and shows a missing file.

import { useState } from "react";
import { afterEach, describe, expect, it, onTestFinished } from "vitest";
import { commands, page, userEvent } from "vitest/browser";
import { type DocumentTab, DocumentTabs, LocateDifferentDialog, RestoreNotice } from "../../src/index.ts";
import { cleanup, renderIn, settled } from "./render.tsx";

afterEach(() => cleanup());

declare module "vitest/browser" {
  interface BrowserCommands {
    /** Makes the page prefer reduced motion or not, whatever the system says, or follow the system again with `null`. */
    reducedMotion: (value: "reduce" | "no-preference" | null) => Promise<void>;
  }
}

const TABS: DocumentTab[] = [
  { id: "a", title: "report.pdf" },
  { id: "b", title: "notes.pdf" },
  { id: "c", title: "minutes.pdf" },
];

/** The strip over its own state, recording what it was asked to do. */
function Strip({ log }: { log: string[] }) {
  const [tabs, setTabs] = useState(TABS);
  const [active, setActive] = useState<string | undefined>("a");
  return (
    <DocumentTabs
      tabs={tabs}
      active={active}
      onActivate={(id) => {
        log.push(`activate ${id}`);
        setActive(id);
      }}
      onClose={(id) => {
        log.push(`close ${id}`);
        setTabs((all) => all.filter((t) => t.id !== id));
      }}
      onReorder={(ids) => {
        log.push(`reorder ${ids.join("")}`);
        setTabs((all) => ids.map((id) => all.find((t) => t.id === id) as DocumentTab));
      }}
      moves={[{ id: "new", label: "Move tab to new window" }]}
      onMove={(id) => log.push(`move ${id}`)}
      onDragStart={(id) => log.push(`drag ${id}`)}
      onDragOut={(id, outside) => log.push(`out ${id} ${outside ? "outside" : "inside"}`)}
      onDragCancel={(id) => log.push(`cancel ${id}`)}
      platform="mac"
    />
  );
}

describe("the document tabs", () => {
  it("are one tab stop; the arrows move along them and Enter shows the focused one", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const tabs = page.getByRole("tab");
    expect(tabs.elements().map((tab) => tab.getAttribute("tabindex"))).toEqual(["0", "-1", "-1"]);
    await userEvent.click(page.getByRole("tab", { name: "report.pdf" }));
    await userEvent.keyboard("{ArrowRight}");
    expect(document.activeElement?.textContent).toBe("notes.pdf");
    await userEvent.keyboard("{End}");
    expect(document.activeElement?.textContent).toBe("minutes.pdf");
    await userEvent.keyboard("{ArrowRight}");
    expect(document.activeElement?.textContent).toBe("report.pdf");
    await userEvent.keyboard("{ArrowLeft}{Enter}");
    expect(log.at(-1)).toBe("activate c");
    expect(page.getByRole("tab", { name: "minutes.pdf" }).element().getAttribute("aria-selected")).toBe("true");
  });

  it("close the focused tab with Delete, and move focus to the next", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    await userEvent.click(page.getByRole("tab", { name: "notes.pdf" }));
    await userEvent.keyboard("{Delete}");
    expect(log).toContain("close b");
    expect(page.getByRole("tab").elements()).toHaveLength(2);
    expect(document.activeElement?.textContent).toBe("minutes.pdf");
  });

  it("close a tab with its close button", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const close = document.querySelector<HTMLButtonElement>('button[title="Close notes.pdf"]');
    close?.click();
    expect(log).toEqual(["close b"]);
  });

  it("list every tab and the moves in the strip's menu", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    await page.getByRole("button", { name: "All tabs, 3 open" }).click();
    await page.getByRole("menuitem", { name: "minutes.pdf" }).click();
    await page.getByRole("button", { name: "All tabs, 3 open" }).click();
    await page.getByRole("menuitem", { name: "Move tab to new window" }).click();
    expect(log).toEqual(["activate c", "move new"]);
  });

  it("move a tab one step right when dropped past its right-hand neighbour's middle", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const onto = page.getByRole("tab", { name: "notes.pdf" }).element().getBoundingClientRect();
    const from = page.getByRole("tab", { name: "report.pdf" }).element();
    drag(from, { x: onto.right - 4, y: onto.top + 8 });
    // The click a browser sends after the release that ends the drag shows no tab.
    from.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    await expect.poll(() => log).toEqual(["drag a", "reorder bac"]);
  });

  it("open a tab's context menu with Shift+F10, showing that tab first", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    await userEvent.click(page.getByRole("tab", { name: "report.pdf" }));
    await userEvent.keyboard("{ArrowRight}{Shift>}{F10}{/Shift}");
    await page.getByRole("menuitem", { name: "Move tab to new window" }).click();
    expect(log).toEqual(["activate a", "activate b", "move new"]);
  });

  it("reorder when a tab is dropped on another's first half", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const onto = page.getByRole("tab", { name: "report.pdf" }).element().getBoundingClientRect();
    drag(page.getByRole("tab", { name: "minutes.pdf" }).element(), { x: onto.left + 4, y: onto.top + 8 });
    await expect.poll(() => log).toEqual(["drag c", "reorder cab"]);
  });

  it("show where the tab would go and the grabbing cursor while it is dragged", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const from = page.getByRole("tab", { name: "minutes.pdf" }).element();
    const onto = page.getByRole("tab", { name: "report.pdf" }).element().getBoundingClientRect();
    press(from, middleOf(from.getBoundingClientRect()));
    move(from, { x: onto.left + 4, y: onto.top + 8 });
    await expect
      .poll(() => [document.querySelector('[data-drop="before"]')?.textContent, getComputedStyle(from).cursor])
      .toEqual(["report.pdf", "grabbing"]);
    release(from, { x: onto.left + 4, y: onto.top + 8 });
    await expect
      .poll(() => [document.querySelector("[data-drop]"), "tabDragging" in document.documentElement.dataset])
      .toEqual([null, false]);
  });

  it("hand a tab released off the strip to the shell, inside the page or out of it", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const from = page.getByRole("tab", { name: "notes.pdf" }).element();
    const start = middleOf(from.getBoundingClientRect());
    const below = { x: start.x, y: start.y + 200 };
    press(from, start);
    move(from, below);
    release(from, below);
    press(from, start);
    move(from, below);
    release(from, { x: -20, y: -20 });
    await expect.poll(() => log).toEqual(["drag b", "out b inside", "drag b", "out b outside"]);
  });

  it("leave the tab where it was when Escape cancels the drag", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const from = page.getByRole("tab", { name: "notes.pdf" }).element();
    const start = middleOf(from.getBoundingClientRect());
    press(from, start);
    move(from, { x: start.x, y: start.y + 200 });
    await userEvent.keyboard("{Escape}");
    release(from, { x: start.x, y: start.y + 200 });
    expect(log).toEqual(["drag b", "cancel b"]);
  });

  it("show a tab on a click that doesn't move", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    await userEvent.click(page.getByRole("tab", { name: "minutes.pdf" }));
    expect(log).toEqual(["activate c"]);
  });

  it("hand a tab dropped on another strip to the shell, not to that strip", async () => {
    const log: string[] = [];
    const other: string[] = [];
    renderIn(
      <>
        <Strip log={log} />
        <Strip log={other} />
      </>,
    );
    const mine = page.getByRole("tab", { name: "report.pdf" }).nth(0).element();
    const theirs = page.getByRole("tab", { name: "notes.pdf" }).nth(1).element().getBoundingClientRect();
    drag(mine, { x: theirs.left + 4, y: theirs.top + 8 });
    await expect.poll(() => log).toEqual(["drag a", "out a inside"]);
    expect(other).toEqual([]);
  });

  it("mark a missing file's tab, and say Not found or each tab's page in the menu", async () => {
    renderIn(
      <DocumentTabs
        tabs={[
          { id: "a", title: "report.pdf", detail: "Page 12" },
          { id: "b", title: "gone.pdf", missing: true },
        ]}
        active="a"
        onActivate={() => undefined}
        onClose={() => undefined}
        platform="mac"
      />,
    );
    await expect.element(page.getByRole("tab", { name: "gone.pdf, not found" })).toBeVisible();
    await page.getByRole("button", { name: "All tabs, 2 open" }).click();
    await expect.element(page.getByRole("menuitem", { name: "report.pdf" })).toHaveAccessibleDescription("Page 12");
    await expect.element(page.getByRole("menuitem", { name: "gone.pdf" })).toHaveAccessibleDescription("Not found");
  });
});

describe("the restore notice", () => {
  it("counts the tabs and the missing files, and shows a missing one", async () => {
    const log: string[] = [];
    renderIn(
      <RestoreNotice
        tabs={8}
        missing={1}
        onShowMissing={() => log.push("show")}
        onDismiss={() => log.push("dismiss")}
      />,
    );
    await expect
      .element(page.getByRole("status"))
      .toHaveTextContent(
        "Restored your last session. 8 tabs, each at the page you were reading. 1 file was not found.",
      );
    await page.getByRole("button", { name: "Show missing file" }).click();
    await page.getByRole("button", { name: "Dismiss the restore notice" }).click();
    expect(log).toEqual(["show", "dismiss"]);
  });

  it("offers no missing file to show once none is missing", async () => {
    renderIn(<RestoreNotice tabs={1} missing={0} onDismiss={() => undefined} />);
    await expect
      .element(page.getByRole("status"))
      .toHaveTextContent("Restored your last session. 1 tab, at the page you were reading.");
    expect(page.getByRole("button", { name: "Show missing file" }).elements()).toEqual([]);
  });

  it("asks before a located file that looks different opens, Cancel first", async () => {
    const answers: boolean[] = [];
    renderIn(<LocateDifferentDialog name="gone.pdf" onAnswer={(open) => answers.push(open)} />);
    const ask = page.getByRole("alertdialog", { name: "This file looks different from the one you had open" });
    await expect.element(ask).toBeVisible();
    await expect.element(ask.getByRole("button", { name: "Cancel" })).toHaveFocus();
    await ask.getByRole("button", { name: "Open anyway" }).click();
    expect(answers).toEqual([true]);
  });
});

/** The strip over tabs the test sets: Reverse reorders them, Add opens a fourth. */
function Moving() {
  const [tabs, setTabs] = useState(TABS);
  return (
    <>
      <DocumentTabs tabs={tabs} active="a" onActivate={() => {}} onClose={() => {}} platform="mac" />
      <button type="button" onClick={() => setTabs((all) => [...all].reverse())}>
        Reverse
      </button>
      <button type="button" onClick={() => setTabs((all) => [...all, { id: "d", title: "budget.pdf" }])}>
        Add
      </button>
    </>
  );
}

/** The keyframes of the animations running on tab `id`'s box. */
const motion = (id: string) =>
  (document.querySelector(`[data-tab-box="${id}"]`) as HTMLElement)
    .getAnimations()
    .map((a) => (a.effect as KeyframeEffect).getKeyframes());

describe("the document tabs' motion", () => {
  it("draws the strip still when it appears", () => {
    renderIn(<Moving />);
    expect(["a", "b", "c"].flatMap(motion)).toEqual([]);
  });

  it("slides tabs to their new places, and fades a new tab in, growing", async () => {
    // WebKit on Linux prefers reduced motion: ask for motion first.
    await commands.reducedMotion("no-preference");
    onTestFinished(() => commands.reducedMotion(null));
    renderIn(<Moving />);
    await userEvent.click(page.getByRole("button", { name: "Reverse" }));
    // The first and last tabs swap, so both slide; the middle one stays.
    for (const id of ["a", "c"]) expect(motion(id)[0]?.[0]?.transform).toMatch(/^translateX\(-?\d/);
    expect(motion("b")).toEqual([]);
    await userEvent.click(page.getByRole("button", { name: "Add" }));
    const [from] = motion("d")[0] ?? [];
    expect(Number(from?.opacity)).toBe(0);
    expect(from?.transform).toBe("scale(0.97)");
  });

  it("under reduced motion slides nothing, and a new tab only fades", async () => {
    document.documentElement.setAttribute("data-motion", "reduce");
    onTestFinished(() => document.documentElement.removeAttribute("data-motion"));
    renderIn(<Moving />);
    await userEvent.click(page.getByRole("button", { name: "Reverse" }));
    expect(["a", "b", "c"].flatMap(motion)).toEqual([]);
    await userEvent.click(page.getByRole("button", { name: "Add" }));
    const [from] = motion("d")[0] ?? [];
    expect(Number(from?.opacity)).toBe(0);
    expect(from?.transform).toBe("scale(1)");
  });
});

describe("the document tabs' motion after a resize", () => {
  it("slides tabs from where they are after the strip shrank, not from before it", async () => {
    const host = renderIn(<Moving />);
    host.style.inlineSize = "900px";
    await settled();
    host.style.inlineSize = "420px";
    // The resize observer measures again before the next change.
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const before = (document.querySelector('[data-tab-box="b"]') as HTMLElement).offsetLeft;
    await userEvent.click(page.getByRole("button", { name: "Reverse" }));
    const after = (document.querySelector('[data-tab-box="b"]') as HTMLElement).offsetLeft;
    // The middle tab stays in the middle: no slide from its place in the wider strip.
    expect(after).toBe(before);
    expect(motion("b")).toEqual([]);
  });
});

/** The middle of `box`. */
function middleOf(box: DOMRect): { x: number; y: number } {
  return { x: box.left + box.width / 2, y: box.top + box.height / 2 };
}

/** A mouse pointer event of `type` at `at`, dispatched to `target`. */
function pointer(target: Element, type: string, at: { x: number; y: number }, buttons: number): void {
  const init = { bubbles: true, cancelable: true, pointerId: 1, pointerType: "mouse", isPrimary: true };
  target.dispatchEvent(new PointerEvent(type, { ...init, button: 0, buttons, clientX: at.x, clientY: at.y }));
}

const press = (target: Element, at: { x: number; y: number }) => pointer(target, "pointerdown", at, 1);
const move = (target: Element, at: { x: number; y: number }) => pointer(target, "pointermove", at, 1);
const release = (target: Element, at: { x: number; y: number }) => pointer(target, "pointerup", at, 0);

/** Presses `from` in its middle, moves to `to` and releases there. Dispatched rather than through
 * Playwright's mouse, which every test file running at once shares. */
function drag(from: Element, to: { x: number; y: number }): void {
  press(from, middleOf(from.getBoundingClientRect()));
  move(from, to);
  release(from, to);
}
