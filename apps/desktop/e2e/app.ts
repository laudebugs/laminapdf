// The desktop reader as the scenarios see it: open a fixture, read the view, wait for paint.
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { $, browser } from "@wdio/globals";
import { Key } from "webdriverio";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..", "..");

/** A page's render state changing, at `performance.now()` in the app's window. */
export interface Paint {
  readonly page: number;
  readonly state: "placeholder" | "low" | "sharp" | "error";
  readonly at: number;
}

/** A file under the repository (`fixtures/`, or `target/fixtures/` for generated ones). */
export function fixture(...parts: string[]): string {
  return join(root, ...parts);
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
 */
export async function open(path: string): Promise<number> {
  await invoke("e2e_queue_pick", { path });
  const since = await now();
  const choose = $("button=Choose a file");
  if (await choose.isExisting()) await choose.click();
  else await browser.keys([process.platform === "darwin" ? Key.Command : Key.Ctrl, "o"]);
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

/** Kills this window's engine as a crash would. */
export async function killEngine(): Promise<void> {
  await invoke("debug_kill_engine");
}

/** `performance.now()` in the app's window. */
export async function now(): Promise<number> {
  return browser.execute(() => performance.now());
}

/** Every change of a page's render state since `since`. */
export async function paintsSince(since: number): Promise<readonly Paint[]> {
  return browser.execute((from) => globalThis.__laminaE2e?.paintsSince(from) ?? [], since);
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

/** The view's page count, current page (0-based) and zoom, as the viewer reports them. */
export async function view(): Promise<{ pageCount: number; currentPage: number; percent: number; scrollTop: number }> {
  const state = await browser.execute(() => {
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

interface TauriGlobal {
  readonly __TAURI__: { readonly core: { invoke(command: string, args: object): Promise<unknown> } };
}
