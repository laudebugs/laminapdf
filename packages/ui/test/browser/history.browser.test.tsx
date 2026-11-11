// The History rail tab and the edit commands in a browser: the list as a screen reader reads it,
// the keys that move through it, a row chosen by mouse or Enter, the rows drawn for 500 steps, and
// Undo's keys, which act on the document but leave a text field its own undo.

import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { userEvent } from "vitest/browser";
import { CommandRegistry, CommandsProvider, HistoryPanel, registerEditCommands } from "../../src/index.ts";
import { FakeEdits, fakePlatform } from "../fakes.ts";
import { all, cleanup, focused, renderIn } from "./render.tsx";

afterEach(cleanup);

const LABELS = ["Rotate page 5", "Rotate page 2", "Rotate page 5"];
const TIME = { formatTime: () => "16:02", openedAt: 0 };

function panel(edits = new FakeEdits(LABELS, 2)) {
  const host = renderIn(
    <div style={{ height: "400px", display: "flex", flexDirection: "column" }}>
      <HistoryPanel edits={edits} {...TIME} />
    </div>,
  );
  return { edits, host };
}

const rows = (host: HTMLElement) => all(host, ".paper-history-row");

describe("HistoryPanel", () => {
  it("lists the opened file then each step, the current one marked and the undone one struck", () => {
    const { host } = panel();
    expect(host.querySelector("h2")?.textContent).toBe("History");
    expect(host.querySelector(".paper-history-count")?.textContent).toBe("3 steps");
    expect(rows(host).map((r) => r.textContent)).toEqual([
      "Opened fileNo changes · 16:02",
      "Rotate page 5Page 1 · 16:02",
      "Rotate page 2Page 2 · 16:02",
      "Rotate page 5Page 3 · Undone · 16:02",
    ]);
    expect(rows(host).map((r) => r.getAttribute("aria-current"))).toEqual([null, null, "step", null]);
    expect(getComputedStyle(rows(host)[3]?.querySelector(".paper-history-label") as Element).textDecorationLine).toBe(
      "line-through",
    );
    // The list is an ordered list of buttons, each item saying where it sits in the whole list.
    expect(host.querySelector("ol")?.getAttribute("aria-label")).toBe("Edit history");
    expect(all(host, "li").map((li) => li.getAttribute("aria-posinset"))).toEqual(["1", "2", "3", "4"]);
  });

  it("goes to a step when a row is clicked", async () => {
    const { edits, host } = panel();
    await userEvent.click(rows(host)[0] as HTMLElement);
    expect(edits.went).toEqual([0]);
    await expect.poll(() => rows(host)[0]?.getAttribute("aria-current")).toBe("step");
  });

  it("takes one Tab stop, on the current row, and moves with the arrows, Home and End", async () => {
    const { edits, host } = panel();
    expect(rows(host).map((r) => r.tabIndex)).toEqual([-1, -1, 0, -1]);
    (rows(host)[2] as HTMLElement).focus();
    await userEvent.keyboard("{ArrowDown}");
    expect(focused().dataset.row).toBe("3");
    await userEvent.keyboard("{Home}");
    expect(focused().dataset.row).toBe("0");
    await userEvent.keyboard("{End}");
    expect(focused().dataset.row).toBe("3");
    await userEvent.keyboard("{ArrowUp}{Enter}");
    expect(edits.went).toEqual([2]);
  });

  it("draws only the rows in view for 500 steps, and reaches the last by End", async () => {
    const labels = Array.from({ length: 500 }, (_, i) => `Change ${i + 1}`);
    const { host } = panel(new FakeEdits(labels, 0));
    expect(rows(host).length).toBeLessThan(60);
    (rows(host)[0] as HTMLElement).focus();
    await userEvent.keyboard("{End}");
    await expect.poll(() => focused().textContent).toContain("Change 500");
    expect(all(host, "li").at(-1)?.getAttribute("aria-setsize")).toBe("501");
  });

  it("says the first row no longer is the file as opened once the oldest steps were dropped", () => {
    const edits = new FakeEdits(LABELS, 3);
    edits.set({ history: { ...edits.getState().history, dropped: 1 } });
    const { host } = panel(edits);
    expect(rows(host)[0]?.textContent).toContain("Earlier steps no longer kept");
  });

  it("keeps a partial undo's notice at the top, and marks the row", () => {
    const edits = new FakeEdits(LABELS, 2);
    const step = { id: 3, label: "Rotate page 5", pages: [2], at: 0, partial: true as const };
    edits.set({
      history: {
        ...edits.getState().history,
        steps: [...edits.getState().history.steps.slice(0, 2), step],
        position: 3,
      },
      notice: { step, way: "undo", outcome: "partial" },
    });
    const { host } = panel(edits);
    expect(host.querySelector('[role="status"]')?.textContent).toContain("Undid part of “Rotate page 5”");
    expect(rows(host)[3]?.textContent).toContain("Partly undone");
  });

  it.each(["light", "dark"] as const)("has no axe violations in the %s theme", async (theme) => {
    const host = renderIn(<HistoryPanel edits={new FakeEdits(LABELS, 2)} {...TIME} />, { theme });
    const result = await axe.run(host);
    expect(result.violations.filter((v) => v.impact === "serious" || v.impact === "critical")).toEqual([]);
  });
});

describe("Undo's keys", () => {
  function shell(edits: FakeEdits) {
    const registry = new CommandRegistry();
    registerEditCommands(registry);
    return renderIn(
      <CommandsProvider registry={registry} context={{ platform: fakePlatform(), edits }} keys="other">
        <input aria-label="Note" />
        <button type="button">Elsewhere</button>
      </CommandsProvider>,
    );
  }

  it("undo the document's last change and redo it with Ctrl+Y or Ctrl+Shift+Z", async () => {
    const edits = new FakeEdits(LABELS, 3);
    const host = shell(edits);
    (host.querySelector("button") as HTMLElement).focus();
    await userEvent.keyboard("{Control>}z{/Control}");
    await userEvent.keyboard("{Control>}y{/Control}");
    await userEvent.keyboard("{Control>}z{/Control}");
    await userEvent.keyboard("{Control>}{Shift>}z{/Shift}{/Control}");
    expect(edits.went).toEqual([2, 3, 2, 3]);
  });

  it("leave a text field its own undo", async () => {
    const edits = new FakeEdits(LABELS, 3);
    const host = shell(edits);
    (host.querySelector("input") as HTMLElement).focus();
    await userEvent.keyboard("ab{Control>}z{/Control}");
    expect(edits.went).toEqual([]);
  });
});
