// The reader's entry: the Paper tokens, fonts and components' styles, the first paint's font
// preloaded, one platform and one engine for the page, and the app rendered into #app. The engine
// starts loading the render backend at once, while the user picks a file.

// First: the page's network feed wraps fetch and friends before any other module runs.
import { networkMonitor } from "./network/page-feed.ts";
import "@laminapdf/tokens/tokens.css";
import "@laminapdf/fonts/fonts.css";
import "@laminapdf/ui/styles.css";
import "./app.css";
import { FONT_PRELOADS } from "@laminapdf/fonts";
import { createWebPlatform, PlatformProvider } from "@laminapdf/ui";
import { createRoot } from "react-dom/client";
import { App } from "./app.tsx";
import { wantsDebug } from "./document.ts";
import { openLaunchedFiles } from "./offline/launch.ts";
import { AppUpdates } from "./offline/updates.ts";
import { watchFragment } from "./open/watch.ts";
import { Engine } from "./session.ts";

for (const font of FONT_PRELOADS) {
  const link = document.createElement("link");
  link.rel = "preload";
  link.as = "font";
  link.type = font.type;
  link.crossOrigin = "";
  link.href = font.href;
  document.head.append(link);
}

// React Aria inserts a <style> element for pressable elements unless one with this ID exists, and the
// CSP (style-src 'self') blocks inline styles: so the element is claimed here and app.css carries
// its rule. A newer React Aria that renames the ID shows up as a CSP violation in the e2e suite.
const pressable = document.createElement("meta");
pressable.id = "react-aria-pressable-style";
document.head.append(pressable);

const root = document.getElementById("app");
if (root === null) throw new Error("index.html has no #app");
// One platform for the page: two would each open the recent-files store and take the launch queue.
const platform = createWebPlatform();
const engine = new Engine(platform, "pdfium", { outlineText: wantsDebug(location.hash), network: networkMonitor });
// `#url=` opens a remote file now and whenever the fragment changes.
watchFragment(window, engine);
// A file the operating system opened with the installed reader ("Open with", the manifest's
// file_handlers; Chromium) opens through the same path as one chosen here. The reader shows one
// document, so of several files the first opens.
openLaunchedFiles(platform, engine.reader);
// The service worker (R15), in production builds only: the dev server's modules change under it.
const updates = new AppUpdates(
  import.meta.env.PROD && "serviceWorker" in navigator ? navigator.serviceWorker : undefined,
  () => location.reload(),
);
// After the load event, so installing the shell's files never competes with the first page.
// A browser that refuses it (a private window, an untrusted certificate) leaves the reader working online.
window.addEventListener(
  "load",
  () => void updates.start().catch((error: unknown) => console.warn("the service worker didn't register:", error)),
  { once: true },
);
createRoot(root).render(
  <PlatformProvider platform={platform}>
    <App
      engine={engine}
      platform={platform}
      network={networkMonitor}
      updates={updates}
      debug={wantsDebug(location.hash)}
    />
  </PlatformProvider>,
);
