// Export (O16) and its progress, and a new PDF from images (O07), in a browser: each kind's options
// and the choice it reports, the cards a file has nothing for, range errors, the Markdown notice,
// the progress text and its failed pages, the images' order, and axe on each dialog.

import axe from "axe-core";
import { afterEach, describe, expect, it, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import {
  type ExportChoice,
  type ExportCounts,
  ExportDialog,
  type ExportDialogProps,
  ExportProgressDialog,
  type ExportProgressDialogProps,
  exportProgressTitle,
  type ImagesToPdfChoice,
  ImagesToPdfDialog,
} from "../../src/dialogs.ts";
import { cleanup, renderIn, settled } from "./render.tsx";

afterEach(cleanup);

const COUNTS: ExportCounts = { pictures: 12, links: 3, bookmarks: 5, tagged: true };

/** Export over a 24-page file in view at page 3, recording each choice. */
function exportDialog(props: Partial<ExportDialogProps> = {}) {
  const chosen: ExportChoice[] = [];
  const onClose = vi.fn();
  renderIn(
    <ExportDialog
      isOpen
      pageCount={24}
      currentPage={2}
      pageSize={{ width: 612, height: 792 }}
      counts={COUNTS}
      onExport={(choice) => chosen.push(choice)}
      onClose={onClose}
      {...props}
    />,
  );
  return { chosen, onClose };
}

const button = (name: string) => page.getByRole("button", { name, exact: true });
const radio = (name: string) => page.getByRole("radio", { name, exact: true });

/** Clicks a radio or checkbox as a person does: on its label, since the drawing covers the hidden input. */
async function pick(input: ReturnType<typeof page.getByRole>): Promise<void> {
  await expect.element(input).toBeInTheDocument();
  const label = input.element().closest("label");
  if (label === null) throw new Error("no label holds the input");
  await userEvent.click(label);
}

const choose = (name: string) => pick(radio(name));

async function violations(): Promise<string[]> {
  await settled();
  const result = await axe.run(document.body, {
    runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
  });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}

describe("the export dialog", () => {
  it("exports pages as images: the range, format, resolution and comments", async () => {
    const { chosen } = exportDialog({ pages: [2, 3, 4, 5] });
    await expect.element(page.getByRole("dialog", { name: "Export" })).toBeVisible();
    await expect.element(page.getByLabelText("Pages to export")).toHaveValue("3–6");
    await expect.element(page.getByRole("option", { name: "150 dpi · 1,275 × 1,650 px" })).toBeInTheDocument();
    await choose("JPEG");
    await userEvent.selectOptions(page.getByLabelText("Resolution"), "300");
    await pick(page.getByRole("checkbox", { name: "Include comments and filled-in fields" }));
    await userEvent.click(button("Export 4 pages"));
    expect(chosen).toEqual([{ kind: "pages", pages: [2, 3, 4, 5], format: "jpeg", dpi: 300, annotations: false }]);
  });

  it("opens on the palette's kind and exports text from this page", async () => {
    const { chosen } = exportDialog({ kind: "text" });
    await expect.element(radio("Text")).toBeChecked();
    await choose("This page (3)");
    await choose("JSON with positions");
    await userEvent.click(button("Export text"));
    expect(chosen).toEqual([{ kind: "text", pages: [2], format: "json", tagged: true }]);
  });

  it("warns that Markdown from an untagged file guesses its structure", async () => {
    exportDialog({ kind: "text", counts: { ...COUNTS, tagged: false } });
    await choose("Markdown");
    await expect.element(page.getByText(/this file has no tags, so headings and lists are guessed/)).toBeVisible();
  });

  it("exports the pictures of every page, counted, with a minimum size", async () => {
    const { chosen } = exportDialog();
    await choose("Pictures in the file");
    await pick(page.getByRole("checkbox", { name: /Keep originals/ }));
    await userEvent.selectOptions(page.getByLabelText("Minimum size"), "64");
    await userEvent.click(button("Export 12 pictures"));
    expect(chosen).toEqual([{ kind: "pictures", pages: [...Array(24).keys()], keepOriginals: false, minimumSize: 64 }]);
  });

  it("exports links and bookmarks without asking for pages", async () => {
    const { chosen } = exportDialog({ kind: "links" });
    await expect.element(page.getByRole("radiogroup", { name: "Pages" })).not.toBeInTheDocument();
    await choose("CSV");
    await userEvent.click(button("Export links and bookmarks"));
    expect(chosen).toEqual([{ kind: "links", format: "csv" }]);
  });

  it("disables the kinds a file has none of", async () => {
    exportDialog({ counts: { pictures: 0, links: 0, bookmarks: 0, tagged: false } });
    await expect.element(radio("Pictures in the file")).toBeDisabled();
    await expect.element(radio("Links and bookmarks")).toBeDisabled();
    await expect.element(page.getByText("None in this file").first()).toBeVisible();
  });

  it("keeps the cards usable and busy while the file is read", async () => {
    exportDialog({ counts: undefined });
    await expect.element(radio("Pages as images")).toBeInTheDocument();
    expect(radio("Pages as images").element().closest("[aria-busy=true]")).not.toBeNull();
    await choose("Pictures in the file");
    await expect.element(button("Export pictures")).toBeEnabled();
  });

  it("offers uncounted pictures without a count", async () => {
    exportDialog({ counts: { ...COUNTS, pictures: undefined } });
    await choose("Pictures in the file");
    await expect.element(page.getByText("The original pictures, unchanged where possible")).toBeInTheDocument();
    await expect.element(button("Export pictures")).toBeEnabled();
  });

  it("keeps every card usable, and isn't busy, when nothing could be counted", async () => {
    exportDialog({ counts: { pictures: undefined, links: undefined, bookmarks: undefined, tagged: false } });
    await expect.element(radio("Links and bookmarks")).toBeEnabled();
    await expect.element(radio("Pictures in the file")).toBeEnabled();
    await expect.element(page.getByText("None in this file")).not.toBeInTheDocument();
    expect(radio("Pages as images").element().closest("[aria-busy]")).toBeNull();
  });

  it("offers links when only the bookmarks are uncounted", async () => {
    exportDialog({ counts: { ...COUNTS, links: 0, bookmarks: undefined } });
    await expect.element(radio("Links and bookmarks")).toBeEnabled();
  });

  it("shows a range's error under the field and exports nothing", async () => {
    const { chosen } = exportDialog();
    await choose("Pages");
    await userEvent.click(button("Export pages"));
    await expect.element(page.getByText("Enter the pages to export, for example 1–5, 11.")).toBeVisible();
    await userEvent.fill(page.getByLabelText("Pages to export"), "30");
    await userEvent.click(button("Export pages"));
    await expect.element(page.getByText("There's no page 30. This file has 24 pages.")).toBeVisible();
    await expect.element(page.getByLabelText("Pages to export")).toHaveFocus();
    expect(chosen).toEqual([]);
  });

  it("closes on Cancel", async () => {
    const { onClose } = exportDialog();
    await userEvent.click(button("Cancel"));
    expect(onClose).toHaveBeenCalled();
  });

  it("names its close button Close Export", async () => {
    exportDialog();
    await expect.element(button("Close Export")).toBeVisible();
  });

  it("passes axe", async () => {
    exportDialog({ kind: "text", counts: { pictures: 0, links: 0, bookmarks: 0, tagged: false } });
    await choose("Markdown");
    expect(await violations()).toEqual([]);
  });
});

/** Export's progress, 15 of 24 pages as PNG, recording Cancel. */
function progressDialog(props: Partial<ExportProgressDialogProps> = {}) {
  const onCancel = vi.fn();
  const title = exportProgressTitle({ kind: "pages", pages: [], format: "png", dpi: 150, annotations: true }, 24);
  renderIn(
    <ExportProgressDialog
      isOpen
      title={title}
      done={15}
      total={24}
      unit="pages"
      onCancel={onCancel}
      onClose={() => undefined}
      {...props}
    />,
  );
  return { onCancel };
}

describe("the export progress dialog", () => {
  it("is named for the work", async () => {
    progressDialog();
    await expect.element(page.getByRole("dialog", { name: "Exporting 24 pages as PNG" })).toBeVisible();
  });

  it("counts the pages done in its bar", async () => {
    progressDialog();
    await expect
      .element(page.getByRole("progressbar", { name: "15 of 24 pages" }))
      .toHaveAttribute("aria-valuenow", "62");
  });

  it("names the zip the pages are packed into", async () => {
    progressDialog({ destination: { zip: "Q3-board-report-pages.zip" } });
    await expect
      .element(
        page.getByText(
          "Pages are drawn on this device and packed into Q3-board-report-pages.zip. Nothing is uploaded.",
        ),
      )
      .toBeVisible();
  });

  it("names the zip the pictures are packed into", async () => {
    progressDialog({ unit: "pictures", destination: { zip: "Q3 pictures.zip" } });
    await expect
      .element(page.getByText("Pictures are read on this device and packed into Q3 pictures.zip. Nothing is uploaded."))
      .toBeVisible();
  });

  it("names the one file it saves", async () => {
    progressDialog({ destination: { file: "Q3 report.json" } });
    await expect
      .element(page.getByText("Made on this device and saved as Q3 report.json. Nothing is uploaded."))
      .toBeVisible();
  });

  it("names the folder it writes into", async () => {
    progressDialog({ destination: { folder: "Exports" } });
    await expect
      .element(
        page.getByText("Pages are drawn on this device and written into the folder Exports. Nothing is uploaded."),
      )
      .toBeVisible();
  });

  it("says only that nothing is uploaded when the destination isn't known", async () => {
    progressDialog();
    await expect.element(page.getByText("Everything is made on this device. Nothing is uploaded.")).toBeVisible();
  });

  it("keeps focus on Cancel export while it runs", async () => {
    progressDialog();
    await expect.element(button("Cancel export")).toHaveFocus();
  });

  it("cancels on Escape", async () => {
    const { onCancel } = progressDialog();
    await expect.element(button("Cancel export")).toHaveFocus();
    await userEvent.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalled();
  });

  it("passes axe while it runs", async () => {
    progressDialog({ destination: { zip: "Q3-board-report-pages.zip" } });
    await expect.element(button("Cancel export")).toBeVisible();
    expect(await violations()).toEqual([]);
  });

  it("titles each kind", () => {
    expect(exportProgressTitle({ kind: "text", pages: [], format: "text", tagged: true }, 3)).toBe(
      "Exporting text as plain text",
    );
    expect(exportProgressTitle({ kind: "pictures", pages: [], keepOriginals: true, minimumSize: 0 }, 1)).toBe(
      "Exporting the pictures of 1 page",
    );
    expect(exportProgressTitle({ kind: "links", format: "json" }, 0)).toBe("Exporting links and bookmarks");
  });

  it("lists the pages that couldn't be drawn and offers the rest", async () => {
    const onExportRest = vi.fn();
    renderIn(
      <ExportProgressDialog
        isOpen
        title="Exporting 24 pages as PNG"
        done={20}
        total={24}
        unit="pages"
        failed={[16]}
        onCancel={() => undefined}
        onExportRest={onExportRest}
        onClose={() => undefined}
      />,
    );
    await expect.element(page.getByText("Page 17 couldn’t be drawn")).toBeVisible();
    await userEvent.click(button("Export the rest"));
    expect(onExportRest).toHaveBeenCalled();
    expect(await violations()).toEqual([]);
  });
});

/** New PDF from images with three receipts, recording what it makes. */
function imagesDialog() {
  const made: ImagesToPdfChoice[] = [];
  renderIn(
    <ImagesToPdfDialog
      isOpen
      files={[{ name: "receipt-front.jpg" }, { name: "receipt-back.jpg" }, { name: "invoice.png" }]}
      onChooseFiles={() => undefined}
      onMake={(choice) => made.push(choice)}
      onClose={() => undefined}
    />,
  );
  return { made };
}

describe("a new PDF from images", () => {
  it("can't move the first image up", async () => {
    imagesDialog();
    await expect.element(button("Move receipt-front.jpg up")).toBeDisabled();
  });

  it("moves an image down with the keyboard", async () => {
    imagesDialog();
    button("Move receipt-front.jpg down").element().focus();
    await userEvent.keyboard("{Enter}");
    await expect.element(page.getByRole("listitem").first()).toHaveTextContent("receipt-back.jpg");
  });

  it("announces an image's new place", async () => {
    imagesDialog();
    button("Move receipt-back.jpg up").element().focus();
    await userEvent.keyboard("{Enter}");
    await expect.element(page.getByText("receipt-back.jpg, 1 of 3")).toBeInTheDocument();
  });

  it("keeps focus in the row when Move up reaches the top", async () => {
    imagesDialog();
    button("Move receipt-back.jpg up").element().focus();
    await userEvent.keyboard("{Enter}");
    await expect.element(button("Move receipt-back.jpg down")).toHaveFocus();
  });

  it("keeps focus in the row when Move down reaches the bottom", async () => {
    imagesDialog();
    button("Move receipt-back.jpg down").element().focus();
    await userEvent.keyboard("{Enter}");
    await expect.element(button("Move receipt-back.jpg up")).toHaveFocus();
  });

  it("makes the PDF in the order, size and margin chosen", async () => {
    const { made } = imagesDialog();
    button("Move receipt-front.jpg down").element().focus();
    await userEvent.keyboard("{Enter}");
    await userEvent.selectOptions(page.getByLabelText("Page size"), "a4");
    await userEvent.selectOptions(page.getByLabelText("Margin"), "medium");
    await userEvent.click(button("Make a PDF of 3 images"));
    expect(made).toEqual([{ order: [1, 0, 2], pageSize: "a4", margin: 36 }]);
  });

  it("passes axe", async () => {
    imagesDialog();
    await expect.element(button("Make a PDF of 3 images")).toBeVisible();
    expect(await violations()).toEqual([]);
  });

  it("can't make a PDF of no images", async () => {
    renderIn(
      <ImagesToPdfDialog
        isOpen
        files={[]}
        onChooseFiles={() => undefined}
        onMake={() => undefined}
        onClose={() => undefined}
      />,
    );
    await expect.element(button("Make a PDF of 0 images")).toBeDisabled();
    await expect.element(page.getByText("No images chosen yet.")).toBeVisible();
  });
});
