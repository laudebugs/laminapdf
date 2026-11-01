// What a quick start's environment would supply (file ID, producer, date), fixed so its bytes are
// the hello golden's. Its own module, so the Vite bundle imports it without the whole suite. It
// repeats examples/hello.ts's values until that example takes them as a parameter (todo Q1f).
import type { BuilderOptions } from "@laminapdf/sdk";

/** The hello golden's file ID, producer and creation date. */
export const HELLO_OPTIONS: BuilderOptions = {
  id: new TextEncoder().encode("lamina-ex-hello!"),
  producer: "lamina examples",
  creationDate: "D:20261030120000Z",
};
