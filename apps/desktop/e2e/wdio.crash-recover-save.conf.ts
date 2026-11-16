// The crash-recovered pair's first run (D7e): one launch from a clean start (no session, the crash
// question already answered so none asks), whose scenario ends the app with a forced panic.
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { crashConsentFile, preferencesFile, sessionFile } from "./paths.ts";
import { clearRunningMark, e2eConfig } from "./wdio.conf.ts";

export const config: WebdriverIO.Config = {
  ...e2eConfig(["crash-recover-save"]),
  onPrepare: () => {
    clearRunningMark();
    rmSync(sessionFile, { force: true });
    rmSync(preferencesFile, { force: true });
    mkdirSync(dirname(crashConsentFile), { recursive: true });
    writeFileSync(crashConsentFile, JSON.stringify({ consent: { state: "asked", send: false } }));
  },
};
