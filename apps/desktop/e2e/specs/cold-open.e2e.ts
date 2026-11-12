// The cold-start scenario, in its own run (wdio.cold-open.conf.ts): the app started with a PDF's
// path opens it once the window is ready, through the queue of requests that came before it. The
// time to the first sharp page, on the webview's clock (from its start, which follows the app's
// own start-up: cold-start.ts's figure), is reported, never asserted (ADR 0018). Being a second
// start of the app, it also checks what the first run left: the recent list kept across the
// restart, cleared from the palette; "Remember recent files" turned off in Settings, which stops
// recording; and the logs, which hold no file's path.
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { fixture, inkedPixels, launchAgain, paintsSince, recentNames, runCommand, waitForDocument } from "../app.ts";
import { output, root } from "../paths.ts";

/** Opens Settings from the More menu and flips "Remember recent files", then closes it. */
async function toggleRememberRecent(): Promise<void> {
  await $('button[aria-label="More"]').click();
  await $('//*[@role="menuitem"][contains(., "Settings")]').click();
  await $("label=Remember recent files").click();
  await browser.keys("Escape");
}

describe("opening from the command line", () => {
  it("should open the file named when the app started", async () => {
    await waitForDocument(2);
    expect(await inkedPixels(0)).toBeGreaterThan(0);
    const sharp = (await paintsSince(0)).find((p) => p.page === 0 && p.state === "sharp");
    const at = sharp === undefined ? "no paint recorded" : `${Math.round(sharp.at)} ms`;
    console.log(`Cold start with a file: first page sharp at ${at} on the webview's clock`);
  });
});

describe("recent files across a restart", () => {
  it("should keep the files the first run opened", async () => {
    // The 1,000-page fixture opened only in the first run.
    expect(await recentNames()).toEqual(expect.arrayContaining(["Relevé de compte 2026.pdf", "1000-pages.pdf"]));
  });

  it("should clear the list, and record nothing, while Remember recent files is off", async () => {
    await toggleRememberRecent();
    await browser.waitUntil(async () => (await recentNames()).length === 0, {
      timeoutMsg: "turning Remember recent files off didn't clear the list",
    });
    // Another file than the one on screen, so the window visibly opens it.
    expect(await launchAgain([fixture("fixtures", "golden", "examples", "hello.pdf")])).toBe(0);
    await waitForDocument(1);
    expect(await recentNames()).toEqual([]);
    await toggleRememberRecent();
  });

  it("should empty the list with the palette's Clear recent", async () => {
    expect(await launchAgain([fixture("fixtures", "golden", "examples", "invoice_from_json.pdf")])).toBe(0);
    await waitForDocument(2);
    await browser.waitUntil(async () => (await recentNames()).length > 0, { timeoutMsg: "the open wasn't recorded" });
    await runCommand("Clear recent");
    await browser.waitUntil(async () => (await recentNames()).length === 0, { timeoutMsg: "the list wasn't cleared" });
  });
});

describe("the logs", () => {
  it("should hold no file's path at info level or above", () => {
    // tauri-plugin-log writes info and above; each run's backend log is in the output folder.
    const lines = readdirSync(output)
      .filter((name) => name.startsWith("wdio-") && name.endsWith(".log"))
      .flatMap((name) => readFileSync(join(output, name), "utf8").split("\n"))
      .filter((line) => line.includes("[Tauri:Backend"));
    expect(lines.length).toBeGreaterThan(0);
    // The repository's path as it would be written: as is, with Windows' backslashes doubled by
    // Debug formatting, or with forward slashes.
    const forms = [root, root.replaceAll("\\", "\\\\"), root.replaceAll("\\", "/")];
    expect(lines.filter((line) => forms.some((form) => line.includes(form)))).toEqual([]);
  });
});
