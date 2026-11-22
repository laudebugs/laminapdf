// Draft screenshots for the Mac App Store listing (docs/guidelines/mac-app-store.md §5):
//
//   cargo xtask web build
//   node tools/desktop/app-store-screenshots.mjs <out-dir> [--only <scene>] [--dist <dir>]
//
// It serves the built web reader (apps/web/dist, or --dist) on a local port, opens each scene's
// fixture in Chromium and saves one PNG per scene and size: 2880x1800 and 1440x900, both from a
// 1440x900 window (the larger at a device scale of 2), so the layout is the same in both. Nothing
// is uploaded and nothing is written under the repository: <out-dir> must lie outside it. The
// images show the reader's UI in a browser, not the store build; retake them from the real app
// on a Mac before submitting (Apple's guideline 2.3.3 asks for screenshots of the app in use).
// The scene list below is also the shot list to follow on the Mac.

import { createReadStream, existsSync, mkdirSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { dirname, extname, isAbsolute, join, normalize, relative, resolve } from "node:path";
import { argv, exit } from "node:process";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
// Playwright is a dev dependency of the reader (apps/web), not of the repository's root.
const { chromium } = createRequire(join(ROOT, "apps/web/package.json"))("playwright");

const WINDOW = { width: 1440, height: 900 };
/** The sizes Apple takes for a Mac screenshot (16:10), as a device scale of the window. */
const SIZES = [
  { name: "2880x1800", scale: 2 },
  { name: "1440x900", scale: 1 },
];
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm" };

const fixture = (path) => join(ROOT, "fixtures", path);

/** Opens `file` with the file input, the way the reader does without a file picker. */
async function open(page, file) {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 60_000 });
  // Let the first pages finish drawing.
  await page.waitForTimeout(1500);
}

/** The scenes, in the listing's order: a caption (the listing's words) and what is shown. */
const SCENES = [
  {
    name: "01-read",
    caption: "Read PDFs in tabs and windows, on your own Mac",
    shows: "A tagged report open in the reader, with its pages beside it",
    file: "golden/tagged-report.pdf",
    async run(page) {
      await page
        .getByRole("button", { name: /sidebar/i })
        .first()
        .click();
      await page.waitForTimeout(1200);
    },
  },
  {
    name: "02-find",
    caption: "Find any word in a document",
    shows: "Find open with a word typed and its matches highlighted",
    file: "golden/links-and-outlines.pdf",
    async run(page) {
      await page.keyboard.press("ControlOrMeta+F");
      await page.getByRole("textbox", { name: "Find in document" }).fill("chapter");
      await page.waitForTimeout(800);
    },
  },
  {
    name: "03-outline",
    caption: "Jump to any heading from the outline",
    shows: "A document with its outline open in the sidebar",
    file: "golden/links-and-outlines.pdf",
  },
  {
    name: "04-organise",
    caption: "Reorder, rotate and delete pages",
    shows: "Organise mode over a document's pages",
    file: "golden/tagged-across-pages.pdf",
    async run(page) {
      await page.getByRole("button", { name: "Organise", exact: true }).click();
      // Bigger thumbnails fill the window better.
      // The grid's own zoom is the "+" at the toolbar's right end.
      for (let i = 0; i < 4; i++) await page.mouse.click(WINDOW.width - 25, 61);
      await page.waitForTimeout(1200);
    },
  },
  {
    name: "05-properties",
    caption: "Edit a document's title, author and keywords",
    shows: "The Document properties form over the page",
    file: "golden/metadata.pdf",
    async run(page) {
      await page.getByRole("button", { name: "More" }).click();
      await page.getByRole("menuitem", { name: "Document properties…" }).click();
      await page.getByRole("dialog", { name: "Document properties" }).getByLabel("Title").waitFor();
    },
  },
  {
    name: "06-dark",
    caption: "Light and dark themes",
    shows: "The same reader in its dark theme, over a multilingual document",
    file: "golden/examples/multilingual.pdf",
    theme: "dark",
  },
];

/** Serves `dist` over http on a free local port, answering unknown paths with index.html. */
function serve(dist) {
  const server = createServer((request, response) => {
    const path = normalize(decodeURIComponent(new URL(request.url, "http://x").pathname)).replace(/^(\.\.[/\\])+/, "");
    let file = join(dist, path);
    if (!file.startsWith(dist) || !existsSync(file) || statSync(file).isDirectory()) file = join(dist, "index.html");
    response.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
    createReadStream(file).pipe(response);
  });
  return new Promise((done) => server.listen(0, "127.0.0.1", () => done(server)));
}

function usage(message) {
  console.error(
    `${message}\nusage: node tools/desktop/app-store-screenshots.mjs <out-dir> [--only <scene>] [--dist <dir>]`,
  );
  exit(2);
}

const args = argv.slice(2);
const flag = (name) => {
  const at = args.indexOf(name);
  return at === -1 ? undefined : args.splice(at, 2)[1];
};
const only = flag("--only");
const dist = resolve(flag("--dist") ?? join(ROOT, "apps/web/dist"));
const [outArg, ...rest] = args;
if (outArg === undefined || rest.length > 0) usage("one output folder is needed");
const out = resolve(outArg);
const inRepo = relative(ROOT, out);
if (inRepo === "" || (!inRepo.startsWith("..") && !isAbsolute(inRepo)))
  usage("the output folder must be outside the repository: the images are not committed");
if (!existsSync(join(dist, "index.html"))) usage(`${dist}/index.html is missing: run \`cargo xtask web build\` first`);
const scenes = SCENES.filter((scene) => only === undefined || scene.name === only);
if (scenes.length === 0) usage(`no scene named ${only}: ${SCENES.map((scene) => scene.name).join(", ")}`);

mkdirSync(out, { recursive: true });
const server = await serve(dist);
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch();
try {
  for (const size of SIZES) {
    for (const scene of scenes) {
      const context = await browser.newContext({
        viewport: WINDOW,
        deviceScaleFactor: size.scale,
        colorScheme: scene.theme === "dark" ? "dark" : "light",
        serviceWorkers: "block",
      });
      const page = await context.newPage();
      await page.addInitScript((theme) => {
        // No file pickers, so the file input takes the fixture; the theme is the scene's.
        Reflect.deleteProperty(globalThis, "showOpenFilePicker");
        Reflect.deleteProperty(globalThis, "showSaveFilePicker");
        globalThis.localStorage.setItem("lamina.appearance", JSON.stringify({ theme }));
        globalThis.localStorage.setItem("lamina.comment.author", "Ada");
      }, scene.theme ?? "light");
      await page.goto(origin);
      await page.getByRole("heading", { name: "Open a PDF" }).waitFor({ timeout: 60_000 });
      await open(page, fixture(scene.file));
      if (scene.run) await scene.run(page);
      const path = join(out, `${scene.name}-${size.name}.png`);
      await page.screenshot({ path });
      console.log(`${path}  ${scene.caption}`);
      await context.close();
    }
  }
} finally {
  await browser.close();
  server.close();
}
