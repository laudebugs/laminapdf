// The desktop reader as the scenarios see it: open a fixture, read the view, wait for paint.
import { spawn } from "node:child_process";
import { renameSync } from "node:fs";
import { join } from "node:path";
import { platform } from "node:process";
import type { ViewState } from "@laminapdf/viewer";
import { $, browser } from "@wdio/globals";
import { type ChainablePromiseElement, Key } from "webdriverio";
import { appBinary, root } from "./paths.ts";
import type { Paint } from "./probe/types.ts";

/** What the scenarios check of the view. */
export type View = Pick<ViewState, "pageCount" | "currentPage" | "percent" | "scrollTop">;

/** A file under the repository (`fixtures/`, or `target/fixtures/` for generated ones). */
export function fixture(...parts: string[]): string {
  return join(root, ...parts);
}

/**
 * The innermost element whose text contains `text`. (WebdriverIO's `*=` matches link text only.)
 */
export function withText(text: string): ChainablePromiseElement {
  return $(`//*[contains(text(), ${JSON.stringify(text)})]`);
}

/** Runs one of the app's commands as the webview would. */
async function invoke(command: string, args: object = {}): Promise<unknown> {
  return browser.execute(
    (name, payload) => (globalThis as unknown as TauriGlobal).__TAURI__.core.invoke(name, payload),
    command,
    args,
  );
}

/**
 * Opens `path` as a user does: Choose a file on the home screen, else the open shortcut (⌘O or
 * Ctrl+O). The e2e build opens the queued path instead of showing the dialog.
 * @returns When the open began, on the app's clock.
 */
export async function open(path: string): Promise<number> {
  await invoke("e2e_queue_pick", { path });
  const since = await now();
  const choose = $("button=Choose a file");
  if (await choose.isExisting()) await choose.click();
  else await browser.keys([platform === "darwin" ? Key.Command : Key.Ctrl, "o"]);
  return since;
}

/**
 * Opens `path` and waits until its first page is drawn sharp: a page drawn after the open, so the
 * document before it can't answer for it.
 * @returns When the open began, on the app's clock.
 */
export async function openAndWait(path: string, timeout = 15_000): Promise<number> {
  const since = await open(path);
  await waitForSharpSince(0, since, timeout);
  return since;
}

/**
 * Kills this window's engine as a crash would, and waits until a new one has reopened the
 * document. (A toast from an earlier restart may still be showing, so the scenarios can't wait
 * on the toast alone.)
 */
export async function killEngine(): Promise<void> {
  const before = await restarts();
  await invoke("debug_kill_engine");
  await browser.waitUntil(async () => (await restarts()) > before, {
    timeout: 20_000,
    timeoutMsg: "the engine didn't restart and reopen the document",
  });
}

/** `performance.now()` in the app's window. */
async function now(): Promise<number> {
  return browser.execute(() => performance.now());
}

/** Every change of a page's render state since `since`. */
export async function paintsSince(since: number): Promise<readonly Paint[]> {
  return browser.execute((from) => globalThis.__laminaE2e?.paintsSince(from) ?? [], since);
}

/** Pixels of page `index` (0-based) that aren't blank paper, on the probe's sample grid. */
export async function inkedPixels(index: number): Promise<number> {
  return browser.execute((page) => globalThis.__laminaE2e?.inkedPixels(page) ?? 0, index);
}

/** Scrolls the view to its end. */
export async function scrollToEnd(): Promise<void> {
  await browser.execute(() => globalThis.__laminaE2e?.scrollToEnd());
}

/** Waits until page `index` (0-based) is drawn sharp. */
export async function waitForSharp(index: number, timeout = 15_000): Promise<void> {
  await browser.waitUntil(
    async () => (await browser.execute((page) => globalThis.__laminaE2e?.pages()[page], index)) === "sharp",
    { timeout, timeoutMsg: `page ${index + 1} was never drawn sharp` },
  );
}

/** Waits until page `index` is drawn sharp again after `since`, and returns when (the app's clock). */
export async function waitForSharpSince(index: number, since: number, timeout = 15_000): Promise<number> {
  let at = 0;
  await browser.waitUntil(
    async () => {
      const paint = (await paintsSince(since)).find((p) => p.page === index && p.state === "sharp");
      at = paint?.at ?? 0;
      return paint !== undefined;
    },
    { timeout, timeoutMsg: `page ${index + 1} wasn't drawn again` },
  );
  return at;
}

/** Clicks the top bar's `label` zoom button and waits until the view's zoom changes. */
export async function zoomOnce(label: "Zoom in" | "Zoom out"): Promise<void> {
  const { percent } = await view();
  await $(`button[aria-label="${label}"]`).click();
  await browser.waitUntil(async () => (await view()).percent !== percent, { timeoutMsg: `${label} didn't zoom` });
}

/**
 * Zooms out once and waits for the engine to answer new tiles and the current page to be sharp:
 * after a kill, only a restarted engine can. (After a restart that reopened the same file the
 * viewer keeps the pictures it has, so a page already drawn isn't drawn again by itself.)
 */
export async function drawsAfterZoomOut(): Promise<void> {
  const { currentPage } = await view();
  const before = await tilesDrawn();
  await zoomOnce("Zoom out");
  await browser.waitUntil(async () => (await tilesDrawn()) > before, { timeoutMsg: "the engine drew no tile" });
  await waitForSharp(currentPage);
}

/** The view's page count, current page (0-based), zoom and scroll, as the viewer reports them. */
export async function view(): Promise<View> {
  const state = await browser.execute((): View | undefined => {
    const v = globalThis.__laminaE2e?.view();
    return v === undefined
      ? undefined
      : { pageCount: v.pageCount, currentPage: v.currentPage, percent: v.percent, scrollTop: v.scrollTop };
  });
  if (state === undefined) throw new Error("no view: the e2e probe isn't loaded");
  return state;
}

/** Goes to page `number` (1-based) through the top bar's page field. */
export async function goToPage(number: number): Promise<void> {
  const field = $('input[aria-label="Page"]');
  await field.click();
  await field.setValue(String(number));
  await browser.keys("Enter");
}

async function tilesDrawn(): Promise<number> {
  return browser.execute(() => globalThis.__laminaE2e?.tilesDrawn() ?? 0);
}

async function restarts(): Promise<number> {
  return browser.execute(() => globalThis.__laminaE2e?.restarts() ?? 0);
}

interface TauriGlobal {
  readonly __TAURI__: { readonly core: { invoke(command: string, args: object): Promise<unknown> } };
}

/**
 * Starts the app's binary a second time with `args`, as a double-click or "Open with" does on
 * Windows and Linux, and resolves with its exit code: the single-instance plugin hands the
 * arguments to the running app, and the second process exits at once.
 */
export function launchAgain(args: readonly string[]): Promise<number | null> {
  return new Promise((resolve, reject) => {
    const second = spawn(appBinary, args, { stdio: "ignore" });
    const timer = setTimeout(() => {
      second.kill();
      reject(new Error("the second launch didn't exit within 20 s: it may have opened a second app"));
    }, 20_000);
    second.once("error", reject);
    second.once("exit", (code) => {
      clearTimeout(timer);
      resolve(code);
    });
  });
}

/** Waits until the window shows a document of `pages` pages with its first page drawn sharp. */
export async function waitForDocument(pages: number, timeout = 20_000): Promise<void> {
  await browser.waitUntil(async () => (await view()).pageCount === pages, {
    timeout,
    timeoutMsg: `no ${pages}-page document opened`,
  });
  await waitForSharp(0, timeout);
}

/** The names in the recent files list, newest first, as the main process keeps it. */
export async function recentNames(): Promise<string[]> {
  const entries = (await invoke("recent_list")) as { name: string }[];
  return entries.map((entry) => entry.name);
}

/** Runs the command palette's command named `title`: ⌘K or Ctrl+K, its title, its row. */
export async function runCommand(title: string): Promise<void> {
  await browser.keys([platform === "darwin" ? Key.Command : Key.Ctrl, "k"]);
  const input = $('input[role="combobox"]');
  await input.waitForExist();
  await input.setValue(title);
  // Enter runs the active row, which typing doesn't choose (↓ does), so click the command's row.
  await $(`//*[@role="option"][normalize-space(.) = "${title}" or .//*[normalize-space(.) = "${title}"]]`).click();
}

/**
 * Moves `from` over `to`, as an editor's atomic save does. On Windows the app does it
 * (`e2e_replace`, Rust's POSIX-semantics rename): Node's rename uses `MoveFileEx`, which Windows
 * refuses over a file another process holds open, as the main process holds a document (ADR 0008
 * §11).
 */
export async function atomicReplace(from: string, to: string): Promise<void> {
  if (platform === "win32") await invoke("e2e_replace", { from, to });
  else renameSync(from, to);
}

/** The window's tabs' names, in order; the strip shows from two tabs on. */
export async function tabNames(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(globalThis.document.querySelectorAll('[role="tab"]'), (tab) => tab.textContent ?? ""),
  );
}

/** The shown tab's name, if the strip shows. */
export async function shownTab(): Promise<string | undefined> {
  const tab = $('[role="tab"][aria-selected="true"]');
  return (await tab.isExisting()) ? tab.getText() : undefined;
}

/**
 * Closes every tab with Cmd+W or Ctrl+W until the start screen shows, so a scenario starts as if
 * nothing were open (the scenarios share one app, and opening no longer replaces a document).
 */
export async function closeAll(): Promise<void> {
  await browser.keys("Escape");
  for (let presses = 0; presses < 40; presses++) {
    if (await $("button=Choose a file").isExisting()) return;
    await browser.keys([platform === "darwin" ? Key.Command : Key.Ctrl, "w"]);
    await browser.pause(150);
  }
  throw new Error("the window didn't go back to its start screen");
}
