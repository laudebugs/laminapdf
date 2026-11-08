import { fileURLToPath } from "node:url";
import babel from "@rolldown/plugin-babel";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The gallery, built with the React Compiler as an app would build the chrome. `@laminapdf/ui`
// resolves to its source, as `@laminapdf/ui/testing`'s states do, so both use one copy of each
// component (one density context, one layer stack). A default export because Vite reads one.
export default defineConfig({
  plugins: [react(), babel({ presets: [reactCompilerPreset()] })],
  resolve: {
    alias: [
      {
        find: /^@laminapdf\/ui$/,
        replacement: fileURLToPath(new URL("../../packages/ui/src/index.ts", import.meta.url)),
      },
    ],
  },
  build: { outDir: "dist", emptyOutDir: true },
  preview: { port: 4174, strictPort: true },
});
