// S08's report question: Send report focused first, Don't send and Send report as the answers,
// Always send carried into the answer and kept across View report and Back, Escape leaving the
// question open, the preview showing the exact JSON with what it leaves out, Copy report; and the
// status bar's notice after the document view restarted.

import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { page, userEvent } from "vitest/browser";
import { type CrashAnswer, CrashNotice, CrashReportDialog, type CrashReportDialogProps } from "../../src/index.ts";
import { cleanup, renderIn, settled } from "./render.tsx";

afterEach(() => cleanup());

const REPORT = JSON.stringify(
  {
    schema: 1,
    report_id: "3f1c2a9e-8b7d-4e6f-9a5b-1c2d3e4f5a6b",
    process: "engine",
    kind: "panic",
    reason: "index out of bounds: the len is 3 but the index is 7 (in <path>)",
    frames: [{ module: "<unknown>", offset: "0x7f00", function: null }],
  },
  null,
  2,
);

/** The dialog over `REPORT`, recording its answers and copies. */
function ask(props: Partial<CrashReportDialogProps> = {}) {
  const answers: CrashAnswer[] = [];
  const copies: string[] = [];
  renderIn(
    <CrashReportDialog
      process="main"
      report={REPORT}
      onAnswer={(answer) => answers.push(answer)}
      onCopy={() => copies.push("copy")}
      {...props}
    />,
  );
  return { answers, copies, dialog: page.getByRole("dialog", { name: "Lamina PDF quit unexpectedly" }) };
}

describe("CrashReportDialog", () => {
  it("asks after the app quit, described by what a report holds, with Send report focused", async () => {
    const { dialog } = ask();
    await expect.element(dialog).toBeVisible();
    await expect.element(dialog).toHaveAttribute("aria-modal", "true");
    await expect
      .element(dialog)
      .toHaveAccessibleDescription(
        "An anonymous report says what went wrong in Lamina PDF. It never holds your files, their names or what's in them.",
      );
    await expect.element(dialog.getByRole("button", { name: "Send report" })).toHaveFocus();
    await expect.element(dialog.getByRole("checkbox", { name: "Always send reports after a crash" })).not.toBeChecked();
  });

  it("says the document view restarted after an engine crash", async () => {
    ask({ process: "engine" });
    const dialog = page.getByRole("dialog", { name: "Lamina PDF ran into a problem" });
    await expect
      .element(dialog)
      .toHaveAccessibleDescription("The document view restarted, and you're back where you were.");
    await expect
      .element(
        dialog.getByText(
          "An anonymous report says what went wrong in Lamina PDF. It never holds your files, their names or what's in them.",
        ),
      )
      .toBeVisible();
  });

  it("answers Don't send without Always, even when it is checked", async () => {
    const { answers, dialog } = ask();
    await dialog.getByText("Always send reports after a crash").click();
    await dialog.getByRole("button", { name: "Don't send" }).click();
    expect(answers).toEqual([{ send: false, always: false }]);
  });

  it("answers Send report with the Always check box as it is", async () => {
    const { answers, dialog } = ask();
    await dialog.getByRole("button", { name: "Send report" }).click();
    await dialog.getByText("Always send reports after a crash").click();
    await dialog.getByRole("button", { name: "Send report" }).click();
    expect(answers).toEqual([
      { send: true, always: false },
      { send: true, always: true },
    ]);
  });

  it("stays open on Escape: it asks a required question", async () => {
    const { answers, dialog } = ask();
    await expect.element(dialog.getByRole("button", { name: "Send report" })).toHaveFocus();
    await userEvent.keyboard("{Escape}");
    await settled();
    await expect.element(dialog).toBeVisible();
    expect(answers).toEqual([]);
  });

  it("shows the exact report with what it leaves out, and Back keeps Always as it was", async () => {
    const { answers, dialog } = ask();
    await dialog.getByText("Always send reports after a crash").click();
    await dialog.getByRole("button", { name: "View report" }).click();
    const json = dialog.getByRole("region", { name: "Crash report" });
    await expect.element(json).toBeVisible();
    expect(json.element().textContent).toBe(REPORT);
    expect(json.element().tabIndex).toBe(0);
    for (const badge of ["No file names or paths", "No document content", "No anonymous update ID"]) {
      await expect.element(dialog.getByRole("listitem").getByText(badge, { exact: true })).toBeVisible();
    }
    await expect.element(dialog.getByText("Sent to Lamina PDF's own server and deleted after 30 days.")).toBeVisible();
    await expect
      .element(
        dialog.getByText(
          "The report ID is random and made for this report only; it is not stored anywhere else on this computer.",
        ),
      )
      .toBeVisible();
    await expect.element(dialog.getByRole("button", { name: "Back" })).toHaveFocus();
    await dialog.getByRole("button", { name: "Back" }).click();
    await expect.element(dialog.getByRole("button", { name: "View report" })).toHaveFocus();
    await expect.element(dialog.getByRole("checkbox", { name: "Always send reports after a crash" })).toBeChecked();
    await dialog.getByRole("button", { name: "Send report" }).click();
    expect(answers).toEqual([{ send: true, always: true }]);
  });

  it("answers from the preview too, and copies the report", async () => {
    const { answers, copies, dialog } = ask({ initialView: "preview" });
    await dialog.getByRole("button", { name: "Copy report" }).click();
    expect(copies).toEqual(["copy"]);
    await dialog.getByRole("button", { name: "Send report" }).click();
    await dialog.getByRole("button", { name: "Don't send" }).click();
    expect(answers).toEqual([
      { send: true, always: false },
      { send: false, always: false },
    ]);
  });

  it("offers no Copy report without onCopy", async () => {
    renderIn(<CrashReportDialog process="webview" report={REPORT} onAnswer={() => undefined} initialView="preview" />);
    await expect.element(page.getByRole("region", { name: "Crash report" })).toBeVisible();
    expect(page.getByRole("button", { name: "Copy report" }).elements()).toEqual([]);
  });

  it.each(["question", "preview"] as const)("passes axe in the %s view", async (initialView) => {
    ask({ initialView });
    await settled();
    const results = await axe.run(document.body, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
    });
    expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)).toEqual([]);
  });
});

describe("CrashNotice", () => {
  it("asks in a status once mounted, and each button answers", async () => {
    const log: string[] = [];
    renderIn(
      <CrashNotice
        onSend={() => log.push("send")}
        onViewReport={() => log.push("view")}
        onDismiss={() => log.push("dismiss")}
      />,
    );
    await expect
      .element(page.getByRole("status"))
      .toHaveTextContent("The document view restarted after a problem. Send an anonymous report?");
    await page.getByRole("button", { name: "View report" }).click();
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await page.getByRole("button", { name: "Don't send" }).click();
    expect(log).toEqual(["view", "send", "dismiss"]);
  });

  it("says the window hit a problem after a page error, which restarted nothing", async () => {
    renderIn(<CrashNotice process="webview" onSend={() => {}} onViewReport={() => {}} onDismiss={() => {}} />);
    await expect
      .element(page.getByRole("status"))
      .toHaveTextContent("Lamina PDF ran into a problem in this window. Send an anonymous report?");
  });
});
