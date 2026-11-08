// Encrypted files (the standard security handler, R2 to R6; fixtures/encrypted/ and the corpus's
// enc.pdf): a file with an empty user password opens without asking; one with a user password
// asks, says when a password is wrong, opens with the user or the owner password, and goes back
// home on Cancel. A remote encrypted file asks too.

import { readFileSync } from "node:fs";
import type { Browser } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { load as loadPage, type Watched, WITHOUT_PICKER } from "./reader.ts";
import { type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const ENCRYPTED = new URL("../../../fixtures/encrypted/", import.meta.url);
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);

/** Every R2–R6 fixture with its passwords (fixtures/encrypted/passwords.json). */
const FILES = Object.entries(
  (
    JSON.parse(readFileSync(new URL("passwords.json", ENCRYPTED), "utf8")) as {
      files: Record<string, { user: string; owner: string }>;
    }
  ).files,
).filter(([name]) => /^r[2-6]-/.test(name));

describe(`encrypted files in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;

  beforeAll(async () => {
    browser = await launch();
    host = await startRemoteHost();
    host.add("locked.pdf", readFileSync(new URL("r4-aes-128-user.pdf", ENCRYPTED)));
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  const load = (path = "/", home = true): Promise<Watched> =>
    loadPage(browser, origin, path, WITHOUT_PICKER, { ignoreHTTPSErrors: true, home });

  /** Chooses `file` with Choose a file. */
  async function choose({ page }: Watched, file: URL): Promise<void> {
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(file.pathname);
  }

  /** Types `password` into the password dialog and presses Open. */
  async function enter({ page }: Watched, password: string): Promise<void> {
    const dialog = page.getByRole("dialog", { name: "Enter the password" });
    await dialog.waitFor();
    await dialog.getByLabel("Password", { exact: true }).fill(password);
    await dialog.getByRole("button", { name: "Open" }).click();
  }

  /** Waits for the first page and the window title from the file's decrypted metadata. */
  async function decrypted({ page }: Watched): Promise<void> {
    await page.locator(".web-viewer canvas").first().waitFor();
    await expect.poll(() => page.title()).toBe("Secret document · Lamina PDF");
  }

  async function clean(watched: Watched): Promise<void> {
    expect(await watched.violations()).toEqual([]);
    expect(watched.foreign.filter((url) => new URL(url).origin !== host.origin)).toEqual([]);
    await watched.page.close();
  }

  it.each(FILES.filter(([, p]) => p.user === ""))("%s opens without a password", async (name) => {
    const watched = await load();
    await choose(watched, new URL(name, ENCRYPTED));
    await decrypted(watched);
    await clean(watched);
  });

  it.each(FILES.filter(([, p]) => p.user !== ""))(
    "%s asks for its password, says when it is wrong and opens with the user password",
    async (name, { user }) => {
      const watched = await load();
      await choose(watched, new URL(name, ENCRYPTED));
      await enter(watched, "not-the-password");
      await watched.page.getByText("That password didn't open the file.").waitFor();
      await enter(watched, user);
      await decrypted(watched);
      await clean(watched);
    },
  );

  it.each(FILES.filter(([, p]) => p.user !== "" && p.user !== p.owner))(
    "%s opens with the owner password",
    async (name, { owner }) => {
      const watched = await load();
      await choose(watched, new URL(name, ENCRYPTED));
      await enter(watched, owner);
      await decrypted(watched);
      await clean(watched);
    },
  );

  it("goes back home when the password prompt is cancelled", async () => {
    const watched = await load();
    await choose(watched, new URL("r6-aes-256-user.pdf", ENCRYPTED));
    const dialog = watched.page.getByRole("dialog", { name: "Enter the password" });
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await watched.page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    await clean(watched);
  });

  it("opens the corpus's enc.pdf, whose user password is empty", async () => {
    const watched = await load();
    await choose(watched, new URL("enc.pdf", CORPUS));
    await watched.page.locator(".web-viewer canvas").first().waitFor();
    await clean(watched);
  });

  it("asks for a remote file's password and opens it", async () => {
    const watched = await load(`/#url=${encodeURIComponent(`${host.origin}/cors/ranges/locked.pdf`)}`, false);
    await enter(watched, "user-pw");
    await decrypted(watched);
    await clean(watched);
  });
});
