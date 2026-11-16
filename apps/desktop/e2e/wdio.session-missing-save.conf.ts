// The missing-file session's save run (D4f), one launch of the app after the other session runs.
import { rmSync } from "node:fs";
import { preferencesFile, sessionFile } from "./paths.ts";
import { makeVanishing } from "./session.ts";
import { clearRunningMark, e2eConfig } from "./wdio.conf.ts";

// From a clean start, with the file that the next run finds gone.
export const config: WebdriverIO.Config = {
  ...e2eConfig(["session-missing-save"]),
  onPrepare: () => {
    clearRunningMark();
    rmSync(sessionFile, { force: true });
    rmSync(preferencesFile, { force: true });
    makeVanishing();
  },
};
