import { defineConfig } from "vite";

// Tauri serves dist/ in release builds; the dev server is only for `pnpm tauri dev`.
export default defineConfig({
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
