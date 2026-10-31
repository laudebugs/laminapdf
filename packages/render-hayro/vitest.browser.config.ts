import { renderBrowserConfig } from "../render/test/browser-config.js";

// The RenderBackend contract in Chromium, Firefox and WebKit, with C2b's goldens: the shared
// config in packages/render/test/browser-config.ts. `cargo xtask pdf-runtimes <browser>` runs it.
// A default export because Vitest reads one.
export default renderBrowserConfig(new URL("./", import.meta.url));
