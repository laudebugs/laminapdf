// The session scenario's save run (D4d), one launch of the app: the session runs follow each other,
// each relaunching the app after the last.
import { e2eConfig } from "./wdio.conf.ts";

export const config = e2eConfig(["session-save"]);
