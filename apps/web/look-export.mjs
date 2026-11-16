// One-off (not committed): screenshots of Export in the built reader.
import { chromium } from "playwright";

const out = process.argv[2];
const b = await chromium.launch();
const p = await b.newPage({ viewport: { width: 1280, height: 860 }, deviceScaleFactor: 2 });
await p.addInitScript(() => Reflect.deleteProperty(window, "showOpenFilePicker"));
await p.goto("http://localhost:8788/");
const ch = p.waitForEvent("filechooser");
await p.getByRole("button", { name: "Choose a file" }).first().click();
await (await ch).setFiles(new URL("../../fixtures/reading-order/header-footer-ordered.pdf", import.meta.url).pathname);
await p.locator(".web-viewer canvas").first().waitFor({ timeout: 60000 });
await p.getByRole("button", { name: "More", exact: true }).first().click();
await p.screenshot({ path: `${out}/more-menu.png` });
await p.getByRole("menuitem", { name: "Export…" }).click();
const d = p.getByRole("dialog", { name: "Export" });
await d.waitFor();
await p.waitForTimeout(1500);
await d.screenshot({ path: `${out}/dialog-pages.png` });
await d.getByText("Text", { exact: true }).first().click();
await d.getByText("Markdown", { exact: true }).first().click();
await p.waitForTimeout(300);
await d.screenshot({ path: `${out}/dialog-text-markdown.png` });
await b.close();
