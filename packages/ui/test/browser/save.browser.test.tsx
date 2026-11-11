// Saving's parts in a browser (Paper O02): the strip under the top bar says nothing for a plain
// file and how a signed, rewritten or conflicting one saves, with the signatures behind Details;
// the confirmation before removing signatures names each one, keeps the focus on Cancel and says
// which way was chosen; and neither has a serious axe violation in the light or dark theme.

import axe from "axe-core";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userEvent } from "vitest/browser";
import { RemoveSignaturesDialog, type SavePlanSlice, SaveStrip } from "../../src/editing.ts";
import { cleanup, focused, renderIn } from "./render.tsx";

afterEach(cleanup);

const CERTIFICATION = { kind: "certification", field: "f7.0" };
const APPROVAL = { kind: "approval", field: "f9.0" };

function plan(constraint: SavePlanSlice["constraint"], wouldInvalidate = [CERTIFICATION]): SavePlanSlice {
  return { constraint, signatures: [CERTIFICATION], wouldInvalidate };
}

const strip = (host: HTMLElement) => host.querySelector<HTMLElement>(".paper-save-strip");

describe("SaveStrip", () => {
  it("says nothing for a plain file, or without a plan", () => {
    const host = renderIn(
      <>
        <SaveStrip plan={plan("free")} />
        <SaveStrip plan={undefined} />
      </>,
    );
    expect(strip(host)).toBeNull();
  });

  it("says a signed file's changes go after the signature, in a labelled region", () => {
    const host = renderIn(<SaveStrip plan={plan("appendOnly")} />);
    expect([strip(host)?.getAttribute("aria-label"), strip(host)?.textContent]).toEqual([
      "How this file saves",
      expect.stringContaining("Saving adds your changes after the signature"),
    ]);
  });

  it("names a conflict's certification, and lists the signatures behind Details", async () => {
    const host = renderIn(<SaveStrip plan={plan("conflict", [CERTIFICATION, APPROVAL])} />);
    expect(strip(host)?.textContent).toContain("Saving will invalidate the certification.");
    await userEvent.click(host.querySelector("button") as HTMLElement);
    expect([...host.querySelectorAll(".paper-save-strip-signatures li")].map((li) => li.textContent)).toEqual([
      "The certification (1 of 2)",
      "The signature (2 of 2)",
    ]);
  });

  it("says a rewrite drops earlier versions", () => {
    const host = renderIn(<SaveStrip plan={plan("requiresRewrite")} />);
    expect(strip(host)?.textContent).toContain("Earlier versions are dropped.");
  });

  it.each(["light", "dark"] as const)("has no serious axe violation in the %s theme", async (theme) => {
    const host = renderIn(<SaveStrip plan={plan("conflict")} />, { theme });
    const result = await axe.run(host);
    expect(result.violations.filter((v) => v.impact === "serious" || v.impact === "critical")).toEqual([]);
  });
});

describe("RemoveSignaturesDialog", () => {
  it("names each signature it removes, with the focus on Cancel", async () => {
    const onAnswer = vi.fn();
    renderIn(<RemoveSignaturesDialog signatures={[CERTIFICATION]} onAnswer={onAnswer} />);
    const dialog = await vi.waitFor(() => {
      const found = document.querySelector<HTMLElement>('[role="alertdialog"]');
      if (found === null) throw new Error("no dialog yet");
      return found;
    });
    expect([
      dialog.getAttribute("aria-label") ?? dialog.querySelector("h2")?.textContent,
      [...dialog.querySelectorAll("li")].map((li) => li.textContent),
      focused()?.textContent,
    ]).toEqual(["Remove the certification and save?", ["The certification"], "Cancel"]);
  });

  it("says which way was chosen", async () => {
    const onAnswer = vi.fn();
    renderIn(<RemoveSignaturesDialog signatures={[CERTIFICATION]} onAnswer={onAnswer} />);
    const remove = await vi.waitFor(() => {
      const found = [...document.querySelectorAll("button")].find((b) => b.textContent === "Remove signature and save");
      if (found === undefined) throw new Error("no button yet");
      return found;
    });
    await userEvent.click(remove);
    expect(onAnswer).toHaveBeenCalledWith(true);
  });

  it("asks nothing without signatures", () => {
    renderIn(<RemoveSignaturesDialog signatures={undefined} onAnswer={() => {}} />);
    expect(document.querySelector('[role="alertdialog"]')).toBeNull();
  });

  it.each(["light", "dark"] as const)("has no serious axe violation in the %s theme", async (theme) => {
    renderIn(<RemoveSignaturesDialog signatures={[CERTIFICATION]} onAnswer={() => {}} />, { theme });
    const dialog = await vi.waitFor(() => {
      const found = document.querySelector<HTMLElement>('[role="alertdialog"]');
      if (found === null) throw new Error("no dialog yet");
      return found;
    });
    const result = await axe.run(dialog);
    expect(result.violations.filter((v) => v.impact === "serious" || v.impact === "critical")).toEqual([]);
  });
});
