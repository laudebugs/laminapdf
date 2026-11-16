// The missing-file session's restore run (D4f): one of the session's files is deleted before the
// app relaunches.
import { rmSync } from "node:fs";
import { vanishing } from "./session.ts";
import { clearRunningMark, e2eConfig } from "./wdio.conf.ts";

export const config: WebdriverIO.Config = {
  ...e2eConfig(["session-missing"]),
  onPrepare: () => {
    clearRunningMark();
    rmSync(vanishing, { force: true });
  },
};
