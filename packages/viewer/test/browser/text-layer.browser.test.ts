// The text layer (todo C5a) over real pages in Chromium, Firefox and WebKit: copy gives the text
// Lamina PDF extracts in five scripts, the encrypted copy too; spans lie over their glyph quads at
// 100 %, 200 % and a quarter turn; a selection runs across pages and stays while the view scrolls;
// a tagged file's structure owns its spans, and an untagged file says so once.

import { Builder, Document, Font, init } from "@laminapdf/sdk";
import { createViewer, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { viewer as client, ENCRYPTED, PASSWORD, read } from "./helpers.js";

let client_: ViewerClient | undefined;
let view: Viewer | undefined;
let host: HTMLElement | undefined;

beforeAll(async () => {
  await init({ text: true });
});

afterEach(async () => {
  view?.destroy();
  host?.remove();
  await client_?.terminate();
  [client_, view, host] = [undefined, undefined, undefined];
  document.getSelection()?.removeAllRanges();
});

/** A viewer 800 × 900 px over `bytes`, once page `page`'s spans are in. */
async function show(bytes: Uint8Array, options: { password?: string; page?: number } = {}): Promise<Viewer> {
  client_ = await client("pdfium");
  host = document.createElement("div");
  Object.assign(host.style, { width: "800px", height: "900px" });
  document.body.append(host);
  view = createViewer({ client: client_ });
  view.mount(host);
  view.open(await client_.open(bytes, options.password === undefined ? {} : { password: options.password }));
  await spansOf(options.page ?? 0);
  return view;
}

/** Page `page`'s spans, once there are some. */
async function spansOf(page: number): Promise<HTMLElement[]> {
  const find = () => [
    ...document.querySelectorAll<HTMLElement>(`.lamina-page[data-page-index="${page}"] .lamina-text-layer span`),
  ];
  await expect.poll(() => find().length, { timeout: 20_000 }).toBeGreaterThan(0);
  return find();
}

/** What copying the current selection writes (Firefox gives the event its own clipboard data). */
function copy(): string {
  const event = new ClipboardEvent("copy", { clipboardData: new DataTransfer(), bubbles: true, cancelable: true });
  document.dispatchEvent(event);
  return event.clipboardData?.getData("text/plain") ?? "";
}

function selectAllOf(page: number): void {
  const layer = document.querySelector(`.lamina-page[data-page-index="${page}"] .lamina-text-layer`) as HTMLElement;
  document.getSelection()?.selectAllChildren(layer);
}

const squash = (s: string) => s.replace(/\s+/g, " ").trim();

/** Page 1's lines as Lamina PDF extracts them (B9's own expectation), one per line. */
async function laminaText(path: string, password?: string): Promise<string> {
  using doc = Document.open(await read(path), password === undefined ? {} : { password });
  return doc.page(0).text().lines.join("\n");
}

describe("copy", () => {
  it("gives the five scripts' text as Lamina PDF extracts it", async () => {
    await show(await read("fixtures/golden/text-shaped.pdf"));
    selectAllOf(0);
    expect(squash(copy())).toBe(squash(await laminaText("fixtures/golden/text-shaped.pdf")));
  });

  it("gives the same text from the encrypted copy, with the password given at open", async () => {
    await show(await read(ENCRYPTED), { password: PASSWORD });
    selectAllOf(0);
    expect(squash(copy())).toBe(squash(await laminaText("fixtures/golden/text-shaped.pdf")));
  });

  it("gives the RC4-128 multiscript copy's text too", async () => {
    await show(await read("fixtures/text/encrypted/build-text-multiscript-rc4-128.pdf"), { password: PASSWORD });
    selectAllOf(0);
    expect(squash(copy())).toBe(squash(await laminaText("fixtures/golden/build-text-multiscript.pdf")));
  });

  it("selects only the spans' text when the whole viewer is selected: never a canvas or a gap", async () => {
    await show(await read("fixtures/golden/text-latin.pdf"));
    const root = view?.element as HTMLElement;
    document.getSelection()?.selectAllChildren(root);
    const spans = (await spansOf(0)).map((s) => s.textContent).join("");
    expect([
      getComputedStyle(root).userSelect,
      getComputedStyle((await spansOf(0))[0] as HTMLElement).userSelect,
      squash(copy()) === squash(await laminaText("fixtures/golden/text-latin.pdf")),
      (document.getSelection()?.toString() ?? "").replace(/\s/g, "") === spans.replace(/\s/g, ""),
    ]).toEqual(["none", "text", true, true]);
  });
});

describe("alignment", () => {
  /** Each span's box against its glyphs' quads mapped to the page's CSS px, worst distance. */
  async function worstMiss(): Promise<number> {
    const doc = client_?.document;
    if (doc === undefined || view === undefined) throw new Error("no document");
    const { runs, quads } = await (client_ as ViewerClient).pageText(0);
    const shell = document.querySelector('.lamina-page[data-page-index="0"]') as HTMLElement;
    const page = shell.getBoundingClientRect();
    const info = doc.pages[0] as { width: number; height: number };
    const rotation = view.getState().rotation;
    const turned = rotation === 90 || rotation === 270;
    const scale = page.width / (turned ? info.height : info.width);
    const map = (x: number, y: number): [number, number] => {
      const [w, h] = [info.width * scale, info.height * scale];
      const [px, py] = [x * scale, y * scale];
      return rotation === 90
        ? [h - py, px]
        : rotation === 180
          ? [w - px, h - py]
          : rotation === 270
            ? [py, w - px]
            : [px, py];
    };
    const spans = await spansOf(0);
    let worst = 0;
    runs
      .filter((run) => run.glyphRange[1] > run.glyphRange[0] && run.text !== "")
      .forEach((run, k) => {
        const points: [number, number][] = [];
        for (let i = run.glyphRange[0] * 8; i < run.glyphRange[1] * 8; i += 2)
          points.push(map(quads[i] ?? 0, quads[i + 1] ?? 0));
        const xs = points.map((p) => p[0]);
        const ys = points.map((p) => p[1]);
        const box = (spans[k] as HTMLElement).getBoundingClientRect();
        const slack = Math.max(1.5, 0.02 * run.size * scale);
        const miss = Math.max(
          Math.abs(box.left - page.left - Math.min(...xs)),
          Math.abs(box.top - page.top - Math.min(...ys)),
          Math.abs(box.right - page.left - Math.max(...xs)),
          Math.abs(box.bottom - page.top - Math.max(...ys)),
        );
        worst = Math.max(worst, miss / slack);
      });
    return worst;
  }

  it("lays every span over its glyphs at 100 % and 200 % and turned a quarter", async () => {
    const v = await show(await read("fixtures/golden/text-latin.pdf"));
    v.setZoom(100);
    await new Promise((r) => setTimeout(r, 300));
    const at100 = await worstMiss();
    v.setZoom(200);
    await new Promise((r) => setTimeout(r, 300));
    const at200 = await worstMiss();
    v.rotate(90);
    await new Promise((r) => setTimeout(r, 300));
    const turned = await worstMiss();
    // 1 is the tolerance: 1.5 CSS px or 2 % of the font size, whichever is larger.
    expect([at100 <= 1, at200 <= 1, turned <= 1]).toEqual([true, true, true]);
  });
});

describe("selection across pages", () => {
  /** A document of `n` pages, each with two lines of text. */
  function pages(n: number): Uint8Array {
    using b = new Builder();
    const font = Font.standard("Helvetica");
    for (let i = 1; i <= n; i++) {
      const page = b.page("A4");
      page.surface().text(`Page ${i} first line`, { font, size: 14, at: [72, 100] });
      page.surface().text(`Page ${i} last line`, { font, size: 14, at: [72, 700] });
      page.finish();
    }
    using doc = b.finish();
    return doc.save();
  }

  it("copies the tail of one page then the head of the next, and keeps both while the view scrolls far away", async () => {
    const v = await show(pages(30));
    const [, last] = await spansOf(0);
    const [first] = await spansOf(1);
    const range = document.createRange();
    range.setStart((last as HTMLElement).firstChild as Text, 5);
    range.setEnd((first as HTMLElement).firstChild as Text, 6);
    document.getSelection()?.removeAllRanges();
    document.getSelection()?.addRange(range);
    const copied = copy();
    // The selection's pages are pinned on `selectionchange`, which Firefox sends a little later.
    await new Promise((r) => setTimeout(r, 200));
    v.goToPage(25);
    await new Promise((r) => setTimeout(r, 500));
    const mounted = [0, 1].map((p) => document.querySelector(`.lamina-page[data-page-index="${p}"]`) !== null);
    expect([copied, mounted, document.getSelection()?.isCollapsed]).toEqual([
      "1 last line\nPage 2",
      [true, true],
      false,
    ]);
  });
});

describe("structure", () => {
  it("gives a tagged file's headings and paragraphs the spans their content drew", async () => {
    await show(await read("fixtures/golden/tagged-report.pdf"));
    await expect
      .poll(() => document.querySelector(".lamina-structure [role=heading]")?.getAttribute("aria-owns") ?? "", {
        timeout: 10_000,
      })
      .not.toBe("");
    const heading = document.querySelector(".lamina-structure [role=heading]") as HTMLElement;
    const owned = (heading.getAttribute("aria-owns") ?? "")
      .split(" ")
      .map((id) => document.getElementById(id)?.textContent);
    expect([heading.getAttribute("aria-level"), owned.length > 0, owned.every((t) => (t ?? "") !== "")]).toEqual([
      "1",
      true,
      true,
    ]);
  });

  it("follows the structure tree's order with its roles: a heading, a paragraph, a figure named by its alt text", async () => {
    await show(await read("fixtures/golden/tagged-report.pdf"));
    await expect
      .poll(() => document.querySelectorAll(".lamina-structure [role]").length, { timeout: 10_000 })
      .toBeGreaterThan(3);
    const roles = [...document.querySelectorAll(".lamina-structure [role]")]
      .slice(0, 3)
      .map((el) => [el.getAttribute("role"), el.getAttribute("aria-label")]);
    expect(roles).toEqual([
      ["heading", null],
      ["paragraph", null],
      ["img", "A bar chart of sales by quarter, rising from 120 to 310"],
    ]);
  });

  it("says once that an untagged file is untagged", async () => {
    await show(await read("fixtures/golden/examples/hello.pdf"));
    await expect
      .poll(() => document.querySelectorAll(".lamina-structure[role=status]").length, { timeout: 10_000 })
      .toBe(1);
    expect(document.querySelector(".lamina-structure")?.textContent).toBe("Untagged document");
  });
});
