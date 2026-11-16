// The crash-report run (D7a): one launch of the app from a clean start (no session, no consent
// given, no report kept), with its reports going to a local stand-in for the update Worker.
import { rmSync } from "node:fs";
import type { Server } from "node:http";
import { env } from "node:process";
import { CRASH_PORT, makeMarkedFile, startStandIn } from "./crash.ts";
import { crashConsentFile, crashesDir, preferencesFile, sessionFile } from "./paths.ts";
import { clearRunningMark, e2eConfig } from "./wdio.conf.ts";

// The app the service starts inherits this (the e2e build reads it; others don't).
env.LAMINA_PDF_CRASH_ENDPOINT = `http://127.0.0.1:${CRASH_PORT}/crash`;

let standIn: Server | undefined;
export const config: WebdriverIO.Config = {
  ...e2eConfig(["crash"]),
  onPrepare: () => {
    clearRunningMark();
    rmSync(sessionFile, { force: true });
    rmSync(preferencesFile, { force: true });
    rmSync(crashConsentFile, { force: true });
    rmSync(crashesDir, { recursive: true, force: true });
    makeMarkedFile();
    standIn = startStandIn();
  },
  onComplete: () => {
    standIn?.close();
  },
};
