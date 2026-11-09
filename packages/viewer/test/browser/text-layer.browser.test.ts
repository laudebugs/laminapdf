// The text layer (todo C5a) over real pages in Chromium, Firefox and WebKit: copy gives the text
// Lamina PDF extracts in five scripts, the encrypted copy too; spans lie over their glyph quads at
// 100 %, 200 % and a quarter turn; a selection runs across pages and stays while the view scrolls;
// a tagged file's structure owns its spans, and an untagged file says so once; a real mouse drag
// selects a word, whatever layers lie over the text, and the selection's tint never hides the ink.

import { Builder, Document, Font, init } from "@laminapdf/sdk";
import { decodePng } from "@laminapdf/test-utils/browser-harness";
import { createViewer, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { commands, userEvent } from "vitest/browser";
import { viewer as client, ENCRYPTED, PASSWORD, read } from "./helpers.js";

declare module "vitest/browser" {
  interface BrowserCommands {
    /** Drags the real mouse across the element `selector` names (vitest.browser.config.ts). */
    dragAcross: (selector: string) => Promise<void>;
    /** A PNG screenshot of the element `selector` names, base64 (vitest.browser.config.ts). */
    screenshotOf: (selector: string) => Promise<string>;
  }
}

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

  it("copies only the page text when a selection runs on past the viewer into the app", async () => {
    await show(await read("fixtures/golden/text-latin.pdf"));
    const root = view?.element as HTMLElement;
    const outside = document.createElement("p");
    outside.textContent = "Text of the app around the viewer";
    root.after(outside);
    try {
      const [first] = await spansOf(0);
      const start = first?.firstChild as Text;
      const end = outside.firstChild as Text;
      document.getSelection()?.setBaseAndExtent(start, 0, end, end.length);
      const copied = copy();
      expect(copied).toContain((first?.textContent ?? "").trim());
      expect(copied).not.toContain("Text of the app");
    } finally {
      outside.remove();
    }
  });

  it("selects only the spans' text when the whole viewer is selected: never a canvas or a gap", async () => {
    await show(await read("fixtures/golden/text-latin.pdf"));
    const root = view?.element as HTMLElement;
    document.getSelection()?.selectAllChildren(root);
    const spans = (await spansOf(0)).map((s) => s.textContent).join("");
    const canvas = root.querySelector("canvas") as HTMLCanvasElement;
    expect([
      getComputedStyle(canvas).userSelect,
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

  /** A paragraph of the app's own text, before or after the viewer. */
  function appText(where: "before" | "after"): HTMLParagraphElement {
    const p = document.createElement("p");
    p.textContent = `The app's text ${where} the viewer`;
    if (where === "before") view?.element.before(p);
    else view?.element.after(p);
    return p;
  }

  it("keeps the pages of a selection that starts before the viewer, and copies only their text", async () => {
    const v = await show(pages(30));
    const before = appText("before");
    try {
      const [first] = await spansOf(1);
      const start = before.firstChild as Text;
      document.getSelection()?.setBaseAndExtent(start, 0, (first as HTMLElement).firstChild as Text, 6);
      const copied = copy();
      await new Promise((r) => setTimeout(r, 200));
      v.goToPage(25);
      await new Promise((r) => setTimeout(r, 500));
      const mounted = [0, 1].map((p) => document.querySelector(`.lamina-page[data-page-index="${p}"]`) !== null);
      expect([
        copied.startsWith("Page 1 first line"),
        copied.endsWith("Page 2"),
        copied.includes("app"),
        mounted,
      ]).toEqual([true, true, false, [true, true]]);
    } finally {
      before.remove();
    }
  });

  it("copies the mounted pages' text when a selection runs from before the viewer to after it", async () => {
    await show(pages(30));
    const before = appText("before");
    const after = appText("after");
    try {
      const end = after.firstChild as Text;
      document.getSelection()?.setBaseAndExtent(before.firstChild as Text, 0, end, end.length);
      const copied = copy();
      expect([copied.startsWith("Page 1 first line"), copied.includes("app")]).toEqual([true, false]);
    } finally {
      before.remove();
      after.remove();
    }
  });

  it("leaves a selection from one viewer into another to the browser", async () => {
    const v = await show(pages(3));
    const otherClient = await client("pdfium");
    const otherHost = document.createElement("div");
    Object.assign(otherHost.style, { width: "800px", height: "900px" });
    document.body.append(otherHost);
    const other = createViewer({ client: otherClient });
    try {
      other.mount(otherHost);
      other.open(await otherClient.open(pages(3)));
      const [mine] = await spansOf(0);
      await expect
        .poll(() => otherHost.querySelectorAll(".lamina-text-layer span").length, { timeout: 20_000 })
        .toBeGreaterThan(0);
      const theirs = otherHost.querySelector(".lamina-text-layer span") as HTMLElement;
      document
        .getSelection()
        ?.setBaseAndExtent((mine as HTMLElement).firstChild as Text, 0, theirs.firstChild as Text, 4);
      // Neither viewer writes the clipboard: the copy event goes to the browser untouched.
      expect([copy(), v.element.contains(mine as Node)]).toEqual(["", true]);
    } finally {
      other.destroy();
      otherClient.terminate();
      otherHost.remove();
    }
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

describe("pointer", () => {
  /** The first span of page 0 with more than `min` characters, wholly in view, marked for `dragAcross`. */
  async function word(min = 8): Promise<HTMLElement> {
    const span = (await spansOf(0)).find((s) => {
      const box = s.getBoundingClientRect();
      const inView =
        box.top >= 0 && box.bottom <= window.innerHeight && box.left >= 0 && box.right <= window.innerWidth;
      return (s.textContent ?? "").trim().length > min && inView;
    });
    if (span === undefined) throw new Error("no word in view on page 0");
    span.id = "drag-target";
    return span;
  }

  /** The element the browser gives a press at the centre of `span`. */
  function hitAt(span: HTMLElement): Element | null {
    const box = span.getBoundingClientRect();
    return document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
  }

  /**
   * The selection is the start of `span`'s text, most of it: the drag stops 2 px inside the span,
   * which can fall before a narrow last letter.
   */
  function expectStartOf(span: HTMLElement): void {
    const text = span.textContent?.trim() ?? "";
    const selected = document.getSelection()?.toString().trim() ?? "";
    expect(selected.length).toBeGreaterThanOrEqual(text.length - 1);
    expect(text.startsWith(selected)).toBe(true);
  }

  it("selects a word under a mouse drag: the layers over the text let the pointer through", async () => {
    await show(await read("fixtures/golden/text-latin.pdf"));
    const span = await word();
    expect(hitAt(span)).toBe(span);
    await commands.dragAcross("#drag-target");
    expectStartOf(span);
  });

  // Firefox stopped selecting under a `user-select: none` ancestor once a field had had the focus,
  // however the field lost it, and while it still had it.
  it.each([
    ["it keeps the focus", async () => {}],
    ["Escape", async () => userEvent.keyboard("{Escape}")],
    ["blur", async (field: HTMLInputElement) => field.blur()],
    ["Tab", async () => userEvent.tab()],
  ] as const)("selects a word under a mouse drag after a text field had the focus (then: %s)", async (_name, leave) => {
    await show(await read("fixtures/golden/text-latin.pdf"));
    const field = document.createElement("input");
    document.body.append(field);
    try {
      field.focus();
      await userEvent.keyboard("12");
      await leave(field);
      const span = await word();
      await commands.dragAcross("#drag-target");
      expectStartOf(span);
    } finally {
      field.remove();
    }
  });

  it("keeps the text under a layer added on top, even one that covers the page", async () => {
    const v = await show(await read("fixtures/golden/text-latin.pdf"));
    v.addLayer({
      name: "notes",
      onMount(_page, slot) {
        const cover = document.createElement("div");
        Object.assign(cover.style, { position: "absolute", inset: "0" });
        slot.append(cover);
      },
    });
    const span = await word();
    expect(hitAt(span)).toBe(span);
    await commands.dragAcross("#drag-target");
    expectStartOf(span);
  });
});

describe("selection tint", () => {
  /** How many pixels of `selector`'s screenshot are ink: darker than any selection tint. */
  async function inkPixels(selector: string): Promise<number> {
    const png = Uint8Array.from(atob(await commands.screenshotOf(selector)), (c) => c.charCodeAt(0));
    const { pixels: data } = await decodePng(png);
    let ink = 0;
    for (let i = 0; i < data.length; i += 4) {
      const luma = 0.2126 * (data[i] ?? 255) + 0.7152 * (data[i + 1] ?? 255) + 0.0722 * (data[i + 2] ?? 255);
      if (luma < 100) ink++;
    }
    return ink;
  }

  it("leaves a selected word's ink as dark as drawn under an opaque tint", async () => {
    await show(await read("fixtures/golden/text-latin.pdf"));
    // An app's tint, opaque as the Paper selection tint is.
    const style = document.createElement("style");
    style.textContent = ".lamina-text-layer span::selection { background: #cedbdd; }";
    document.head.append(style);
    try {
      const span = (await spansOf(0)).find((s) => {
        const box = s.getBoundingClientRect();
        const inView =
          box.top >= 0 && box.bottom <= window.innerHeight && box.left >= 0 && box.right <= window.innerWidth;
        return (s.textContent ?? "").trim().length > 8 && inView;
      });
      if (span === undefined) throw new Error("no word in view on page 0");
      span.id = "tinted";
      // The page's sharp pixels are in once the word shows ink.
      await expect.poll(() => inkPixels("#tinted"), { timeout: 20_000 }).toBeGreaterThan(20);
      const before = await inkPixels("#tinted");
      document.getSelection()?.selectAllChildren(span);
      const after = await inkPixels("#tinted");
      expect(after).toBeGreaterThanOrEqual(before * 0.8);
    } finally {
      style.remove();
    }
  });
});
