// Link previews (T5a) in Chromium, Firefox and WebKit, over fixtures/links/links.pdf: the link layer
// asks for a preview 300 ms after a mouse comes onto an internal link and ends it 300 ms after it
// leaves (unless the preview holds it), on visible keyboard focus, and on a still long-press, whose
// click is then cancelled; web links never ask. `Viewer.preview` draws one picture of the
// destination through the worker, keeps at most 8, and leaves nothing behind when aborted.
import { createViewer, type LinkPreviewRequest, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { viewer as client, ENCRYPTED, PASSWORD, read } from "./helpers.js";

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

async function until(done: () => boolean, limit = 600): Promise<void> {
  for (let i = 0; i < limit && !done(); i++) await frame();
  expect(done()).toBe(true);
}

const host = document.createElement("div");
Object.assign(host.style, { width: "800px", height: "600px", position: "fixed", top: "0", left: "0" });
document.body.append(host);

let rpc: ViewerClient;
let bytes: Uint8Array;
beforeAll(async () => {
  rpc = await client();
  bytes = await read("fixtures/links/links.pdf");
});
afterAll(() => rpc.terminate());

interface Opened {
  viewer: Viewer;
  previews: LinkPreviewRequest[];
  ended: HTMLElement[];
}

async function opened(): Promise<Opened> {
  const viewer = createViewer({ client: rpc, links: { href: (page) => `#page=${page + 1}` } });
  viewer.mount(host);
  viewer.open(await rpc.open(bytes));
  const previews: LinkPreviewRequest[] = [];
  const ended: HTMLElement[] = [];
  viewer.on("linkpreview", (request) => previews.push(request));
  viewer.on("linkpreviewend", ({ anchor }) => ended.push(anchor));
  await until(() => anchors(viewer).length > 0);
  return { viewer, previews, ended };
}

function anchors(viewer: Viewer): HTMLAnchorElement[] {
  const shell = viewer.element.querySelector(`[data-page-index="0"]`) ?? viewer.element;
  return [...shell.querySelectorAll<HTMLAnchorElement>(".lamina-layer-annot a.lamina-link")];
}

/** The first anchor named `name`. */
function anchor(viewer: Viewer, name: string): HTMLAnchorElement {
  const found = anchors(viewer).find((a) => a.getAttribute("aria-label") === name);
  if (found === undefined) throw new Error(`no link named ${name}`);
  return found;
}

function pointer(el: Element, type: string, init: PointerEventInit = {}): void {
  const box = el.getBoundingClientRect();
  el.dispatchEvent(
    new PointerEvent(type, {
      bubbles: true,
      cancelable: true,
      pointerId: 1,
      pointerType: "mouse",
      clientX: box.left + box.width / 2,
      clientY: box.top + box.height / 2,
      ...init,
    }),
  );
}

describe("link previews from the link layer", () => {
  it("asks 300 ms after the mouse comes onto an internal link, with its destination", async () => {
    const { viewer, previews } = await opened();
    const link = anchor(viewer, "Link to page 3");
    pointer(link, "pointerover");
    await sleep(150);
    const early = previews.length;
    await sleep(300);
    expect([early, previews[0]?.shows.kind, previews[0]?.pointer, previews[0]?.anchor === link]).toEqual([
      0,
      "target",
      "fine",
      true,
    ]);
    viewer.destroy();
  });

  it("asks nothing when the mouse leaves within 300 ms", async () => {
    const { viewer, previews } = await opened();
    const link = anchor(viewer, "Link to page 3");
    pointer(link, "pointerover");
    await sleep(100);
    pointer(link, "pointerout", { relatedTarget: viewer.element } as PointerEventInit);
    await sleep(400);
    expect(previews).toHaveLength(0);
    viewer.destroy();
  });

  it("ends 300 ms after the mouse leaves, unless the preview holds it", async () => {
    const { viewer, ended } = await opened();
    const link = anchor(viewer, "Link to page 3");
    pointer(link, "pointerover");
    await sleep(400);
    pointer(link, "pointerout", { relatedTarget: viewer.element } as PointerEventInit);
    viewer.holdLinkPreview(true);
    await sleep(500);
    const held = ended.length;
    viewer.holdLinkPreview(false);
    await sleep(450);
    expect([held, ended.length]).toEqual([0, 1]);
    viewer.destroy();
  });

  it("asks on a still long-press and cancels the click its lift sends", async () => {
    const { viewer, previews } = await opened();
    const link = anchor(viewer, "Link to page 3");
    const touch = { pointerType: "touch", pointerId: 7 };
    pointer(link, "pointerdown", touch);
    await sleep(600);
    pointer(link, "pointerup", touch);
    const followed = !link.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, button: 0 }));
    const back = viewer.history.getState().back;
    expect([previews[0]?.pointer, followed, back]).toEqual(["touch", true, undefined]);
    viewer.destroy();
  });

  it("asks nothing for a long-press that moves", async () => {
    const { viewer, previews } = await opened();
    const link = anchor(viewer, "Link to page 3");
    const touch = { pointerType: "touch", pointerId: 7 };
    pointer(link, "pointerdown", touch);
    const box = link.getBoundingClientRect();
    link.dispatchEvent(
      new PointerEvent("pointermove", {
        bubbles: true,
        pointerType: "touch",
        pointerId: 7,
        clientX: box.left + box.width / 2 + 20,
        clientY: box.top + box.height / 2,
      }),
    );
    await sleep(600);
    expect(previews).toHaveLength(0);
    viewer.destroy();
  });

  it("starts no render when the mouse leaves within 300 ms", async () => {
    const { viewer } = await opened();
    // Wait until the viewer has drawn what it wants: 500 ms with no render starting.
    let renders = 0;
    let last = performance.now();
    const off = rpc.on("render-start", () => {
      renders++;
      last = performance.now();
    });
    await until(() => performance.now() - last > 500);
    renders = 0;
    const link = anchor(viewer, "Link to page 3");
    pointer(link, "pointerover");
    await sleep(100);
    pointer(link, "pointerout", { relatedTarget: viewer.element } as PointerEventInit);
    await sleep(500);
    off();
    expect(renders).toBe(0);
    viewer.destroy();
  });

  it("never asks for a web link", async () => {
    const { viewer, previews } = await opened();
    pointer(anchor(viewer, "The project site"), "pointerover");
    await sleep(450);
    expect(previews).toHaveLength(0);
    viewer.destroy();
  });

  it("stops the OS's link menu on a link", async () => {
    const { viewer } = await opened();
    const link = anchor(viewer, "Link to page 3");
    expect(getComputedStyle(link).getPropertyValue("-webkit-touch-callout") || "none").toBe("none");
    viewer.destroy();
  });
});

describe("Viewer.preview", () => {
  it("draws the destination as one picture", async () => {
    const { viewer } = await opened();
    const picture = await viewer.preview({ page: 2, view: { kind: "at", x: 72, y: 100 } });
    expect([picture.bitmap.width > 0, picture.width > 0, picture.height > 0, picture.page]).toEqual([
      true,
      true,
      true,
      2,
    ]);
    viewer.destroy();
  });

  it("keeps at most 8 pictures", async () => {
    const { viewer } = await opened();
    for (let y = 0; y < 12; y++) await viewer.preview({ page: 1, view: { kind: "at", x: 72, y: 40 + y * 50 } });
    expect(viewer.stats().previewBitmaps).toBe(8);
    viewer.destroy();
  });

  it("rejects when aborted and keeps nothing", async () => {
    const { viewer } = await opened();
    const abort = new AbortController();
    const picture = viewer.preview({ page: 2, view: { kind: "fit" } }, { signal: abort.signal });
    abort.abort();
    const error = await picture.then(
      () => undefined,
      (e: unknown) => e,
    );
    expect([
      (error as Error | undefined)?.name,
      viewer.stats().previewBitmaps,
      viewer.stats().drawsOnUnmounted,
    ]).toEqual(["AbortError", 0, 0]);
    viewer.destroy();
  });

  it("keeps one picture for two identical previews drawn at once", async () => {
    const { viewer } = await opened();
    const target = { page: 2, view: { kind: "at", x: 72, y: 100 } } as const;
    await Promise.all([viewer.preview(target), viewer.preview(target)]);
    expect(viewer.stats().previewBitmaps).toBe(1);
    viewer.destroy();
  });

  it("draws an encrypted document opened with its password, asking nothing more", async () => {
    const viewer = createViewer({ client: rpc });
    viewer.mount(host);
    let stuck = 0;
    const off = rpc.on("render-stuck", () => {
      stuck++;
    });
    viewer.open(await rpc.open(await read(ENCRYPTED), { password: PASSWORD }));
    const picture = await viewer.preview({ page: 0, view: { kind: "at", x: 56, y: 60 } });
    off();
    // Its text was read too: the document is decrypted, not merely drawn.
    expect([picture.bitmap.width > 0, picture.text.length > 0, stuck]).toEqual([true, true, 0]);
    viewer.destroy();
  });

  it("closes its pictures on a zoom change", async () => {
    const { viewer } = await opened();
    await viewer.preview({ page: 2, view: { kind: "fit" } });
    viewer.setZoom(150);
    await frame();
    expect(viewer.stats().previewBitmaps).toBe(0);
    viewer.destroy();
  });
});
