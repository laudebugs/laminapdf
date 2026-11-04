// The viewer's document worker: `@laminapdf/sdk/worker`'s server with the viewer's methods
// added, so it speaks the same protocol (handshake, queue, cancellation, errors, crash reports).
// Started by ViewerClient (`new Worker(new URL("./worker/entry.js", import.meta.url))`).
import { serve } from "@laminapdf/sdk/worker/server";
import { viewerMethods } from "./methods.js";

serve({ methods: viewerMethods });
