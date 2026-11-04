// The test Worker's images, bundled as Wrangler `Data` modules like its fonts (whose declarations
// are the Worker example's src/data.d.ts: one per pattern in the package's program).
declare module "*.jpg" {
  const data: ArrayBuffer;
  export default data;
}
