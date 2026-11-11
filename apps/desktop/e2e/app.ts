// The desktop reader as the scenarios see it: open a fixture, read the view, wait for paint.
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { $, browser } from "@wdio/globals";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..", "..");

/** A file under the repository's `fixtures/` (or `target/fixtures/` for generated ones). */
export function fixture(...parts: string[]): string {
  return join(root, ...parts);
}

/** Opens `path` through Choose a file: the e2e build opens the queued path instead of the dialog. */
export async function open(path: string): Promise<void> {
  await browser.execute(
    (file) => (globalThis as unknown as TauriGlobal).__TAURI__.core.invoke("e2e_queue_pick", { path: file }),
    path,
  );
  await $("button=Choose a file").click();
}

/** Waits until page `index` (0-based) is drawn sharp. */
export async function waitForSharp(index: number, timeout = 15_000): Promise<void> {
  await browser.waitUntil(
    async () => (await browser.execute((page) => globalThis.__laminaE2e?.pages()[page], index)) === "sharp",
    { timeout, timeoutMsg: `page ${index + 1} was never drawn sharp` },
  );
}

/** The view's page count and current page (0-based), as the viewer reports them. */
export async function view(): Promise<{ pageCount: number; currentPage: number; percent: number }> {
  const state = await browser.execute(() => {
    const v = globalThis.__laminaE2e?.view();
    return v === undefined ? undefined : { pageCount: v.pageCount, currentPage: v.currentPage, percent: v.percent };
  });
  if (state === undefined) throw new Error("no view: the e2e probe isn't loaded");
  return state;
}

interface TauriGlobal {
  readonly __TAURI__: { readonly core: { invoke(command: string, args: object): Promise<unknown> } };
}
