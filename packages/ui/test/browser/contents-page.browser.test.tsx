import { useState } from "react";
import { flushSync } from "react-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userEvent } from "vitest/browser";
import { type ContentsChoice, ContentsPageDialog, type ContentsPreviewEntry } from "../../src/editing.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(cleanup);

const ENTRIES: readonly ContentsPreviewEntry[] = [
  { level: 1, title: "Summary", label: "2" },
  { level: 2, title: "Backlog", label: "3" },
];

/** The dialog open over `preview`; returns what Create was called with. */
function open(preview: (choice: ContentsChoice) => Promise<readonly ContentsPreviewEntry[]>) {
  const created: ContentsChoice[] = [];
  renderIn(
    <ContentsPageDialog
      isOpen
      onOpenChange={() => {}}
      firstPage="1"
      preview={preview}
      onCreate={(choice) => created.push(choice)}
    />,
  );
  return created;
}

/** Opens or closes {@link Host}'s dialog. */
let setOpen: (open: boolean) => void = () => {};

/** The dialog, opened and closed as an app would, through {@link setOpen}. */
function Host() {
  const [open, set] = useState(true);
  setOpen = set;
  return (
    <ContentsPageDialog
      isOpen={open}
      onOpenChange={set}
      firstPage="1"
      preview={() => Promise.resolve(ENTRIES)}
      onCreate={() => {}}
    />
  );
}

const button = (name: string) =>
  [...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name);
/** The radio labelled `name`, by its visible label. */
const radio = (name: string) =>
  [...document.querySelectorAll<HTMLElement>("label")].find((l) => l.textContent === name) as HTMLElement;
const entries = () => [...document.querySelectorAll(".paper-contents-preview-entry")].map((li) => li.textContent);

describe("the contents page dialog", () => {
  it("previews each entry with the label its page will show", async () => {
    open(() => Promise.resolve(ENTRIES));
    await expect.poll(entries).toEqual(["Summary2", "Backlog3"]);
  });

  it("reads the preview again when the levels change, not when the title does", async () => {
    const preview = vi.fn((_: ContentsChoice) => Promise.resolve(ENTRIES));
    open(preview);
    await expect.poll(entries).toHaveLength(2);
    await userEvent.click(radio("Top level only"));
    await userEvent.type(document.querySelector<HTMLElement>("input[type='text']") as HTMLElement, " here");
    await expect.poll(() => preview.mock.calls.map(([c]) => c.levels)).toEqual([2, 1]);
  });

  it("creates the page from the choices", async () => {
    const created = open(() => Promise.resolve(ENTRIES));
    await expect.poll(entries).toHaveLength(2);
    await userEvent.click(radio("At the end"));
    await userEvent.click(button("Create contents page") as HTMLElement);
    expect(created).toEqual([{ insert: "end", levels: 2, title: "Contents", bookmark: true }]);
  });

  it("can't create a page for a file without bookmarks", async () => {
    open(() => Promise.resolve([]));
    await expect
      .poll(() => document.querySelector(".paper-contents-preview-note")?.textContent)
      .toMatch(/None of this file/);
    expect(button("Create contents page")?.disabled).toBe(true);
  });

  it("starts from the defaults each time it opens", async () => {
    renderIn(<Host />);
    await expect.poll(entries).toHaveLength(2);
    await userEvent.click(radio("At the end"));
    flushSync(() => setOpen(false));
    await expect.poll(() => document.querySelector("[role='dialog']")).toBeNull();
    flushSync(() => setOpen(true));
    await expect.poll(() => document.querySelector<HTMLInputElement>("input[value='start']")?.checked).toBe(true);
  });
});
