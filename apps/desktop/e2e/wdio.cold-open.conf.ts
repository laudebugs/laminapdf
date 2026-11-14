// The cold start with a file: the app started with a PDF's path, as a double-click on a file starts
// it when it isn't running. The main run (wdio.conf.ts) starts the app with no file, so this run
// starts its own.
import { rmSync } from "node:fs";
import { oddlyNamedCopy, sessionFile } from "./paths.ts";
import { e2eConfig } from "./wdio.conf.ts";

// Without the main run's session: this run checks what a launch with a file opens, not what comes back.
export const config: WebdriverIO.Config = {
  ...e2eConfig(["cold-open"], [oddlyNamedCopy()]),
  onPrepare: () => rmSync(sessionFile, { force: true }),
};
