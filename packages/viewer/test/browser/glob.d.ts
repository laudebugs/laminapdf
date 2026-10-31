// Vite's `?url` imports, for tsc.
declare module "*.wasm?url" {
  const url: string;
  export default url;
}
