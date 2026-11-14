// What the session scenarios share: the three files they open, and the session file as the app
// left it, read from the test process as another program would.
import { existsSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { fixture } from "./app.ts";
import { output, sessionFile } from "./paths.ts";

export const example = (name: string) => fixture("fixtures", "golden", "examples", name);

/** The main window's files, in tab order, and the second window's, as session-save leaves them. */
export const MAIN_TABS = ["invoice_from_json.pdf", "hello.pdf"];
export const OTHER_TABS = ["markdown_to_pdf.pdf"];

/** What session-save measured, for session-restore to compare: the zoom and the main window's size. */
export const expected = join(output, "session-expected.json");

export interface Expected {
  readonly percent: number;
  readonly width: number;
  readonly height: number;
}

/** Each window's file names in the session file, in order; `undefined` while there is no file. */
export function savedTabs(): string[][] | undefined {
  if (!existsSync(sessionFile)) return undefined;
  const session = JSON.parse(readFileSync(sessionFile, "utf8")) as { windows: { tabs: { path: string }[] }[] };
  return session.windows.map((window) => window.tabs.map((tab) => basename(tab.path)));
}
