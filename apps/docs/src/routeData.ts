// Starlight's route middleware: TypeDoc's reference pages are generated at build time into a folder
// git ignores, so "Edit page" would open a file that doesn't exist; they get none.
import { defineRouteMiddleware, type StarlightRouteData } from "@astrojs/starlight/route-data";

/** Where TypeDoc writes the @laminapdf/sdk reference (astro.config.mjs's `output`). */
const GENERATED = "reference/sdk";

export const onRequest = defineRouteMiddleware((context) => {
  // Starlight's own `Locals` type isn't in this project's types; the route is its exported shape.
  const route = (context.locals as { starlightRoute: StarlightRouteData }).starlightRoute;
  if (route.id === GENERATED || route.id.startsWith(`${GENERATED}/`)) route.editUrl = undefined;
});
