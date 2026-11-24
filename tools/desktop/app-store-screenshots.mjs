// Draft screenshots for the Mac App Store listing (docs/guidelines/mac-app-store.md §5):
//
//   cargo xtask web build
//   node tools/desktop/app-store-screenshots.mjs <out-dir> [--only <scene>] [--dist <dir>]
//
// It serves the built web reader (apps/web/dist, or --dist) on a local port, opens each scene's
// fixture in Chromium and saves one PNG per scene and size: 2880x1800 and 1440x900, both from a
// 1440x900 window (the larger at a device scale of 2), so the layout is the same in both. Nothing
// is uploaded and nothing is written under the repository: <out-dir> must lie outside it. The run
// refuses every request that leaves the local server and fails if one was made, and it fails on an
// image that isn't an RGB PNG of the right size (Apple takes no alpha channel). The images show the
// reader's UI in a browser, not the store build; retake them from the real app on a Mac before
// submitting (Apple's guideline 2.3.3 asks for screenshots of the app in use). The scene list below
// is also the shot list to follow on the Mac.

import { createReadStream, existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { dirname, extname, isAbsolute, join, relative, resolve } from "node:path";
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
const WAIT = 60_000;

const fixture = (path) => join(ROOT, "fixtures", path);

/** Waits for the viewer's first page to be drawn sharp (not the blurry first pass) and the page to settle. */
async function drawn(page) {
  await page.locator('.lamina-page[data-render="sharp"]').first().waitFor({ timeout: WAIT });
  await page.evaluate(
    () =>
      new Promise((done) =>
        globalThis.document.fonts.ready.then(() => requestAnimationFrame(() => requestAnimationFrame(() => done()))),
      ),
  );
}

/** Opens `file` with the file input, the way the reader does without a file picker. */
async function open(page, file) {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file);
  await drawn(page);
}

/** Shows the sidebar if it is closed, then selects its tab `name`. */
async function sidebarTab(page, name) {
  const tab = page.getByRole("tab", { name, exact: true });
  if ((await tab.count()) === 0) {
    await page
      .getByRole("button", { name: /sidebar/i })
      .first()
      .click();
  }
  await tab.waitFor({ timeout: WAIT });
  await tab.click();
  await page.locator('[role="tab"][aria-selected="true"]').filter({ hasText: name }).waitFor({ timeout: WAIT });
}

/** The scenes, in the listing's order: a caption (the listing's words) and what is shown. */
const SCENES = [
  {
    name: "01-read",
    caption: "Read PDFs in tabs and windows, on your own Mac",
    shows: "A tagged report open in the reader, with its pages beside it",
    file: "golden/tagged-report.pdf",
    async run(page) {
      await sidebarTab(page, "Pages");
      // The report's two pages, drawn small in the sidebar.
      await page.waitForFunction(
        () => globalThis.document.querySelectorAll('[role="tabpanel"] canvas, [role="tabpanel"] img').length >= 2,
      );
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
      await page
        .locator(".paper-find-count")
        .filter({ hasText: /1 of 4/ })
        .waitFor({ timeout: WAIT });
    },
  },
  {
    name: "03-outline",
    caption: "Jump to any heading from the outline",
    shows: "A document with its outline open in the sidebar",
    file: "golden/links-and-outlines.pdf",
    async run(page) {
      await sidebarTab(page, "Outline");
      await page.getByText("Chapter 2", { exact: true }).first().waitFor({ timeout: WAIT });
    },
  },
  {
    name: "04-organise",
    caption: "Reorder, rotate and delete pages",
    shows: "Organise mode over a document's pages",
    file: "golden/tagged-across-pages.pdf",
    async run(page) {
      await page.getByRole("button", { name: "Organise", exact: true }).click();
      await page.getByText("3 pages", { exact: true }).waitFor({ timeout: WAIT });
      // Bigger thumbnails fill the window better, until the control reaches its end.
      const larger = page.getByRole("button", { name: "Larger thumbnails" });
      for (let i = 0; i < 4 && (await larger.isEnabled()); i++) await larger.click();
      await page.getByText("3", { exact: true }).last().waitFor({ timeout: WAIT });
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
      const title = page.getByRole("dialog", { name: "Document properties" }).getByLabel("Title");
      await title.waitFor({ timeout: WAIT });
      await page.waitForFunction(() => globalThis.document.querySelector('[role="dialog"] input')?.value !== "");
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
    let file = join(dist, "index.html");
    try {
      const wanted = resolve(dist, `.${decodeURIComponent(new URL(request.url, "http://x").pathname)}`);
      const inside = relative(dist, wanted);
      if (
        inside !== "" &&
        !inside.startsWith("..") &&
        !isAbsolute(inside) &&
        existsSync(wanted) &&
        statSync(wanted).isFile()
      )
        file = wanted;
    } catch {
      response.writeHead(400).end();
      return;
    }
    const stream = createReadStream(file);
    stream.on("error", () => response.destroy());
    response.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
    stream.pipe(response);
  });
  return new Promise((done) => server.listen(0, "127.0.0.1", () => done(server)));
}

function usage(message) {
  console.error(
    `${message}\nusage: node tools/desktop/app-store-screenshots.mjs <out-dir> [--only <scene>] [--dist <dir>]`,
  );
  exit(2);
}

/** Takes the value of option `name` out of `args`: once at most, and never without a value. */
function option(args, name) {
  const at = args.indexOf(name);
  if (at === -1) return undefined;
  const value = args[at + 1];
  if (value === undefined || value.startsWith("--")) usage(`${name} needs a value`);
  args.splice(at, 2);
  if (args.includes(name)) usage(`${name} was given more than once`);
  return value;
}

/** Fails unless the PNG at `path` is `width` x `height` pixels in RGB, with no alpha channel. */
function checkPng(path, width, height) {
  const bytes = readFileSync(path);
  const isPng = bytes.length > 26 && bytes.readUInt32BE(0) === 0x89504e47;
  if (!isPng) throw new Error(`${path} is not a PNG`);
  // After the 8-byte signature and the IHDR chunk's length and name: width, height, bit depth, colour type.
  const [w, h, colour] = [bytes.readUInt32BE(16), bytes.readUInt32BE(20), bytes[25]];
  if (w !== width || h !== height) throw new Error(`${path} is ${w}x${h}, not ${width}x${height}`);
  if (colour !== 2) {
    throw new Error(
      `${path} has colour type ${colour} (2 is RGB, 6 is RGBA): Apple takes no alpha channel. Flatten it, for example in Preview (File > Export, untick Alpha).`,
    );
  }
}

const args = argv.slice(2);
const only = option(args, "--only");
const distArg = option(args, "--dist");
const [outArg, ...rest] = args;
if (outArg === undefined || rest.length > 0) usage("one output folder is needed");
const dist = resolve(distArg ?? join(ROOT, "apps/web/dist"));
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
/** Requests that left the local server: refused, and a failure of the run. */
const refused = [];
const browser = await chromium.launch();
try {
  for (const size of SIZES) {
    for (const scene of scenes) {
      const context = await browser.newContext({
        viewport: WINDOW,
        deviceScaleFactor: size.scale,
        colorScheme: scene.theme === "dark" ? "dark" : "light",
        reducedMotion: "reduce",
        serviceWorkers: "block",
      });
      await context.route("**/*", (route) => {
        if (route.request().url().startsWith(`${origin}/`)) return route.continue();
        refused.push(route.request().url());
        return route.abort();
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
      await page.getByRole("heading", { name: "Open a PDF" }).waitFor({ timeout: WAIT });
      await open(page, fixture(scene.file));
      if (scene.run) await scene.run(page);
      const path = join(out, `${scene.name}-${size.name}.png`);
      await page.screenshot({ path, animations: "disabled", caret: "hide" });
      checkPng(path, WINDOW.width * size.scale, WINDOW.height * size.scale);
      console.log(`${path}  ${scene.caption}`);
      await context.close();
    }
  }
} finally {
  await browser.close();
  server.close();
}
if (refused.length > 0) {
  console.error(`a request left the local server and was refused:\n${[...new Set(refused)].join("\n")}`);
  exit(1);
}
