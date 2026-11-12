// The desktop reader's entry: the Paper tokens, fonts and components' styles, the first paint's font
// preloaded, one desktop Platform, and the app rendered into #app. Fonts ship in the app bundle:
// nothing is fetched at run time.
import "@laminapdf/tokens/tokens.css";
import "@laminapdf/fonts/fonts.css";
import "@laminapdf/ui/styles.css";
import "./app.css";
import { FONT_PRELOADS } from "@laminapdf/fonts";
import { PlatformProvider } from "@laminapdf/ui";
import { createViewer } from "@laminapdf/viewer";
import { isTauri } from "@tauri-apps/api/core";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { App, type DesktopEngine } from "./app.tsx";
import { type AppInfo, loadAppInfo } from "./app-info.ts";
import { DesktopEngineClient, type DesktopShell } from "./engine/desktop-engine-client.ts";
import { tauriShell } from "./engine/tauri-shell.ts";
import { createDesktopPlatform } from "./platform/desktop.ts";

/**
 * This window's engine client and its view. The text and link layers stay off until the engine
 * reads text; `#raw` in the address uses the raw RGBA transport instead of WebP, for diagnosis
 * (ADR 0008 §4).
 */
function startEngine(shell: DesktopShell | undefined): DesktopEngine | undefined {
  if (shell === undefined) return undefined;
  const client = new DesktopEngineClient(shell, { transport: location.hash === "#raw" ? "raw" : "webp" });
  return { shell, client, viewer: createViewer({ client, text: false, links: false }) };
}

for (const font of FONT_PRELOADS) {
  const link = document.createElement("link");
  link.rel = "preload";
  link.as = "font";
  link.type = font.type;
  link.crossOrigin = "";
  link.href = font.href;
  document.head.append(link);
}

// React Aria inserts a <style> element unless one with this ID exists, and the CSP blocks inline
// styles; the web reader does the same (apps/web/src/main.tsx).
const pressable = document.createElement("meta");
pressable.id = "react-aria-pressable-style";
document.head.append(pressable);

const root = document.getElementById("app");
if (root === null) throw new Error("index.html has no #app");
// The one place that asks where it runs (ADR 0010): outside Tauri (`pnpm dev` in a browser) the
// shell's commands don't exist.
const desktop = isTauri();
// Shortcut chips and `Mod` follow the OS, as in the web reader (apps/web/src/app.tsx).
const keys = /Mac|iPhone|iPad/.test(navigator.platform) ? "mac" : "other";
// Inside the shell, one engine client for this window and the view it draws; the platform's
// open dialog runs in the main process.
const shell = desktop ? tauriShell() : undefined;
const engine = startEngine(shell);
const platform = createDesktopPlatform(undefined, shell);
const app = createRoot(root);
const render = (info: AppInfo | undefined) =>
  app.render(
    <PlatformProvider platform={platform}>
      <App platform={platform} info={info} keys={keys} engine={engine} />
    </PlatformProvider>,
  );
// The empty state commits before the first command goes out, so that command marks the moment the
// window can take input (ADR 0008 §4's cold-start gate); `render` alone only schedules the work.
flushSync(() => render(undefined));
if (desktop) loadAppInfo().then(render, (error: unknown) => console.error("app_info failed", error));
