// Copy modes and column selection (todo U2a) over real pages in Chromium, Firefox and WebKit:
// copy as paragraph follows Lamina PDF's reading order on U2b's corpus and scores at least its
// threshold; it writes HTML too; a paragraph pastes without a line break inside it; Alt-drag
// selects one column; and the selection says when it is settled. Copy exact's lines are
// layers/copy.test.ts's, and the commands' copy is the web app's e2e test's.

import { init } from "@laminapdf/sdk";
import { createViewer, type TextSelection, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { viewer as client, read } from "./helpers.js";

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

/** A viewer 800 × 1100 px over `path`, once page 1's spans are in. */
async function show(path: string, onSelection?: (s: TextSelection | undefined) => void): Promise<Viewer> {
  client_ = await client("pdfium");
  host = document.createElement("div");
  Object.assign(host.style, { width: "800px", height: "1100px" });
  document.body.append(host);
  view = createViewer({ client: client_, zoom: "fit-page", text: onSelection === undefined ? true : { onSelection } });
  view.mount(host);
  view.open(await client_.open(await read(path)));
  await expect.poll(() => spans().length, { timeout: 20_000 }).toBeGreaterThan(0);
  return view;
}

const spans = () => [
  ...document.querySelectorAll<HTMLElement>('.lamina-page[data-page-index="0"] .lamina-text-layer span'),
];

/** What copying the selection writes, as plain text and as HTML. */
function copy(): { text: string; html: string } {
  const event = new ClipboardEvent("copy", { clipboardData: new DataTransfer(), bubbles: true, cancelable: true });
  document.dispatchEvent(event);
  return {
    text: event.clipboardData?.getData("text/plain") ?? "",
    html: event.clipboardData?.getData("text/html") ?? "",
  };
}

function selectPage(): void {
  const layer = document.querySelector('.lamina-page[data-page-index="0"] .lamina-text-layer') as HTMLElement;
  document.getSelection()?.selectAllChildren(layer);
}

/** Copy as paragraph once the page's reading order has been read (the first copy may come before). */
async function paragraphCopy(): Promise<string> {
  // The selection asks for the page's blocks; a copy after they arrive joins them.
  let last = "";
  await expect
    .poll(
      () => {
        const now = copy().text;
        const settled = now === last && now !== "";
        last = now;
        return settled;
      },
      { timeout: 20_000, interval: 300 },
    )
    .toBe(true);
  return last;
}

/** The Levenshtein distance between two sequences. */
function levenshtein<T>(a: readonly T[], b: readonly T[]): number {
  let previous = Array.from({ length: b.length + 1 }, (_, j) => j);
  a.forEach((x, i) => {
    const current = [i + 1];
    b.forEach((y, j) => {
      current.push(
        Math.min(
          (previous[j] as number) + (x === y ? 0 : 1),
          (previous[j + 1] as number) + 1,
          (current[j] as number) + 1,
        ),
      );
    });
    previous = current;
  });
  return previous[b.length] as number;
}

const squash = (s: string) => s.replace(/\s+/g, " ").trim();

/**
 * U2b's measure, simplified to one copied text: **order** is how far the blocks come in reading
 * order (1 − the edit distance between the order their texts are found in and 0…n−1, over n), and
 * **text** the share of blocks whose text is found whole; the score is their mean.
 */
function score(copied: string, blocks: readonly string[]): number {
  const text = squash(copied);
  const found = blocks.map((b, k) => ({ k, at: text.indexOf(squash(b)) })).filter((f) => f.at >= 0);
  const order = found.sort((a, b) => a.at - b.at).map((f) => f.k);
  const expected = blocks.map((_, k) => k);
  const orderScore = blocks.length === 0 ? 1 : 1 - levenshtein(order, expected) / blocks.length;
  const textScore = blocks.length === 0 ? 1 : found.length / blocks.length;
  return (orderScore + textScore) / 2;
}

describe("copy as paragraph", () => {
  // The corpus's files whose first page holds every block: a page's reading order is all a
  // selection on it can follow.
  const names = [
    "one-column",
    "two-columns",
    "three-columns",
    "sidebar",
    "pull-quote",
    "narrow-gutter",
    "wrapped-grid",
    "header-footer",
    "hebrew-two-columns",
    "table",
  ].flatMap((n) => [`${n}-ordered`, `${n}-reversed`]);

  it("scores at least U2b's threshold over the reading-order corpus", async () => {
    const threshold = JSON.parse(new TextDecoder().decode(await read("fixtures/reading-order/threshold.json"))).mean;
    const scores: Record<string, number> = {};
    for (const name of names) {
      const expected = JSON.parse(new TextDecoder().decode(await read(`fixtures/reading-order/${name}.json`)));
      const blocks: string[] = expected.blocks
        .filter((b: { page: number }) => b.page === 0)
        .map((b: { text: string }) => b.text);
      await show(`fixtures/reading-order/${name}.pdf`);
      selectPage();
      scores[name] = score(await paragraphCopy(), blocks);
      view?.destroy();
      host?.remove();
      await client_?.terminate();
    }
    const mean = Object.values(scores).reduce((a, b) => a + b, 0) / names.length;
    expect(mean, JSON.stringify(scores, null, 1)).toBeGreaterThanOrEqual(threshold);
  });

  it("pastes each paragraph into a textarea without a line break inside it", async () => {
    await show("fixtures/reading-order/two-columns-ordered.pdf");
    selectPage();
    const text = await paragraphCopy();
    const area = document.createElement("textarea");
    area.value = text;
    document.body.append(area);
    const paragraphs = area.value.split("\n\n");
    area.remove();
    expect(paragraphs.length).toBeGreaterThan(2);
    for (const p of paragraphs) expect(p).not.toContain("\n");
  });

  it("writes HTML with a paragraph per block", async () => {
    await show("fixtures/reading-order/two-columns-ordered.pdf");
    selectPage();
    await paragraphCopy();
    expect(copy().html.match(/<p>/g)?.length ?? 0).toBeGreaterThan(2);
  });
});

describe("column selection", () => {
  it("selects the runs inside an Alt-drag's rectangle, one column of two", async () => {
    let last: TextSelection | undefined;
    await show("fixtures/reading-order/two-columns-ordered.pdf", (s) => {
      last = s;
    });
    const root = view?.element as HTMLElement;
    const left = spans()
      .map((s) => s.getBoundingClientRect())
      .filter((r) => r.left < root.getBoundingClientRect().left + root.clientWidth / 2);
    const top = Math.min(...left.map((r) => r.top)) - 2;
    const bottom = Math.max(...left.map((r) => r.bottom)) + 2;
    const x0 = Math.min(...left.map((r) => r.left)) - 2;
    const x1 = Math.max(...left.map((r) => r.right)) + 2;
    const at = (type: string, x: number, y: number) =>
      new PointerEvent(type, {
        clientX: x,
        clientY: y,
        altKey: true,
        button: 0,
        pointerId: 7,
        pointerType: "mouse",
        bubbles: true,
        cancelable: true,
      });
    root.dispatchEvent(at("pointerdown", x0, top));
    document.dispatchEvent(at("pointermove", x1, bottom));
    document.dispatchEvent(at("pointerup", x1, bottom));
    expect(last?.column).toBe(true);
    expect(last?.settled).toBe(true);
    const expected = JSON.parse(
      new TextDecoder().decode(await read("fixtures/reading-order/two-columns-ordered.json")),
    );
    const text = squash(copy().text);
    // The left column's paragraphs, not the right column's.
    const blocks: { box: number[]; text: string; role: string }[] = expected.blocks;
    const leftBlocks = blocks.filter((b) => b.role !== "heading" && (b.box[0] as number) < 200);
    const rightBlocks = blocks.filter((b) => (b.box[0] as number) > 250);
    for (const b of leftBlocks) expect(text).toContain(squash(b.text).slice(0, 40));
    for (const b of rightBlocks) expect(text).not.toContain(squash(b.text).slice(0, 40));
    // Escape ends it.
    root.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    expect(last).toBeUndefined();
  });
});

describe("selection events", () => {
  it("are unsettled while the pointer is down and settled once it is up", async () => {
    const seen: TextSelection[] = [];
    await show("fixtures/reading-order/one-column-ordered.pdf", (s) => {
      if (s !== undefined) seen.push(s);
    });
    const root = view?.element as HTMLElement;
    const first = spans()[0] as HTMLElement;
    const r = first.getBoundingClientRect();
    root.dispatchEvent(
      new PointerEvent("pointerdown", { clientX: r.left + 1, clientY: r.top + 2, pointerType: "mouse", bubbles: true }),
    );
    document.getSelection()?.selectAllChildren(first);
    await expect.poll(() => seen.length).toBeGreaterThan(0);
    expect(seen.at(-1)?.settled).toBe(false);
    document.dispatchEvent(
      new PointerEvent("pointerup", { clientX: r.right, clientY: r.top + 2, pointerType: "mouse", bubbles: true }),
    );
    expect(seen.at(-1)?.settled).toBe(true);
    expect(seen.at(-1)?.pointerType).toBe("mouse");
  });
});
