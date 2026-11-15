// The window's document tabs: one Tab stop that the arrow keys move along, Enter shows the focused
// tab, Delete closes it, the close buttons and the strip's menu reach every tab, dragging a tab
// across the strip reorders it (by the mouse, through Playwright), a marker and the grabbing cursor
// show while it is dragged, a release anywhere else is handed to the shell, and Escape cancels. A
// tab whose file is missing carries a named warning mark, and the menu says "Not found" or each
// tab's page; the restore notice counts what came back and shows a missing file.

import { useState } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { page, userEvent } from "vitest/browser";
import { type DocumentTab, DocumentTabs, LocateDifferentDialog, RestoreNotice } from "../../src/index.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(() => cleanup());

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
    const onto = page.getByRole("tab", { name: "notes.pdf" });
    const width = onto.element().getBoundingClientRect().width;
    await userEvent.dragAndDrop(page.getByRole("tab", { name: "report.pdf" }), onto, {
      targetPosition: { x: width - 4, y: 8 },
    });
    // The release that ends the drag clicks too, and shows no tab.
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
    await userEvent.dragAndDrop(
      page.getByRole("tab", { name: "minutes.pdf" }),
      page.getByRole("tab", { name: "report.pdf" }),
      {
        targetPosition: { x: 4, y: 8 },
      },
    );
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
    const mine = page.getByRole("tab", { name: "report.pdf" }).nth(0);
    const theirs = page.getByRole("tab", { name: "notes.pdf" }).nth(1);
    await userEvent.dragAndDrop(mine, theirs, { targetPosition: { x: 4, y: 8 } });
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
