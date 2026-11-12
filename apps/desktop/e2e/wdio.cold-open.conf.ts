// The cold start with a file: the app started with a PDF's path, as a double-click on a file starts
// it when it isn't running. The main run (wdio.conf.ts) starts the app with no file, so this run
// starts its own.
import { oddlyNamedCopy } from "./paths.ts";
import { e2eConfig } from "./wdio.conf.ts";

export const config = e2eConfig(["cold-open"], [oddlyNamedCopy()]);
