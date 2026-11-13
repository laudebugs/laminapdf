// The window's document tabs: one Tab stop that the arrow keys move along, Enter shows the focused
// tab, Delete closes it, the close buttons and the strip's menu reach every tab, and dragging a tab
// onto another reorders them.

import { useState } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { page, userEvent } from "vitest/browser";
import { type DocumentTab, DocumentTabs } from "../../src/index.ts";
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
    await expect.poll(() => log).toEqual(["reorder bac"]);
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
    await expect.poll(() => log).toEqual(["reorder cab"]);
  });
});
