// The desktop reader's entry: the Paper tokens, fonts and components' styles, the first paint's font
// preloaded, one desktop Platform, and the app rendered into #app. Fonts ship in the app bundle:
// nothing is fetched at run time.
import "@laminapdf/tokens/tokens.css";
import "@laminapdf/fonts/fonts.css";
import "@laminapdf/ui/styles.css";
import "./app.css";
import { FONT_PRELOADS } from "@laminapdf/fonts";
import { PlatformProvider } from "@laminapdf/ui";
import { isTauri } from "@tauri-apps/api/core";
import { createRoot } from "react-dom/client";
import { type AppInfo, loadAppInfo } from "./app-info.ts";
import { App } from "./app.tsx";
import { createDesktopPlatform } from "./platform/desktop.ts";

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
const platform = createDesktopPlatform();
const app = createRoot(root);
const render = (info: AppInfo | undefined) =>
  app.render(
    <PlatformProvider platform={platform}>
      <App platform={platform} info={info} />
    </PlatformProvider>,
  );
render(undefined);
if (desktop) loadAppInfo().then(render, (error: unknown) => console.error("app_info failed", error));
