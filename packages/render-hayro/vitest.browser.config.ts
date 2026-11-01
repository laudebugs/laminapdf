import { renderBrowserConfig } from "@laminapdf/test-utils/browser-config";

// The RenderBackend contract in Chromium, Firefox and WebKit, with C2b's goldens: the shared
// config in @laminapdf/test-utils. `cargo xtask pdf-runtimes <browser>` runs it.
// A default export because Vitest reads one.
export default renderBrowserConfig(new URL("./", import.meta.url));
