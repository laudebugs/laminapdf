// The spike's page: the fixture picker and print buttons, and path A's print-only DOM.
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Plan {
  label: string;
  mode: string;
  dpi: number;
  fixtures: { name: string; pages: { w: number; h: number }[] }[];
}

const plan = await invoke<Plan>("plan");
const status = document.getElementById("status") as HTMLElement;
const select = document.getElementById("fixture") as HTMLSelectElement;
const printRoot = document.getElementById("print") as HTMLElement;
// "lamina://localhost/" on macOS and Linux, "http://lamina.localhost/" on Windows.
const base = convertFileSrc("", "lamina");
const say = (text: string) => {
  status.textContent = text;
  void invoke("report", { msg: text });
};

for (const f of plan.fixtures) select.add(new Option(`${f.name} (${f.pages.length} pages)`, f.name));

/** One named @page rule per page size, in an adopted stylesheet (the CSP allows no inline style). */
const pageSheet = new CSSStyleSheet();
document.adoptedStyleSheets = [...document.adoptedStyleSheets, pageSheet];

/** Path A: every page as an <img> at the plan's dpi, each on its own sized page. */
async function prepare(name: string): Promise<void> {
  const fixture = plan.fixtures.find((f) => f.name === name);
  if (!fixture) throw new Error(`no fixture ${name}`);
  const sizes = [...new Set(fixture.pages.map((p) => `${p.w} ${p.h}`))];
  const rules = sizes.map(
    (s, i) => `@page size${i} { size: ${s.split(" ").map((n) => `${n}pt`).join(" ")}; margin: 0; }`,
  );
  rules.push(".page { break-after: page; overflow: hidden; } .page:last-child { break-after: auto; }");
  rules.push(".page img { display: block; width: 100%; height: 100%; }");
  sizes.forEach((s, i) => {
    const [w, h] = s.split(" ");
    rules.push(`.size${i} { page: size${i}; width: ${w}pt; height: ${h}pt; }`);
  });
  pageSheet.replaceSync(rules.join("\n"));
  printRoot.replaceChildren();
  const images = fixture.pages.map((p, i) => {
    const page = document.createElement("div");
    page.className = `page size${sizes.indexOf(`${p.w} ${p.h}`)}`;
    const img = document.createElement("img");
    // The fixture's name from the app's own plan, encoded: nothing read from the page reaches
    // the URL (CodeQL js/xss-through-dom).
    img.src = `${base}page/${encodeURIComponent(fixture.name)}/${i}`;
    img.alt = `page ${i + 1}`;
    page.append(img);
    printRoot.append(page);
    return img;
  });
  await Promise.all(images.map((img) => img.decode()));
}

/** The fixture whose print DOM is being built: Rust repeats its request until we answer. */
let building: string | null = null;

await listen<string>("prepare", async (event) => {
  if (building === event.payload) return;
  building = event.payload;
  try {
    await prepare(event.payload);
    say(`print DOM ready: ${event.payload}`);
    await invoke("webview_ready", { error: null });
  } catch (e) {
    await invoke("webview_ready", { error: String(e) });
  } finally {
    building = null;
  }
});

for (const button of document.querySelectorAll<HTMLButtonElement>("button[data-path]")) {
  button.addEventListener("click", async () => {
    const path = button.dataset.path;
    const fixture = select.value;
    say(`${path}: ${fixture}…`);
    try {
      if (path === "windowprint") {
        await prepare(fixture);
        window.print();
        say("window.print() returned");
      } else {
        await invoke(`print_${path === "webview" ? "webview" : `native_${path}`}`, { fixture });
        say(`${path}: done`);
      }
    } catch (e) {
      say(`${path}: ${String(e)}`);
    }
  });
}

if (plan.mode === "dialog-windowprint") {
  await prepare("mixed-50");
  window.print();
}
say(`mode ${plan.mode}`);
