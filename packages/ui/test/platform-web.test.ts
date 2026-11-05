import { describe, expect, it } from "vitest";
import { message } from "../src/messages.ts";
import type { PlatformFile } from "../src/platform/types.ts";
import { createWebPlatform, detectCapabilities, type WebScope } from "../src/platform/web.ts";
import { fakeFile, fakeIndexedDb, fakeScope } from "./fakes.ts";

const PDF = new Uint8Array([37, 80, 68, 70, 45, 50]);
const method = () => undefined;
/** The names of the handles whose permission was requested, in order. */
const requested: string[] = [];

/**
 * A file handle over `bytes` that records what is written to it. It holds `kept` permission
 * already, and a request for one answers `permission`.
 */
function fakeHandle(name: string, bytes = PDF, permission: PermissionState = "granted", kept: PermissionState = "prompt") {
  const written: Uint8Array[] = [];
  const handle = {
    kind: "file" as const,
    name,
    getFile: async () => new File([bytes], name, { type: "application/pdf" }),
    createWritable: async () => ({
      write: async (chunk: Uint8Array) => void written.push(chunk),
      close: async () => undefined,
    }),
    queryPermission: async () => kept,
    requestPermission: async () => {
      requested.push(name);
      return permission;
    },
  };
  return { handle: handle as unknown as FileSystemFileHandle, written };
}

/** A window like an installed Chromium's: pickers, writable handles, launch queue, OPFS. */
function chromiumLike(parts: Partial<WebScope> = {}): WebScope & { readonly opened: string[] } {
  return fakeScope({
    showOpenFilePicker: async () => [],
    showSaveFilePicker: async () => fakeHandle("saved.pdf").handle,
    FileSystemFileHandle: { prototype: { createWritable: method } },
    FileSystemHandle: { prototype: { requestPermission: method } },
    launchQueue: { setConsumer: () => undefined },
    matchMedia: (query) => ({ matches: query === "(display-mode: standalone)" }),
    navigator: { storage: { getDirectory: method, persist: method } } as unknown as Navigator,
    crossOriginIsolated: true,
    ...parts,
  });
}

/** A document whose created elements record their clicks. */
function recordingDocument() {
  const clicked: Record<string, unknown>[] = [];
  const document = {
    createElement: () => {
      const element: Record<string, unknown> = { addEventListener: method };
      element.click = () => clicked.push(element);
      return element;
    },
  } as unknown as Document;
  return { document, clicked };
}

const abort = () => Promise.reject(new DOMException("The user aborted a request.", "AbortError"));

describe("the web platform's capabilities", () => {
  it("are all off in a browser with none of the APIs, but URLs still open", () => {
    const flags = detectCapabilities(fakeScope());
    expect(Object.entries(flags).filter(([, on]) => on)).toEqual([["canOpenUrl", true]]);
  });

  it("are on where an installed Chromium has the APIs, and never the desktop-only ones", () => {
    // Without IndexedDB, handles can't be kept, so recent files can't reopen.
    expect(detectCapabilities(chromiumLike())).toEqual({
      canWriteInPlace: true,
      canSaveAsDialog: true,
      canWatchFiles: false,
      hasNativePrint: false,
      multiWindow: false,
      hasFileHandlers: true,
      canReopenRecent: false,
      hasOpfs: true,
      canPersistStorage: true,
      hasNativeMenus: false,
      canOpenUrl: true,
      crossOriginIsolated: true,
      ownsKeyboard: false,
    });
    expect(detectCapabilities(chromiumLike({ indexedDB: {} as IDBFactory })).canReopenRecent).toBe(true);
  });

  it("need the app installed before the OS can open files with it", () => {
    const tab = chromiumLike({ matchMedia: () => ({ matches: false }) });
    expect(detectCapabilities(tab).hasFileHandlers).toBe(false);
    expect(createWebPlatform(tab).howToEnable("hasFileHandlers")).toBe(message("platform.enable.install"));
    expect(createWebPlatform(fakeScope()).howToEnable("hasFileHandlers")).toBeUndefined();
  });

  it("say how to enable what another browser has, and nothing for what the web never has", () => {
    const platform = createWebPlatform(fakeScope());
    expect(platform.howToEnable("canWriteInPlace")).toBe(message("platform.enable.savePicker"));
    expect(platform.howToEnable("canSaveAsDialog")).toBe(message("platform.enable.savePicker"));
    expect(platform.howToEnable("canReopenRecent")).toBe(message("platform.enable.reopen"));
    expect(platform.howToEnable("canWatchFiles")).toBeUndefined();
    expect(platform.howToEnable("hasNativeMenus")).toBeUndefined();
    expect(createWebPlatform(chromiumLike()).howToEnable("canWriteInPlace")).toBeUndefined();
  });
});

describe("opening files on the web", () => {
  it("reads a picked file by ranges and marks it writable where handles can write", async () => {
    const { handle } = fakeHandle("picked.pdf");
    const platform = createWebPlatform(chromiumLike({ showOpenFilePicker: async () => [handle] }));
    const [file] = await platform.openFiles({ multiple: false });
    expect(file).toMatchObject({ name: "picked.pdf", size: PDF.length, writable: true, handle });
    expect(await file?.read(1, 3)).toEqual(PDF.slice(1, 4));
  });

  it("returns nothing when the picker is dismissed", async () => {
    const platform = createWebPlatform(chromiumLike({ showOpenFilePicker: abort }));
    expect(await platform.openFiles({ multiple: true })).toEqual([]);
  });

  it("passes on a picker failure that isn't a dismissal", async () => {
    const failing = () => Promise.reject(new DOMException("blocked", "SecurityError"));
    const platform = createWebPlatform(chromiumLike({ showOpenFilePicker: failing }));
    await expect(platform.openFiles({ multiple: false })).rejects.toThrow("blocked");
  });

  it("hands files the OS opened in the app to every listener", async () => {
    let consumer: ((params: { files: readonly FileSystemHandle[] }) => void) | undefined;
    const launchQueue = {
      setConsumer: (c: typeof consumer) => {
        consumer = c;
      },
    };
    const platform = createWebPlatform(chromiumLike({ launchQueue }));
    const received: PlatformFile[][] = [];
    platform.onOpenRequest((files) => received.push(files));
    consumer?.({ files: [fakeHandle("launched.pdf").handle] });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(received.map((files) => files.map((f) => f.name))).toEqual([["launched.pdf"]]);
  });
});

describe("saving on the web", () => {
  it("downloads under the suggested name where there is no save dialog", async () => {
    const { document, clicked } = recordingDocument();
    const platform = createWebPlatform(fakeScope({ document }));
    expect(await platform.saveAs(PDF, "copy.pdf")).toEqual({ saved: true, name: "copy.pdf" });
    expect(clicked).toMatchObject([{ href: "blob:fake", download: "copy.pdf" }]);
  });

  it("writes to the file the save dialog chose, under the name the user gave", async () => {
    const { handle, written } = fakeHandle("chosen.pdf");
    const platform = createWebPlatform(chromiumLike({ showSaveFilePicker: async () => handle }));
    expect(await platform.saveAs(PDF, "copy.pdf")).toEqual({ saved: true, name: "chosen.pdf" });
    expect(written).toEqual([PDF]);
  });

  it("reports a dismissed save dialog as cancelled", async () => {
    const platform = createWebPlatform(chromiumLike({ showSaveFilePicker: abort }));
    expect(await platform.saveAs(PDF, "copy.pdf")).toEqual({ saved: false, reason: "cancelled" });
  });

  it("overwrites a picked file in place", async () => {
    const { handle, written } = fakeHandle("picked.pdf");
    const platform = createWebPlatform(chromiumLike({ showOpenFilePicker: async () => [handle] }));
    const [file] = await platform.openFiles({ multiple: false });
    if (file === undefined) throw new Error("no file");
    expect(await platform.saveInPlace(file, PDF)).toEqual({ saved: true, name: "picked.pdf" });
    expect(written).toEqual([PDF]);
  });

  it("refuses to save in place a file that isn't writable, or where nothing is", async () => {
    const { handle } = fakeHandle("picked.pdf");
    const refused = message("platform.error.notWritable");
    await expect(createWebPlatform(chromiumLike()).saveInPlace(fakeFile(), PDF)).rejects.toThrow(refused);
    const readOnly = { ...fakeFile(), handle, writable: true };
    await expect(createWebPlatform(fakeScope()).saveInPlace(readOnly, PDF)).rejects.toThrow(refused);
  });
});

describe("links and the clipboard on the web", () => {
  it("opens web and email links in a new tab without an opener", async () => {
    const opened: [string, string, string][] = [];
    const platform = createWebPlatform(fakeScope({ open: (...args) => void opened.push(args) }));
    await platform.openExternal("https://laminapdf.com/");
    await platform.openExternal("mailto:hello@example.com");
    expect(opened).toEqual([
      ["https://laminapdf.com/", "_blank", "noopener,noreferrer"],
      ["mailto:hello@example.com", "_blank", "noopener,noreferrer"],
    ]);
  });

  it.each(["javascript:alert(1)", "file:///etc/passwd", "data:text/html,hi"])("refuses %s", async (url) => {
    const scope = fakeScope();
    await expect(createWebPlatform(scope).openExternal(url)).rejects.toThrow(message("platform.error.externalScheme"));
    expect(scope.opened).toEqual([]);
  });

  it("writes plain text to the clipboard", async () => {
    const copied: string[] = [];
    const navigator = { clipboard: { writeText: async (text: string) => void copied.push(text) } };
    await createWebPlatform(fakeScope({ navigator: navigator as unknown as Navigator })).copyText("page 3");
    expect(copied).toEqual(["page 3"]);
  });
});

describe("storage and recent files on the web", () => {
  it("has no private file system or persistence where the browser has none", async () => {
    const { storage } = createWebPlatform(fakeScope());
    expect(await storage.opfs()).toBeUndefined();
    expect(await storage.persist()).toBe(false);
  });

  it("keeps small values until they are deleted or cleared", async () => {
    const { storage } = createWebPlatform(fakeScope());
    await storage.set("theme", "dark");
    await storage.set("zoom", 1.5);
    await storage.delete("zoom");
    expect([await storage.get("theme"), await storage.get("zoom")]).toEqual(["dark", undefined]);
    await storage.clear();
    expect(await storage.get("theme")).toBeUndefined();
  });

  it("lists recent documents newest first, once each, and at most twenty", async () => {
    const { recent } = createWebPlatform(fakeScope());
    for (let n = 0; n < 22; n++) await recent.add(fakeFile(`doc-${n}.pdf`));
    await recent.add(fakeFile("doc-5.pdf"));
    const names = (await recent.list()).map((entry) => entry.name);
    expect(names).toHaveLength(20);
    expect(names.slice(0, 3)).toEqual(["doc-5.pdf", "doc-21.pdf", "doc-20.pdf"]);
    expect(names.filter((name) => name === "doc-5.pdf")).toHaveLength(1);
    expect(names).not.toContain("doc-1.pdf");
  });

  it("removes one recent document, or all of them", async () => {
    const { recent } = createWebPlatform(fakeScope());
    await recent.add(fakeFile("a.pdf"));
    await recent.add(fakeFile("b.pdf"));
    const [b] = await recent.list();
    await recent.remove(b?.id ?? "");
    expect((await recent.list()).map((entry) => entry.name)).toEqual(["a.pdf"]);
    await recent.clear();
    expect(await recent.list()).toEqual([]);
  });

  it("can't reopen a recent document where the browser can't keep access to files", () => {
    expect(createWebPlatform(fakeScope()).recent.reopen).toBeUndefined();
  });

  it("reopens a recent document from its kept handle, asking only when access wasn't kept", async () => {
    const platform = createWebPlatform(chromiumLike({ indexedDB: fakeIndexedDb() }));
    const handles = [
      fakeHandle("kept.pdf", PDF, "denied", "granted").handle,
      fakeHandle("allowed.pdf").handle,
      fakeHandle("denied.pdf", PDF, "denied").handle,
    ];
    for (const handle of handles) await platform.recent.add({ ...fakeFile(handle.name), handle });
    const ids = Object.fromEntries((await platform.recent.list()).map((entry) => [entry.name, entry.id]));
    requested.length = 0;
    expect(await platform.recent.reopen?.(ids["kept.pdf"] ?? "")).toMatchObject({ name: "kept.pdf" });
    expect(await platform.recent.reopen?.(ids["allowed.pdf"] ?? "")).toMatchObject({ name: "allowed.pdf" });
    expect(await platform.recent.reopen?.(ids["denied.pdf"] ?? "")).toBeUndefined();
    expect(requested).toEqual(["allowed.pdf", "denied.pdf"]);
  });

  it("forgets a recent document's handle when it is removed or drops off the list", async () => {
    const platform = createWebPlatform(chromiumLike({ indexedDB: fakeIndexedDb() }));
    const { handle } = fakeHandle("first.pdf");
    await platform.recent.add({ ...fakeFile("first.pdf"), handle });
    const [first] = await platform.recent.list();
    for (let n = 0; n < 20; n++) await platform.recent.add({ ...fakeFile(`doc-${n}.pdf`), handle });
    expect(await platform.recent.reopen?.(first?.id ?? "")).toBeUndefined();
    const [latest] = await platform.recent.list();
    await platform.recent.remove(latest?.id ?? "");
    expect(await platform.recent.reopen?.(latest?.id ?? "")).toBeUndefined();
  });
});
