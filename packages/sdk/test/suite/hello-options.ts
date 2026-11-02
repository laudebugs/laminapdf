// What an app's environment would supply to the examples (file ID, producer, creation date),
// fixed so their bytes are the goldens'. Its own module, so the Vite bundle imports it without the
// whole suite. `crates/laminapdf/tests/examples.rs` passes the same values to the Rust examples.
// `cargo xtask verify-install` copies this file alone into its test projects: keep it free of
// relative imports.
import type { BuilderOptions } from "@laminapdf/sdk";

/** An example's fixed options: `id` (16 ASCII bytes), the examples' producer and creation date. */
export function exampleOptions(id: string): BuilderOptions {
  return { id: new TextEncoder().encode(id), producer: "lamina examples", creationDate: "D:20261030120000Z" };
}

/** The hello golden's file ID, producer and creation date. */
export const HELLO_OPTIONS: BuilderOptions = exampleOptions("lamina-ex-hello!");
