import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { type MarkStart, PageMarksDialog } from "../../src/editing.ts";
import { MARKS_DIALOG } from "../states.tsx";
import { cleanup, renderIn } from "./render.tsx";

afterEach(cleanup);

const text = () => document.body.textContent ?? "";

/** The dialog with a button that asks for another tab while it is open, as the palette does. */
function Host({ canHide = true }: { canHide?: boolean }) {
  const [start, setStart] = useState<MarkStart>({ mode: "add", tab: "bates" });
  return (
    <>
      <button type="button" onClick={() => setStart({ mode: "add", tab: "watermark" })}>
        Ask for Watermark
      </button>
      <PageMarksDialog {...MARKS_DIALOG} canHide={canHide} start={start} />
    </>
  );
}

describe("the Page marks dialog", () => {
  it("offers Readers can hide it unless the file can't hold the layer (PDF/A-1)", async () => {
    renderIn(<PageMarksDialog {...MARKS_DIALOG} canHide start={{ mode: "add", tab: "watermark" }} />);
    await vi.waitFor(() => expect(text()).toContain("Readers can hide it"));
    cleanup();
    renderIn(<PageMarksDialog {...MARKS_DIALOG} canHide={false} start={{ mode: "add", tab: "watermark" }} />);
    await vi.waitFor(() => expect(text()).toContain("Add watermark"));
    expect(text()).not.toContain("Readers can hide it");
  });

  it("numbers the first and last of the pages it goes on, not the document's", async () => {
    renderIn(<Host />);
    await vi.waitFor(() => expect(text()).toContain("The first page gets 000001, the last 000024."));
    await page.getByText("A range", { exact: true }).click();
    await userEvent.fill(page.getByRole("textbox", { name: "Page range" }), "2–4");
    await vi.waitFor(() => expect(text()).toContain("The first page gets 000001, the last 000003."));
  });

  it("opens the tab asked for while it is open, from a fresh form", async () => {
    renderIn(<Host />);
    await vi.waitFor(() => expect(text()).toContain("Start number"));
    // The page behind a modal is inert, so ask as the palette does: from outside, in code.
    (document.querySelector("button") as HTMLButtonElement).click();
    await vi.waitFor(() => expect(text()).toContain("Use"));
    expect(text()).not.toContain("Start number");
  });
});
