// The link layer and the history (C6b) in Chromium, Firefox and WebKit, over fixtures/links/links.pdf
// (every link kind): anchors in page order with names, internal links going there as history
// jumps that Back and Forward undo and redo, web links asking first, refused and unsupported ones
// doing nothing, and text beside a link still reachable by the pointer.
import { createViewer, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { read, viewer as client } from "./helpers.js";

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

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

/** The viewer over links.pdf, with `#page=` addresses and web links collected. */
async function opened(): Promise<{ viewer: Viewer; external: string[] }> {
  const external: string[] = [];
  const viewer = createViewer({
    client: rpc,
    links: { href: (page) => `#page=${page + 1}`, onExternal: (href) => external.push(href) },
  });
  viewer.mount(host);
  viewer.open(await rpc.open(bytes));
  await until(() => anchors(viewer, 0).length > 0);
  return { viewer, external };
}

function anchors(viewer: Viewer, page: number): HTMLAnchorElement[] {
  const shell = viewer.element.querySelector(`[data-page-index="${page}"]`) ?? viewer.element;
  return [...shell.querySelectorAll<HTMLAnchorElement>(".lamina-layer-annot a.lamina-link")];
}

const click = (el: Element, init: MouseEventInit = {}) =>
  el.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, button: 0, ...init }));

describe("the link layer", () => {
  it("draws every visible link of the page in order, named", async () => {
    const { viewer } = await opened();
    const names = anchors(viewer, 0).map((a) => a.getAttribute("aria-label"));
    expect(names).toHaveLength(17);
    expect(names.slice(0, 5)).toEqual([
      "Link to page 3",
      "Link to page 2",
      "Link to page 3",
      "The project site",
      "mailto:someone@example.com",
    ]);
    expect(names[5]).toBe("https://xn--bcher-kva.example/");
    viewer.destroy();
  });

  it("gives internal links the app's address and web links their own", async () => {
    const { viewer } = await opened();
    const [explicit, , , https] = anchors(viewer, 0);
    expect(explicit?.getAttribute("href")).toBe("#page=3");
    expect(https?.getAttribute("href")).toBe("https://example.com/a?b=c#d");
    expect(https?.rel).toBe("noopener noreferrer");
    viewer.destroy();
  });

  it("leaves refused, unsupported and broken links without an address, saying why", async () => {
    const { viewer } = await opened();
    const links = anchors(viewer, 0);
    for (const index of [6, 7, 8, 9, 12, 13, 14, 16]) {
      const a = links[index] as HTMLAnchorElement;
      expect(a.hasAttribute("href")).toBe(false);
      expect(a.getAttribute("aria-disabled")).toBe("true");
      expect(a.title).not.toBe("");
    }
    expect(links[9]?.title).toContain("budget-2026.pdf");
    viewer.destroy();
  });

  it("goes to an internal link's page as a jump that Back and Forward undo and redo", async () => {
    const { viewer } = await opened();
    const [explicit] = anchors(viewer, 0);
    expect(click(explicit as Element)).toBe(false);
    await until(() => viewer.getState().visibleRange.first === 2);
    expect(viewer.history.back()).toBe(true);
    await until(() => viewer.getState().currentPage === 0);
    expect(viewer.history.forward()).toBe(true);
    await until(() => viewer.getState().visibleRange.first === 2);
    viewer.destroy();
  });

  it("follows a named action", async () => {
    const { viewer } = await opened();
    click(anchors(viewer, 0)[10] as Element);
    await until(() => viewer.getState().currentPage === 1);
    viewer.destroy();
  });

  it("asks before a web link opens, on any click", async () => {
    const { viewer, external } = await opened();
    const https = anchors(viewer, 0)[3] as Element;
    expect(click(https)).toBe(false);
    expect(click(https, { ctrlKey: true })).toBe(false);
    https.dispatchEvent(new MouseEvent("auxclick", { bubbles: true, cancelable: true, button: 1 }));
    expect(external).toEqual(Array(3).fill("https://example.com/a?b=c#d"));
    viewer.destroy();
  });

  it("leaves a modified click on an internal link to the browser", async () => {
    const { viewer } = await opened();
    expect(click(anchors(viewer, 0)[0] as Element, { ctrlKey: true })).toBe(true);
    expect(viewer.history.getState().current).toBeUndefined();
    viewer.destroy();
  });

  it("is in the Tab order in page order", async () => {
    const { viewer } = await opened();
    const links = anchors(viewer, 0);
    expect(links.every((a) => a.tabIndex === 0)).toBe(true);
    const all = [...viewer.element.querySelectorAll("a.lamina-link")];
    expect(all.slice(0, links.length)).toEqual(links);
    viewer.destroy();
  });

  it("lets the pointer reach the text beside a link", async () => {
    const { viewer } = await opened();
    const link = anchors(viewer, 0)[0] as HTMLElement;
    const r = link.getBoundingClientRect();
    expect(document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2)).toBe(link);
    // Left of the link (it starts 72 pt in): the page itself, not a layer's slot.
    const beside = document.elementFromPoint(r.left - 10, r.top + r.height / 2);
    expect(beside).not.toBeNull();
    expect(beside?.closest(".lamina-layer-annot, .lamina-layer-highlight")).toBeNull();
    viewer.destroy();
  });
});
