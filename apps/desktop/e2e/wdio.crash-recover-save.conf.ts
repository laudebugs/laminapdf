// The crash-recovered pair's first run: one launch from a clean start (no session, the crash
// question already answered so none asks), ended by the service's kill, which leaves the running
// mark as a crash would.
import { rmSync } from "node:fs";
import { preferencesFile, sessionFile } from "./paths.ts";
import { clearRunningMark, crashQuestionAnswered, e2eConfig } from "./wdio.conf.ts";

export const config: WebdriverIO.Config = {
  ...e2eConfig(["crash-recover-save"]),
  onPrepare: () => {
    clearRunningMark();
    rmSync(sessionFile, { force: true });
    rmSync(preferencesFile, { force: true });
    crashQuestionAnswered();
  },
};
