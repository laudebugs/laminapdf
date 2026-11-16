// Crash reports (D7a), in their own launch (wdio.crash.conf.ts): with reporting off, as it starts,
// a crash of the document process sends nothing and asks once, in the window's notice; View report
// shows the report itself. Turned on in Settings › Privacy, each crash (the engine's panic, its
// segmentation fault, the page's own error) reaches the Worker's stand-in as a report with no
// path, no marker of the open file's folder, name or content, and no install ID. The segmentation
// fault is a real one, a write through a null pointer in the engine (D7d): on every OS its
// report's native stack, read from its minidump on this computer, names the engine as the faulting
// module.
import { platform } from "node:process";
import { expect } from "@wdio/globals";
import { invoke, open, pageSizes, restarts, withText } from "../app.ts";
import { CONTENT_MARKER, FOLDER_MARKER, markedFile, NAME_MARKER, type Received, requests } from "../crash.ts";

/** The reports the stand-in has, once there are `count`. */
async function reportsOnceThere(count: number): Promise<Record<string, unknown>[]> {
  await browser.waitUntil(async () => requests().length >= count, {
    timeout: 30_000,
    timeoutMsg: `the stand-in didn't get ${count} report(s)`,
  });
  return requests().map((request) => JSON.parse(request.body) as Record<string, unknown>);
}

/** Whether the main process sends crash reports. */
async function sendsReports(): Promise<boolean> {
  return ((await invoke("crash_settings")) as { enabled: boolean }).enabled;
}

/** Forces a crash with `command` (and its `args`) and waits until the document process restarted. */
async function crashAndWait(command: string, args: object = {}): Promise<void> {
  const before = await restarts();
  await invoke(command, args);
  await browser.waitUntil(async () => (await restarts()) > before, {
    timeout: 20_000,
    timeoutMsg: `the document process didn't restart after ${command}`,
  });
}

/** The engine's file name, as a native stack names its module. */
const ENGINE_MODULE = `laminapdf-desktop-engine${platform === "win32" ? ".exe" : ""}`;

/** A native crash's stack, as a report carries it (D7d). */
interface NativeStack {
  readonly crash_reason: string;
  readonly frames: readonly { readonly module: string; readonly offset: number; readonly function: null }[];
  readonly modules: readonly { readonly name: string; readonly debug_id: string }[];
}

/** Whether `request` holds anything of the open file, any path, or the install ID's header. */
function leaks(request: Received): string[] {
  const found = [FOLDER_MARKER, NAME_MARKER, CONTENT_MARKER, "/Users/", "/home/", "\\\\", ":\\"].filter((marker) =>
    request.body.includes(marker),
  );
  if (Object.keys(request.headers).some((name) => name.toLowerCase() === "lamina-install-id")) found.push("install ID");
  return found;
}

describe("crash reports", () => {
  it("should send nothing while off, and ask once in the notice, showing the report", async () => {
    // marker-text.pdf's text uses fonts made of the marker, so its pages draw blank: wait until
    // the engine has opened it (its two pages measured), not for a drawn page. The run's first
    // open is in a cold app, where the engine starts and the fonts are scanned first.
    await open(markedFile);
    await browser.waitUntil(async () => (await pageSizes().catch(() => [])).length === 2, {
      timeout: 30_000,
      timeoutMsg: "the engine didn't open the marked file",
    });
    await crashAndWait("e2e_crash_engine_segv");
    await expect(withText("The document view restarted after a problem")).toBeDisplayed();
    await $("button=View report").click();
    const report = $('[role="region"][aria-label="Crash report"]');
    await expect(report).toHaveText(expect.stringContaining('"process": "engine"'));
    // The report shown is the one that would be sent; its native stack names the engine.
    await expect(report).toHaveText(expect.stringContaining(`"module": "${ENGINE_MODULE}"`));
    await $("button=Don't send").click();
    await expect(report).not.toBeExisting();
    // A moment for anything that would go out.
    await browser.pause(1500);
    expect(requests()).toEqual([]);
  });

  it("should send each crash once turned on, with nothing of the file in it", async () => {
    await $('button[aria-label="More"]').click();
    await $('//*[@role="menuitem"][contains(., "Settings")]').click();
    await $('//button[.//span[@class="paper-settings-nav-name" and text()="Privacy"]]').click();
    await $("button=See a sample report").click();
    await expect($('[aria-label="Sample crash report"]')).toHaveText(expect.stringContaining('"schema": 1'));
    // Clicked in the page, as toggleSetting does (WebKit's driver can't click the hidden input),
    // and only while off, so a retry doesn't turn it off again; the main process says when it is on.
    if (!(await sendsReports())) {
      await browser.execute(() => {
        const label = [...globalThis.document.querySelectorAll("label")].find((l) =>
          l.textContent?.includes("Send crash reports"),
        );
        label?.querySelector<HTMLInputElement>('input[role="switch"]')?.click();
      });
    }
    await browser.waitUntil(sendsReports, { timeoutMsg: "Send crash reports didn't turn on" });
    await browser.keys("Escape");

    // The panic's message names the open file, as a careless format! would: the report must not.
    await crashAndWait("e2e_crash_engine_panic", { message: `could not read ${markedFile}` });
    await crashAndWait("e2e_crash_engine_segv");
    await browser.execute((path) => {
      setTimeout(() => {
        throw new Error(`could not draw ${path}`);
      });
    }, markedFile);
    const reports = await reportsOnceThere(3);
    expect(reports.map((report) => [report.process, report.kind])).toEqual([
      ["engine", "panic"],
      ["engine", "signal"],
      ["webview", "error"],
    ]);
    expectNativeStack(reports[1]?.native as NativeStack | undefined);
    expect(requests().flatMap(leaks)).toEqual([]);
  });
});

/** The segfault's report names the engine as its faulting module. */
function expectNativeStack(stack: NativeStack | undefined): void {
  expect(stack?.frames[0]?.module).toBe(ENGINE_MODULE);
  expect(stack?.modules.find((module) => module.name === ENGINE_MODULE)?.debug_id).toMatch(/^[0-9A-Fa-f]{32,40}$/);
  expect(stack?.crash_reason).not.toBe("");
}
