// Vite's client types for the worker's `?url` imports (scripts/build-runner.mjs builds it with Vite).
/// <reference types="vite/client" />

/** The editor's TypeScript files by path (scripts/build-runner.mjs's `laminaTypes`). */
declare module "virtual:lamina-types" {
  const files: Readonly<Record<string, string>>;
  export default files;
}
