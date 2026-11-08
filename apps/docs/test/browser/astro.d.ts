/** An .astro component, as the test setup imports it to render; Astro compiles it. */
declare module "*.astro" {
  import type { AstroComponentFactory } from "astro/runtime/server/index.js";

  const component: AstroComponentFactory;
  export default component;
}
