// The session scenario's save run (D4d), one launch of the app: the session runs follow each other,
// each relaunching the app after the last.
import { rmSync } from "node:fs";
import { preferencesFile, sessionFile } from "./paths.ts";
import { e2eConfig } from "./wdio.conf.ts";

// From a clean start: no session, and the preferences at their defaults (both switches on).
export const config: WebdriverIO.Config = {
  ...e2eConfig(["session-save"]),
  onPrepare: () => {
    rmSync(sessionFile, { force: true });
    rmSync(preferencesFile, { force: true });
  },
};
