import { fileURLToPath } from "node:url";
import babel from "@rolldown/plugin-babel";
import react, { reactCompilerPreset } from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The demo page (examples/ui-demo.html), built with the React Compiler through Babel, as an app
// would build the chrome. `pnpm demo` serves it; `pnpm demo:build` writes examples/dist/.
// A default export because Vite reads one.
export default defineConfig({
  root: "examples",
  plugins: [react(), babel({ presets: [reactCompilerPreset()] })],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rolldownOptions: { input: fileURLToPath(new URL("./examples/ui-demo.html", import.meta.url)) },
  },
});
