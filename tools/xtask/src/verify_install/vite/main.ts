// `cargo xtask verify-install` in Chromium: a Vite app over the installed tarballs (the `browser`
// export condition), built for production; chromium.mjs reads the check line from the page.
import { init } from "@laminapdf/pdf";
import { hello } from "./hello.ts";
import { report } from "./report.ts";

const out = globalThis as { laminaVerify?: string };
try {
  await init({ text: true });
  out.laminaVerify = await report(hello());
} catch (error) {
  out.laminaVerify = `LAMINA_VERIFY_ERROR ${String(error)}`;
}
