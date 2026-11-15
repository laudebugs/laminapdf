// The e2e suite's server: `wrangler dev --local` over the built dist/ with its _headers
// (@laminapdf/test-utils' wranglerDevSetup, over HTTPS). Its address reaches the tests through
// Vitest's `provide` (inject("origin")). A default export because Vitest reads one.
import { wranglerDevSetup } from "@laminapdf/test-utils/wrangler-dev";

export default wranglerDevSetup(
  new URL("../", import.meta.url),
  "dist/index.html",
  "apps/site/dist/ isn't built: run `pnpm --filter @laminapdf/site build`",
);
