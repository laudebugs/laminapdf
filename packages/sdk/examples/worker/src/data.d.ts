// Wrangler's `Data` modules (wrangler.toml `rules`): the file's bytes. Shared by this example and
// test/workerd/ (one declaration for the package's tsconfig).
declare module "*.ttf" {
  const data: ArrayBuffer;
  export default data;
}
declare module "*.otf" {
  const data: ArrayBuffer;
  export default data;
}
declare module "*.jpg" {
  const data: ArrayBuffer;
  export default data;
}
