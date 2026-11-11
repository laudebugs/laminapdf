import { fileURLToPath } from "node:url";
import babel from "@rolldown/plugin-babel";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The gallery, built with the React Compiler as an app would build the chrome. `@laminapdf/ui` and
// its entry subpaths (`/dialogs`, `/editing`, `/reader-mode`) resolve to their source, as
// `@laminapdf/ui/testing`'s states do, so all use one copy of each component (one density context,
// one layer stack) and the gallery needs no build of the package. A default export because Vite
// reads one.
export default defineConfig({
  plugins: [react(), babel({ presets: [reactCompilerPreset()] })],
  resolve: {
    alias: [
      {
        find: /^@laminapdf\/ui$/,
        replacement: fileURLToPath(new URL("../../packages/ui/src/index.ts", import.meta.url)),
      },
      {
        find: /^@laminapdf\/ui\/(dialogs|editing|reader-mode)$/,
        replacement: fileURLToPath(new URL("../../packages/ui/src/$1.ts", import.meta.url)),
      },
    ],
  },
  build: { outDir: "dist", emptyOutDir: true },
  preview: { port: 4174, strictPort: true },
});
