import babel from "@rolldown/plugin-babel";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The desktop reader's frontend, built as the web reader is: the React Compiler through Babel, and
// only files in dist/ (no inline script or style: the CSP in tauri.conf.json allows none). Tauri
// runs `pnpm dev` on the fixed port below and serves dist/ in builds. The build targets the
// platform's webview: WKWebView or WebKitGTK (Safari 15's features) or WebView2 (Chrome 105's).
// A default export because Vite reads one.
const platform = process.env.TAURI_ENV_PLATFORM;
const debug = process.env.TAURI_ENV_DEBUG === "true";

export default defineConfig({
  plugins: [react(), babel({ presets: [reactCompilerPreset()] })],
  // Tauri prints its own output; Vite clearing the screen would hide Rust's errors.
  clearScreen: false,
  server: { port: 1420, strictPort: true, fs: { allow: ["../.."] } },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: platform === "windows" ? "chrome105" : "safari15",
    modulePreload: { polyfill: false },
    assetsInlineLimit: 0,
    sourcemap: debug,
    minify: !debug,
  },
});
