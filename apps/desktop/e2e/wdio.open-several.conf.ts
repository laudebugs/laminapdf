// Several files opened together: the app started with three PDFs' paths, as selecting them in
// Finder or Explorer and opening them together starts it. The window asks whether to merge them
// into one document or open each. The main run (wdio.conf.ts) starts the app with no file, so this
// run starts its own, last, since it leaves the merged file in the recent list.
import { rmSync } from "node:fs";
import { join } from "node:path";
import { root, sessionFile } from "./paths.ts";
import { clearRunningMark, e2eConfig } from "./wdio.conf.ts";

/** The three files, in the order the launch names them. */
export const SEVERAL = ["hello.pdf", "insert_pages.pdf", "extract_pages.pdf"].map((name) =>
  join(root, "fixtures", "golden", "examples", name),
);

// Without the main run's session: this run checks what a launch with several files asks.
export const config: WebdriverIO.Config = {
  ...e2eConfig(["open-several"], SEVERAL),
  onPrepare: () => {
    clearRunningMark();
    rmSync(sessionFile, { force: true });
  },
};
