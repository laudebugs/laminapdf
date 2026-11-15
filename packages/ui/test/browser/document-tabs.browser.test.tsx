// The window's document tabs: one Tab stop that the arrow keys move along, Enter shows the focused
// tab, Delete closes it, the close buttons and the strip's menu reach every tab, dragging a tab
// onto another reorders them, and a drag that ends anywhere else is handed to the shell. A tab whose
// file is missing carries a named warning mark, and the menu says "Not found" or each tab's page;
// the restore notice counts what came back and shows a missing file.

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

  it("move a tab one step right when dropped on its right-hand neighbour", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const from = page.getByRole("tab", { name: "report.pdf" }).element();
    const onto = page.getByRole("tab", { name: "notes.pdf" }).element();
    const data = new DataTransfer();
    from.dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: data }));
    onto.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: data }));
    onto.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: data }));
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

  it("reorder when a tab is dropped on another", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const from = page.getByRole("tab", { name: "minutes.pdf" }).element();
    const onto = page.getByRole("tab", { name: "report.pdf" }).element();
    const data = new DataTransfer();
    from.dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: data }));
    onto.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: data }));
    onto.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: data }));
    await expect.poll(() => log).toEqual(["drag c", "reorder cab"]);
  });

  it("hand a drag that ends off the strip to the shell, inside the page or out of it", async () => {
    const log: string[] = [];
    renderIn(<Strip log={log} />);
    const from = page.getByRole("tab", { name: "notes.pdf" }).element();
    const data = new DataTransfer();
    from.dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: data }));
    document.body.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: data }));
    from.dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: data }));
    // No dragover for a while: the pointer has left the page.
    from.dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: data }));
    await new Promise((resolve) => setTimeout(resolve, 400));
    from.dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: data }));
    await expect.poll(() => log).toEqual(["drag b", "out b inside", "drag b", "out b outside"]);
  });

  it("keep a drag that dropped on the strip to itself, and take no other strip's tab", async () => {
    const log: string[] = [];
    renderIn(
      <>
        <Strip log={log} />
        <Strip log={[]} />
      </>,
    );
    const mine = page.getByRole("tab", { name: "report.pdf" }).elements()[0];
    const onto = page.getByRole("tab", { name: "notes.pdf" }).elements()[1];
    if (mine === undefined || onto === undefined) throw new Error("two strips expected");
    const data = new DataTransfer();
    mine.dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: data }));
    mine.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: data }));
    mine.dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: data }));
    // The other strip refuses this one's tab: its dragover isn't cancelled, so no drop happens there.
    const foreign = new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: data });
    onto.dispatchEvent(foreign);
    expect(foreign.defaultPrevented).toBe(false);
    expect(log).toEqual(["drag a"]);
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
    await expect.element(page.getByRole("tab", { name: "Not found gone.pdf" })).toBeVisible();
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

  it("offers no missing file to show when every file came back", async () => {
    renderIn(<RestoreNotice tabs={1} missing={0} onShowMissing={() => undefined} onDismiss={() => undefined} />);
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
