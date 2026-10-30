// Wrangler's `Data` modules (wrangler.toml `rules`): the file's bytes.
declare module "*.ttf" {
  const data: ArrayBuffer;
  export default data;
}
declare module "*.otf" {
  const data: ArrayBuffer;
  export default data;
}
