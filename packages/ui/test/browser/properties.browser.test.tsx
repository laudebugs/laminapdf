// The editable properties dialog (O04) and the capability page (O05) in a browser: Apply sends
// only what changed and closes, Escape discards, Remove metadata… marks a value that Keep takes
// back, the notices say what the file's standard and its earlier versions mean, a failed Apply
// keeps the form; the page filters by search, group and columns and keeps the filters.

import { afterEach, describe, expect, it, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { type DocumentMetadataData, type MetadataChange, PropertiesDialog } from "../../src/dialogs.ts";
import type { CapabilityMatrix } from "../../src/help/capabilities.ts";
import { CapabilityPage } from "../../src/help/capability-page.tsx";
import { PlatformProvider } from "../../src/index.ts";
import { fakePlatform } from "../fakes.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(cleanup);

const META: DocumentMetadataData = {
  title: "Q3 Board Report",
  authors: ["Ada Lovelace"],
  creator: "Report template 2.3",
  disagreements: [],
  packet: "parsed",
};

/** The dialog over `metadata`, recording each Apply and close. */
function dialog(metadata: DocumentMetadataData, options: { rewritesWhole?: boolean; fail?: boolean } = {}) {
  const applied: MetadataChange[] = [];
  const onClose = vi.fn();
  renderIn(
    <PropertiesDialog
      isOpen
      fileName="q3.pdf"
      state={{ status: "loading" }}
      context={{ pages: [] }}
      onCopy={() => undefined}
      onClose={onClose}
      editing={{
        metadata: { status: "ready", metadata },
        rewritesWhole: options.rewritesWhole ?? false,
        onApply: async (change) => {
          applied.push(change);
          if (options.fail === true) throw new Error("the worker refused");
        },
      }}
    />,
  );
  return { applied, onClose };
}

const button = (name: string) => page.getByRole("button", { name, exact: true });

describe("the editable properties dialog", () => {
  it("applies only the changed field, as one change, then closes", async () => {
    const { applied, onClose } = dialog(META);
    await expect.element(button("Apply")).toBeDisabled();
    await userEvent.fill(page.getByLabelText("Title"), "Quarterly figures");
    await userEvent.click(button("Apply"));
    await vi.waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(applied).toEqual([{ patch: { title: "Quarterly figures" }, remove: [] }]);
  });

  it("discards the form on Escape", async () => {
    const { applied, onClose } = dialog(META);
    await userEvent.fill(page.getByLabelText("Title"), "Changed");
    await userEvent.keyboard("{Escape}");
    await vi.waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(applied).toEqual([]);
  });

  it("marks a value for removal, says earlier versions keep it, and Keep takes it back", async () => {
    dialog(META);
    await userEvent.click(button("Remove metadata…"));
    await userEvent.click(page.getByText("Report template 2.3"));
    await userEvent.click(button("Mark for removal"));
    await expect.element(page.getByText("Will be removed")).toBeVisible();
    await expect.element(page.getByText("Removed values stay in earlier versions.")).toBeVisible();
    await userEvent.click(button("Keep Creator"));
    await expect.element(page.getByLabelText("Creator")).toHaveValue("Report template 2.3");
    await expect.element(button("Apply")).toBeDisabled();
  });

  it("says nothing about earlier versions when the save rewrites the file", async () => {
    dialog(META, { rewritesWhole: true });
    await userEvent.fill(page.getByLabelText("Title"), "");
    await expect.element(button("Apply")).toBeEnabled();
    expect(page.getByText("Removed values stay in earlier versions.").elements()).toEqual([]);
  });

  it("names a PDF/A file's level and the fields where Info and XMP disagree", async () => {
    dialog({ ...META, pdfa: "2B", disagreements: ["title"] });
    await expect.element(page.getByText("PDF/A-2b file.")).toBeVisible();
    await expect.element(page.getByText(/disagree on title/)).toBeVisible();
    // Apply writes the value shown to both, so it is on before any change.
    await expect.element(button("Apply")).toBeEnabled();
  });

  it("keeps the form and says so when Apply fails", async () => {
    const { onClose } = dialog(META, { fail: true });
    await userEvent.fill(page.getByLabelText("Title"), "Changed");
    await userEvent.click(button("Apply"));
    await expect.element(page.getByText("Lamina PDF couldn't change the properties.")).toBeVisible();
    await expect.element(page.getByLabelText("Title")).toHaveValue("Changed");
    expect(onClose).not.toHaveBeenCalled();
  });
});

const op = (id: string, name: string) => ({
  id,
  name,
  round: "E1",
  target: "round-trip" as const,
  surfaces: { core: "round-trip" as const, pdf: null, web: "read" as const, desktop: null },
});

const MATRIX: CapabilityMatrix = {
  version: 1,
  groups: [
    { id: "C", name: "Page organisation", ops: [op("C.01", "Rotate pages"), op("C.06", "Merge documents")] },
    { id: "J", name: "Metadata", ops: [op("J.01", "Title, author, subject, keywords")] },
  ],
};

describe("the capability page", () => {
  it("filters by group and columns, and keeps the filters in platform storage", async () => {
    const platform = fakePlatform();
    renderIn(
      <PlatformProvider platform={platform}>
        <CapabilityPage
          matrix={MATRIX}
          version="0.2.0"
          backLabel="Back to the reader"
          onBack={() => undefined}
          onHowTested={() => undefined}
        />
      </PlatformProvider>,
    );
    const caption = () => page.getByRole("table").element().querySelector("caption")?.textContent;
    expect(caption()).toBe("3 of 3 operations in 2 groups");
    await userEvent.selectOptions(page.getByLabelText("Group"), "J");
    await vi.waitFor(() => expect(caption()).toBe("1 of 3 operations in 1 group"));
    await userEvent.click(button("Web"));
    expect(page.getByRole("columnheader", { name: "Web" }).elements()).toEqual([]);
    expect(await platform.storage.get("capabilities.filter")).toEqual({
      query: "",
      group: "J",
      surfaces: ["core", "pdf", "desktop"],
    });
  });
});
