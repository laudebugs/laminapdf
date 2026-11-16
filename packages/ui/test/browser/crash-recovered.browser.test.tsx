// S08's tab choice after the app quit unexpectedly: every tab checked but the one in front, which
// is badged; Reopen N tabs following the check boxes; Start without tabs; the report part under
// the tabs, with Always only after Send; and the report preview's answers going back to the tabs.

import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { page, userEvent } from "vitest/browser";
import { CrashRecoveredDialog, type RecoveredChoice, type RecoveredTab } from "../../src/index.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(() => cleanup());

const TABS: RecoveredTab[] = [
  { token: "t1", name: "invoice.pdf", page: 2, front: false },
  { token: "t2", name: "big-scan.pdf", page: 40, front: true },
  { token: "t3", name: "notes.pdf", front: false },
];

const REPORT = JSON.stringify({ schema: 1, process: "main", kind: "panic" }, null, 2);

/** The dialog over `tabs`, with the report part when `withReport`, recording its choices. */
function choose(withReport = false, tabs: RecoveredTab[] = TABS) {
  const choices: RecoveredChoice[] = [];
  renderIn(
    <CrashRecoveredDialog
      tabs={tabs}
      {...(withReport ? { report: { json: REPORT } } : {})}
      onChoose={(choice) => choices.push(choice)}
    />,
  );
  return { choices, dialog: page.getByRole("dialog", { name: "Lamina PDF quit unexpectedly" }) };
}

describe("CrashRecoveredDialog", () => {
  it("checks every tab but the one in front when the app quit", async () => {
    const { dialog } = choose();
    await expect.element(dialog.getByRole("checkbox", { name: /invoice\.pdf/ })).toBeChecked();
    await expect.element(dialog.getByRole("checkbox", { name: /big-scan\.pdf/ })).not.toBeChecked();
  });

  it("badges the tab in front and names each tab's page", async () => {
    const { dialog } = choose();
    await expect.element(dialog.getByText("Open when Lamina PDF quit")).toBeVisible();
    await expect.element(dialog.getByText("page 40")).toBeVisible();
  });

  it("counts the checked tabs on Reopen and reopens those", async () => {
    const { choices, dialog } = choose();
    await dialog.getByText("notes.pdf").click();
    await dialog.getByRole("button", { name: "Reopen 1 tab" }).click();
    expect(choices).toEqual([{ tokens: ["t1"], send: false, always: false }]);
  });

  it("starts without tabs", async () => {
    const { choices, dialog } = choose();
    await dialog.getByRole("button", { name: "Start without tabs" }).click();
    expect(choices).toEqual([{ tokens: [], send: false, always: false }]);
  });

  it("focuses Reopen first", async () => {
    const { dialog } = choose();
    await expect.element(dialog.getByRole("button", { name: "Reopen 2 tabs" })).toHaveFocus();
  });

  it("focuses Start without tabs first when the only tab is the one in front", async () => {
    const { dialog } = choose(false, [{ token: "t1", name: "big-scan.pdf", front: true }]);
    await expect.element(dialog.getByRole("button", { name: "Start without tabs" })).toHaveFocus();
  });

  it("stays open on Escape: it asks a required question", async () => {
    const { choices, dialog } = choose();
    await userEvent.keyboard("{Escape}");
    await expect.element(dialog).toBeVisible();
    expect(choices).toEqual([]);
  });

  it("says why Always waits for Send, and leaves it unchecked when pressed", async () => {
    const { dialog } = choose(true);
    const always = dialog.getByRole("checkbox", { name: "Always send reports after a crash" });
    await expect.element(always).toHaveAccessibleDescription("Available after you choose to send this report");
    await dialog.getByText("Always send reports after a crash").click();
    await expect.element(always).not.toBeChecked();
  });

  it("carries the report's answer into the choice", async () => {
    const { choices, dialog } = choose(true);
    await dialog.getByText("Send an anonymous crash report").click();
    await dialog.getByText("Always send reports after a crash").click();
    await dialog.getByRole("button", { name: "Reopen 2 tabs" }).click();
    expect(choices).toEqual([{ tokens: ["t1", "t3"], send: true, always: true }]);
  });

  it("returns to the tabs with Send checked after Send report in the preview", async () => {
    const { dialog } = choose(true);
    await dialog.getByRole("button", { name: "View report" }).click();
    await dialog.getByRole("button", { name: "Send report" }).click();
    await expect.element(dialog.getByRole("checkbox", { name: "Send an anonymous crash report" })).toBeChecked();
  });

  it("has no axe violation", async () => {
    const { dialog } = choose(true);
    await expect.element(dialog).toBeVisible();
    const results = await axe.run(dialog.element());
    expect(results.violations.map((violation) => violation.id)).toEqual([]);
  });
});
