// The crash-recovered pair's second run (D7e): the launch after crash-recover-save's forced panic.
// It keeps the running mark the panic left (no clearRunningMark), so the app knows it quit
// unexpectedly.
import { e2eConfig } from "./wdio.conf.ts";

export const config: WebdriverIO.Config = {
  ...e2eConfig(["crash-recover"]),
  onPrepare: () => undefined,
};
