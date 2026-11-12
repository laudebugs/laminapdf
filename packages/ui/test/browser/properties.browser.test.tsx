// The editable properties dialog (O04) and the capability page (O05) in a browser: Apply sends
// only what changed and closes, Escape discards, Remove metadata… marks a value that Keep takes
// back, the notices say what the file's standard and its earlier versions mean, a failed Apply
// keeps the form; the page filters by search, group and columns and keeps the filters.

import { afterEach, describe, expect, it, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import {
  type DocumentMetadataData,
  type MetadataChange,
  PropertiesDialog,
  type XmpChange,
  type XmpPacketData,
} from "../../src/dialogs.ts";
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

/** The dialog over `metadata`, recording each Apply (its XMP changes in `xmp`) and close. */
function dialog(
  metadata: DocumentMetadataData,
  options: { rewritesWhole?: boolean; fail?: boolean; xmp?: XmpPacketData } = {},
) {
  const applied: MetadataChange[] = [];
  const xmp: XmpChange[][] = [];
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
        xmp: options.xmp ?? { status: "absent" },
        onApply: async (change, changes) => {
          if (change !== undefined) applied.push(change);
          if (changes.length > 0) xmp.push([...changes]);
          if (options.fail === true) throw new Error("the worker refused");
        },
      }}
    />,
  );
  return { applied, xmp, onClose };
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

const REVIEW = "https://example.com/ns/review/1.0/";

/** A packet with a title, the PDF/A identification and a custom property in attribute form. */
const PACKET = `<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/" xmlns:review="${REVIEW}" review:status="Draft">
<dc:title><rdf:Alt><rdf:li xml:lang="x-default">Q3 Board Report</rdf:li></rdf:Alt></dc:title>
<pdfaid:part>2</pdfaid:part>
<review:reviewers><rdf:Seq><rdf:li>Ada Lovelace</rdf:li></rdf:Seq></review:reviewers>
</rdf:Description></rdf:RDF></x:xmpmeta>`;

const tree = () => page.getByRole("treegrid", { name: "XMP properties" });
const item = (name: string) => tree().getByRole("row", { name: new RegExp(`^${name}`) });

describe("the XMP tab", () => {
  it("lists the schemas, edits a property's value and applies it", async () => {
    const { xmp, onClose } = dialog(META, { xmp: { status: "parsed", xml: PACKET } });
    await userEvent.click(page.getByRole("tab", { name: "XMP" }));
    await expect.element(tree()).toBeVisible();
    await userEvent.click(item("status"));
    await expect.element(page.getByLabelText("Value")).toHaveValue("Draft");
    await userEvent.fill(page.getByLabelText("Value"), "Approved");
    await expect.element(page.getByText("1 property changed")).toBeVisible();
    await userEvent.click(button("Apply"));
    await vi.waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(xmp).toEqual([
      [{ kind: "set", namespace: REVIEW, prefix: "review", name: "status", value: { type: "text", value: "Approved" } }],
    ]);
  });

  it("refuses a value that doesn't fit its type until it does", async () => {
    dialog(META, { xmp: { status: "parsed", xml: PACKET } });
    await userEvent.click(page.getByRole("tab", { name: "XMP" }));
    await userEvent.click(item("status"));
    await userEvent.selectOptions(page.getByLabelText("Type"), "integer");
    await expect.element(page.getByText("Enter a whole number, such as 42.")).toBeVisible();
    await expect.element(button("Apply")).toBeDisabled();
    await userEvent.fill(page.getByLabelText("Value"), "3");
    await expect.element(button("Apply")).toBeEnabled();
  });

  it("sends a mirrored property to the Description tab and keeps the identification", async () => {
    dialog(META, { xmp: { status: "parsed", xml: PACKET } });
    await userEvent.click(page.getByRole("tab", { name: "XMP" }));
    await userEvent.click(item("part"));
    await expect.element(page.getByText(/Kept by Lamina PDF/)).toBeVisible();
    await userEvent.click(item("title"));
    await userEvent.click(button("Edit on the Description tab"));
    await expect.element(page.getByLabelText("Title")).toHaveValue("Q3 Board Report");
  });

  it("removes a property, and Keep takes it back", async () => {
    dialog(META, { xmp: { status: "parsed", xml: PACKET } });
    await userEvent.click(page.getByRole("tab", { name: "XMP" }));
    await userEvent.click(item("reviewers"));
    await userEvent.click(button("Remove property"));
    await expect.element(page.getByText("Will be removed")).toBeVisible();
    await userEvent.click(button("Keep"));
    await expect.element(button("Apply")).toBeDisabled();
  });

  it("adds a property in a new namespace", async () => {
    const { xmp } = dialog(META, { xmp: { status: "absent" } });
    await userEvent.click(page.getByRole("tab", { name: "XMP" }));
    await expect.element(page.getByText(/has no XMP metadata yet/)).toBeVisible();
    await userEvent.click(button("Add property…"));
    await userEvent.fill(page.getByLabelText("Namespace URI"), REVIEW);
    await userEvent.fill(page.getByLabelText("Prefix"), "review");
    await userEvent.fill(page.getByLabelText("Name"), "round");
    await userEvent.selectOptions(page.getByLabelText("Type"), "integer");
    await userEvent.fill(page.getByLabelText("Value"), "2");
    await userEvent.click(button("Add"));
    await userEvent.click(button("Apply"));
    await vi.waitFor(() => expect(xmp).toHaveLength(1));
    expect(xmp[0]).toEqual([
      { kind: "set", namespace: REVIEW, prefix: "review", name: "round", value: { type: "integer", value: 2 } },
    ]);
  });

  it("checks raw XML as it is typed and replaces the packet once it parses", async () => {
    const { xmp } = dialog(META, { xmp: { status: "parsed", xml: PACKET } });
    await userEvent.click(page.getByRole("tab", { name: "XMP" }));
    await userEvent.click(button("Raw XML"));
    const raw = page.getByLabelText("XMP metadata");
    await userEvent.fill(raw, PACKET.replace("</rdf:Description>", ""));
    await expect.element(raw).toHaveAttribute("aria-invalid", "true");
    await expect.element(button("Apply")).toBeDisabled();
    const fixed = PACKET.replace('review:status="Draft"', 'review:status="Final"');
    await userEvent.fill(raw, fixed);
    await expect.element(button("Apply")).toBeEnabled();
    await userEvent.click(button("Apply"));
    await vi.waitFor(() => expect(xmp).toEqual([[{ kind: "replace", xml: fixed }]]));
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
