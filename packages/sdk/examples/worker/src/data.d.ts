// Font files import as their bytes: Wrangler bundles them as `Data` modules (wrangler.toml's
// `rules`), and this tells TypeScript so.
declare module "*.ttf" {
  const data: ArrayBuffer;
  export default data;
}
declare module "*.otf" {
  const data: ArrayBuffer;
  export default data;
}
