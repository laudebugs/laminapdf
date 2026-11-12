// The home screen's Recent list: Clear list has a home whatever the list holds, beside Continue
// reading when that is the only document, else in the Recent section's heading. An entry whose file
// is gone can't be opened, only removed.

import { afterEach, describe, expect, it } from "vitest";
import { page } from "vitest/browser";
import { RecentList } from "../../src/index.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(() => cleanup());

const REPORT = { id: "a", name: "report.pdf", size: 1000, lastOpened: 0, pages: 24 };
const NOTES = { id: "b", name: "notes.pdf", size: 500, lastOpened: 0, pages: 3 };

describe("Clear list", () => {
  it("sits beside Continue reading when that is the only document, and clears", async () => {
    let cleared = 0;
    renderIn(
      <RecentList
        entries={[REPORT]}
        continueReading={{ entry: REPORT, page: 11, pageCount: 24 }}
        onOpen={() => {}}
        onClear={() => cleared++}
      />,
    );
    await page.getByRole("heading", { name: "Continue reading" }).element();
    expect(page.getByRole("heading", { name: "Recent" }).query()).toBeNull();
    await page.getByRole("button", { name: "Clear list" }).click();
    expect(cleared).toBe(1);
  });

  it("appears once, in the Recent section, when other documents are listed", async () => {
    renderIn(
      <RecentList
        entries={[REPORT, NOTES]}
        continueReading={{ entry: REPORT, page: 11, pageCount: 24 }}
        onOpen={() => {}}
        onClear={() => {}}
      />,
    );
    await page.getByRole("heading", { name: "Recent" }).element();
    expect(page.getByRole("button", { name: "Clear list" }).elements()).toHaveLength(1);
  });
});

describe("an entry whose file is gone", () => {
  const GONE = { ...NOTES, missing: true, hasHandle: false };

  it("says the file wasn't found and isn't a button", async () => {
    renderIn(<RecentList entries={[REPORT, GONE]} onOpen={() => {}} onClear={() => {}} onRemove={() => {}} />);
    await page.getByText("This device · File not found").element();
    expect(page.getByRole("button", { name: /^notes\.pdf/ }).query()).toBeNull();
  });

  it("removes that entry when its Remove is pressed", async () => {
    const removed: string[] = [];
    renderIn(
      <RecentList
        entries={[REPORT, GONE]}
        onOpen={() => {}}
        onClear={() => {}}
        onRemove={(entry) => removed.push(entry.id)}
      />,
    );
    await page.getByRole("button", { name: "Remove notes.pdf from recent files" }).click();
    expect(removed).toEqual(["b"]);
  });

  it("offers no Remove when the app can't remove one entry", async () => {
    renderIn(<RecentList entries={[GONE]} onOpen={() => {}} onClear={() => {}} />);
    await page.getByText("This device · File not found").element();
    expect(page.getByRole("button", { name: /^Remove/ }).query()).toBeNull();
  });
});
