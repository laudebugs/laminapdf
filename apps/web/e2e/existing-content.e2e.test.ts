// Select over what a file already had (Paper A05) and A07's strip, in the built reader, in this run's
// browser, with the real mouse and keyboard: a picture, a rectangle and a line of text of a
// hand-written page, each picked, moved from the inspector, resized by a handle, restacked and
// deleted, then saved and read back by pdf.js; and a PDF/UA file whose picture is replaced, which
// breaks its claim: the strip and the badge say so, alternative text puts it right, and Check before
// saving offers Save without the claim, which takes the claim out of the saved file.

import { readFileSync, writeFileSync } from "node:fs";
import { deflateSync } from "node:zlib";
import { getDocument, OPS } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const PAGE = '.lamina-page[data-page-index="0"]';
const WIDTH = 612;
const HEIGHT = 792;

/** A PDF of `objects` (the text of objects 1, 2, …), with its table of offsets. */
function pdf(objects: readonly string[], root = 1): Uint8Array {
  let out = "%PDF-1.7\n";
  const offsets: number[] = [];
  objects.forEach((body, i) => {
    offsets.push(out.length);
    out += `${i + 1} 0 obj\n${body}\nendobj\n`;
  });
  const xref = out.length;
  out += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const offset of offsets) out += `${String(offset).padStart(10, "0")} 00000 n \n`;
  out += `trailer\n<< /Size ${objects.length + 1} /Root ${root} 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(out, "latin1");
}

/** A stream object. */
const stream = (dict: string, data: string) => `<< ${dict} /Length ${data.length} >>\nstream\n${data}\nendstream`;

/** A 2 by 2 pixel RGB image, uncompressed. */
const IMAGE = "/Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8";
const PIXELS = "\xff\x00\x00\x00\xff\x00\x00\x00\xff\xff\xff\x00";

/** A page with a picture, a red rectangle and a line of text, in this order. */
function plain(): Uint8Array {
  const content = [
    "q 150 0 0 100 350 600 cm /Im0 Do Q",
    "1 0 0 rg 100 600 200 100 re f",
    "0 g BT /F1 18 Tf 100 400 Td (Hello existing) Tj ET",
  ].join("\n");
  return pdf([
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${WIDTH} ${HEIGHT}] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> /Font << /F1 6 0 R >> >> >>`,
    stream("", content),
    stream(IMAGE, PIXELS),
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ]);
}

/** A tagged PDF/UA-1 page with one picture in a Figure that has alternative text. */
function accessible(): Uint8Array {
  const xmp =
    '<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?><x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" xmlns:dc="http://purl.org/dc/elements/1.1/"><pdfuaid:part>1</pdfuaid:part><dc:title><rdf:Alt><rdf:li xml:lang="x-default">Figures</rdf:li></rdf:Alt></dc:title></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end="w"?>';
  return pdf([
    "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R /MarkInfo << /Marked true >> /Metadata 8 0 R /Lang (en-US) /ViewerPreferences << /DisplayDocTitle true >> >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${WIDTH} ${HEIGHT}] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> /StructParents 0 /Tabs /S >>`,
    stream("", "/Figure << /MCID 0 >> BDC q 150 0 0 100 100 600 cm /Im0 Do Q EMC"),
    stream(IMAGE, PIXELS),
    "<< /Type /StructTreeRoot /K 7 0 R /ParentTree 9 0 R >>",
    "<< /Type /StructElem /S /Figure /P 6 0 R /Pg 3 0 R /K 0 /Alt (Four coloured squares) >>",
    stream("/Type /Metadata /Subtype /XML", xmp),
    "<< /Nums [0 [7 0 R]] >>",
  ]);
}

/** A `width` by `height` PNG of one colour: a replacement picture. */
function png(width: number, height: number): Buffer {
  const crc = (bytes: Buffer): number => {
    let c = 0xffffffff;
    for (const b of bytes) {
      c ^= b;
      for (let k = 0; k < 8; k++) c = c & 1 ? (c >>> 1) ^ 0xedb88320 : c >>> 1;
    }
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type: string, data: Buffer): Buffer => {
    const body = Buffer.concat([Buffer.from(type), data]);
    const out = Buffer.alloc(body.length + 8);
    out.writeUInt32BE(data.length, 0);
    body.copy(out, 4);
    out.writeUInt32BE(crc(body), body.length + 4);
    return out;
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.set([8, 2, 0, 0, 0], 8);
  const row = Buffer.concat([Buffer.from([0]), Buffer.alloc(width * 3, 0x55)]);
  const raw = Buffer.concat(Array.from({ length: height }, () => row));
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** Writes `bytes` where the file chooser can take them. */
function written(bytes: Uint8Array | Buffer, name = "existing"): string {
  const path = `${process.env.TMPDIR ?? "/tmp"}/${name}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  writeFileSync(path, bytes);
  return path;
}

/** The reader with the PDF `bytes` open in Edit mode, no file pickers of the browser's own. */
async function edit(bytes: Uint8Array): Promise<Watched> {
  const watched = await load(
    browser,
    origin,
    "/",
    {
      script: () => {
        Reflect.deleteProperty(window, "showOpenFilePicker");
        Reflect.deleteProperty(window, "showSaveFilePicker");
      },
    } as never,
    { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 1200 }, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(written(bytes, "existing.pdf"));
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  await page.getByRole("toolbar", { name: "Edit tools" }).waitFor();
  return watched;
}

const inspectorOf = (page: Page) => page.getByRole("complementary", { name: "Page content" });

/** The viewport position of (`x`, `y`) points from page 1's top-left. */
async function at(page: Page, x: number, y: number): Promise<{ x: number; y: number }> {
  const box = await page.locator(PAGE).boundingBox();
  if (box === null) throw new Error("page 1 isn't shown");
  const scale = box.width / WIDTH;
  return { x: box.x + x * scale, y: box.y + y * scale };
}

/** A click on page 1 at (`x`, `y`) points. */
async function click(page: Page, x: number, y: number): Promise<void> {
  const point = await at(page, x, y);
  await page.mouse.move(point.x, point.y, { steps: 3 });
  await page.mouse.click(point.x, point.y);
}

/** Clicks (`x`, `y`) points until what is there is picked: the page's objects are read in the background, and a press before that picks nothing. */
async function clickWhenRead(page: Page, x: number, y: number): Promise<void> {
  const heading = inspectorOf(page).getByRole("heading").first();
  await expect
    .poll(
      async () => {
        await click(page, x, y);
        return heading.isVisible();
      },
      { timeout: 30_000 },
    )
    .toBe(true);
}

/** The count of unsaved changes the top bar says, once it says `count`. */
async function changes(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent(), { timeout: 30_000 }).toBe(text);
}

/** Saves (a download) and returns the file. */
async function save(page: Page): Promise<Uint8Array> {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return new Uint8Array(readFileSync(await (await download).path()));
}

/** Types `value` into the inspector's field `name` and commits it with Enter. */
async function setField(page: Page, name: string, value: number): Promise<void> {
  const field = inspectorOf(page).getByRole("textbox", { name, exact: true });
  await field.fill(String(value));
  await field.press("Enter");
}

/** The inspector's X, Y, W and H fields as numbers. */
async function geometry(page: Page): Promise<{ x: number; y: number; w: number; h: number }> {
  const field = async (name: string) =>
    Number(await inspectorOf(page).getByRole("textbox", { name, exact: true }).inputValue());
  return { x: await field("X"), y: await field("Y"), w: await field("W"), h: await field("H") };
}

type Six = [number, number, number, number, number, number];

/** The matrix the picture is drawn under: the page's `cm` operators, kept and restored as `q` and `Q` say. */
function pictureMatrix(fnArray: readonly number[], argsArray: readonly number[][]): number[] | undefined {
  let ctm: Six = [1, 0, 0, 1, 0, 0];
  const stack: Six[] = [];
  let picture: number[] | undefined;
  fnArray.forEach((fn, i) => {
    if (fn === OPS.save) stack.push(ctm);
    else if (fn === OPS.restore) ctm = stack.pop() ?? ctm;
    else if (fn === OPS.transform) ctm = multiplied(argsArray[i] as Six, ctm);
    else if (fn === OPS.paintImageXObject) picture = ctm;
  });
  return picture;
}

/** `m` applied first, then `ctm`: the new current matrix. */
function multiplied([a, b, c, d, e, f]: Six, [A, B, C, D, E, F]: Six): Six {
  return [a * A + b * C, a * B + b * D, c * A + d * C, c * B + d * D, e * A + f * C + E, e * B + f * D + F];
}

/** What pdf.js reads of page 1 of `bytes`: its text, how many pictures and paths it draws, and where the picture is drawn. */
async function read(bytes: Uint8Array) {
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  const doc = await task.promise;
  try {
    const page = await doc.getPage(1);
    const text = (await page.getTextContent()).items.map((i) => ("str" in i ? i.str : "")).join("");
    const { fnArray, argsArray } = await page.getOperatorList();
    const picture = pictureMatrix(fnArray, argsArray as number[][]);
    const count = (op: number) => fnArray.filter((o) => o === op).length;
    const [first] = (await page.getTextContent()).items;
    const tilt = first !== undefined && "transform" in first ? ((first.transform as number[])[1] ?? 0) : 0;
    return {
      text,
      images: count(OPS.paintImageXObject),
      paths: count(OPS.constructPath),
      clips: count(OPS.clip),
      picture,
      tilt,
    };
  } finally {
    await task.destroy();
  }
}

describe(`Select over what a file already had in ${BROWSER}`, () => {
  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap A.07 web round-trip] [cap B.01 web round-trip] [cap B.02 web round-trip] [cap B.08 web round-trip] [cap B.12 web round-trip] picks, moves, resizes, restacks and deletes the objects of a page, saves, and finds the change", async () => {
    const watched = await edit(plain());
    const { page } = watched;
    const inspector = inspectorOf(page);

    // The picture: a click picks it; the inspector names it and says where it came from.
    await click(page, 420, 140);
    await inspector.getByRole("heading", { name: "Image" }).waitFor();
    expect(await inspector.getByText("Part of the original page").textContent()).toContain("Page 1");
    expect(await geometry(page)).toMatchObject({ x: 350, y: 92, w: 150, h: 100 });
    expect(await serious(page)).toEqual([]);

    // Moved from the inspector, which is what a drag does, typed.
    await setField(page, "X", 400);
    await changes(page, 1);
    expect((await geometry(page)).x).toBe(400);

    // Resized by the corner handle: the width and height change in the inspector.
    const corner = page.locator('[data-content-handle="se"]');
    await corner.waitFor();
    const box = await corner.boundingBox();
    if (box === null) throw new Error("no handle");
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 40, box.y + box.height / 2 + 30, { steps: 5 });
    await page.mouse.up();
    await changes(page, 2);
    expect((await geometry(page)).w).toBeGreaterThan(150);

    // The rectangle: picked, then deleted with the key.
    await click(page, 200, 140);
    await inspector.getByRole("heading", { name: "Shape" }).waitFor();
    await page.keyboard.press("Delete");
    await changes(page, 3);
    await inspector.waitFor({ state: "hidden" });

    // The line of text: picked as a block, sent to the back from the context menu.
    await click(page, 150, 385);
    await inspector.getByRole("heading", { name: "Text block" }).waitFor();
    const point = await at(page, 150, 385);
    await page.mouse.click(point.x, point.y, { button: "right" });
    await page.getByRole("menuitem", { name: "Send to back" }).click();
    await changes(page, 4);
    expect(await serious(page)).toEqual([]);

    // The block is still picked: turned from the inspector's Rotate field.
    await setField(page, "Rotate", 15);
    await changes(page, 5);

    // The picture is cut in from its left edge by the crop's inset field.
    await click(page, 420, 140);
    await inspector.getByRole("heading", { name: "Image" }).waitFor();
    const crop = inspector.getByRole("textbox", { name: "Left", exact: true });
    await crop.fill("10");
    await crop.press("Enter");
    await changes(page, 6);
    const picture = await geometry(page);

    // Undo takes the last step back; Redo makes it again.
    await page.locator(".paper-top-bar").getByRole("button", { name: /^Undo/ }).click();
    await changes(page, 5);
    await page.locator(".paper-top-bar").getByRole("button", { name: /^Redo/ }).click();
    await changes(page, 6);

    const bytes = await save(page);
    const saved = await read(bytes);
    expect(saved.text).toContain("Hello existing");
    expect(saved.images).toBe(1);
    // The rectangle is gone: the only path left is the crop's clip.
    expect(saved.paths).toBe(saved.clips);
    // The picture was moved right by 50 points and made larger: pdf.js draws it from x = 400.
    expect(saved.picture?.[4]).toBeCloseTo(400, 0);
    expect(saved.picture?.[0] ?? 0).toBeGreaterThan(150);
    // The crop is a clip, and the line of text was turned.
    expect(saved.clips).toBeGreaterThan(0);
    expect(Math.abs(saved.tilt)).toBeGreaterThan(0.1);

    // Opened again, the picture is found where it was left.
    const again = await edit(bytes);
    const centre = { x: picture.x + picture.w / 2, y: picture.y + picture.h / 2 };
    await clickWhenRead(again.page, centre.x, centre.y);
    await inspectorOf(again.page).getByRole("heading", { name: "Image" }).waitFor();
    await clean(again);
    await clean(watched);
  });

  it("picks several objects with a box and with Shift, and moves them as one step", async () => {
    const watched = await edit(plain());
    const { page } = watched;
    const inspector = inspectorOf(page);
    // The hover outline shows once the page's objects are read.
    const over = await at(page, 420, 140);
    await expect
      .poll(
        async () => {
          await page.mouse.move(over.x + Math.random() * 4, over.y, { steps: 2 });
          return page.locator(".lamina-content-hover").count();
        },
        { timeout: 30_000 },
      )
      .toBe(1);
    // A box drawn around the picture and the rectangle takes both in.
    const from = await at(page, 60, 60);
    const to = await at(page, 540, 230);
    await page.mouse.move(from.x, from.y);
    await page.mouse.down();
    await page.mouse.move(to.x, to.y, { steps: 6 });
    await page.mouse.up();
    await inspector.getByRole("heading", { name: "2 objects" }).waitFor();
    await setField(page, "Y", 120);
    await changes(page, 1);
    // Shift takes one back out.
    await click(page, 200, 190);
    await inspector.getByRole("heading", { name: "Shape" }).waitFor();
    await page.keyboard.down("Shift");
    await click(page, 420, 190);
    await page.keyboard.up("Shift");
    await inspector.getByRole("heading", { name: "2 objects" }).waitFor();
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });

  it("steps through the objects from the keyboard, with nothing picked to begin with", async () => {
    const watched = await edit(plain());
    const { page } = watched;
    const inspector = inspectorOf(page);
    // A press on empty page gives the page the keys once its objects are read (nothing is picked by it).
    const empty = await at(page, 40, 760);
    await expect
      .poll(
        async () => {
          await page.mouse.click(empty.x, empty.y);
          await page.keyboard.press("]");
          return inspector.isVisible();
        },
        { timeout: 30_000 },
      )
      .toBe(true);
    await inspector.getByRole("heading", { name: "Image" }).waitFor();
    await page.keyboard.press("]");
    await inspector.getByRole("heading", { name: "Shape" }).waitFor();
    await page.keyboard.press("[");
    await inspector.getByRole("heading", { name: "Image" }).waitFor();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("ArrowRight");
    // The three presses gather into one step.
    await changes(page, 1);
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });

  it("puts the strip and badge up when replacing a figure's picture breaks PDF/UA, and alternative text takes them away", async () => {
    const watched = await edit(accessible());
    const { page } = watched;
    const inspector = inspectorOf(page);
    await click(page, 175, 142);
    await inspector.getByRole("heading", { name: "Image" }).waitFor();
    const chooser = page.waitForEvent("filechooser");
    await inspector.getByRole("button", { name: "Replace…" }).click();
    await (await chooser).setFiles(written(png(8, 8), "logo.png"));
    await changes(page, 1);

    const strip = page.getByRole("status").filter({ hasText: "no longer meet PDF/UA-1" }).first();
    await strip.waitFor();
    await page.getByRole("button", { name: /^PDF\/UA-1 · 1 issue/ }).waitFor();
    const alt = inspector.getByRole("textbox", { name: "Alternative text" });
    expect(await alt.getAttribute("aria-invalid")).toBe("true");
    // Details opens the check with no save waiting; its Fix it first still goes to the picture.
    await page.getByRole("button", { name: "Details", exact: true }).click();
    const look = page.getByRole("dialog", { name: "Check before saving" });
    await look.waitFor();
    expect(await look.getByRole("button", { name: "Save without the claim" }).count()).toBe(0);
    await look.getByRole("button", { name: "Fix it first" }).click();
    await look.waitFor({ state: "hidden" });
    await expect.poll(() => alt.evaluate((el) => el === document.activeElement)).toBe(true);
    // The strip's Add alternative text puts the focus in the field.
    await page.getByRole("button", { name: "Add alternative text", exact: true }).click();
    await expect.poll(() => alt.evaluate((el) => el === document.activeElement)).toBe(true);
    expect(await serious(page)).toEqual([]);

    // Alternative text is one step, and the claim holds again.
    await alt.fill("A grey square");
    await alt.press("Tab");
    await changes(page, 2);
    await strip.waitFor({ state: "hidden" });
    await clean(watched);
  });

  it("checks the claims before saving and saves without the one an edit broke", async () => {
    const watched = await edit(accessible());
    const { page } = watched;
    await click(page, 175, 142);
    const chooser = page.waitForEvent("filechooser");
    await inspectorOf(page).getByRole("button", { name: "Replace…" }).click();
    await (await chooser).setFiles(written(png(8, 8), "logo.png"));
    await changes(page, 1);
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "Check before saving" });
    await dialog.waitFor();
    expect(await dialog.textContent()).toContain("PDF/UA-1");
    expect(await serious(page)).toEqual([]);
    // Fix it first closes the check, picks the picture and puts the focus in Alternative text.
    await dialog.getByRole("button", { name: "Fix it first" }).click();
    await dialog.waitFor({ state: "hidden" });
    await expect
      .poll(() =>
        inspectorOf(page)
          .getByRole("textbox", { name: "Alternative text" })
          .evaluate((el) => el === document.activeElement),
      )
      .toBe(true);
    await page.getByRole("button", { name: "Download", exact: true }).click();
    await dialog.waitFor();
    const download: Promise<Download> = page.waitForEvent("download");
    await dialog.getByRole("button", { name: "Save without the claim" }).click();
    const bytes = new Uint8Array(readFileSync(await (await download).path()));
    const text = Buffer.from(bytes).toString("latin1");
    // The earlier revision still holds the claim; the last packet in the file doesn't.
    const last = text.lastIndexOf("<?xpacket begin");
    expect(last).toBeGreaterThan(0);
    expect(text.slice(last)).not.toContain("<pdfuaid:part");
    await clean(watched);
  });
});
