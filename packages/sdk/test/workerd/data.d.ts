/// <reference path="../../examples/worker/src/data.d.ts" />
// The test Worker's images are Wrangler `Data` modules too. Its fonts use the Worker example's
// declarations, referenced above: TypeScript refuses a second declaration of the same pattern.
declare module "*.jpg" {
  const data: ArrayBuffer;
  export default data;
}
