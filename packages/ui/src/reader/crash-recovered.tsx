// After the app quit unexpectedly (desktop shell S08): the last session's tabs as check boxes,
// before any reopens, with the tab that was in front unchecked and badged, since it may have
// caused the crash; then, when the app may ask, the report part under a hairline. Start without
// tabs and Reopen N tabs both answer the report part too. Without a session the app shows
// `CrashReportDialog` alone (S08's "No session to restore").

import { type ReactNode, useState } from "react";
import { Badge } from "../components/badge.js";
import { Button } from "../components/button.js";
import { Checkbox } from "../components/choice.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
import { ReportPreview } from "./crash-report.js";

/** A tab of the last session, as the choice lists it. */
export interface RecoveredTab {
  /** The main process's name for it; never a path. */
  readonly token: string;
  /** The file's name. */
  readonly name: string;
  /** The page it was on, when known. */
  readonly page?: number;
  /** It was the tab in front when the app quit: unchecked at first, and badged. */
  readonly front: boolean;
}

/** The report the app may ask about, under the tabs. */
export interface RecoveredReport {
  /** The exact JSON that would be sent, already pretty-printed. */
  readonly json: string;
  /** Copies the report; left out, there is no Copy report. */
  readonly onCopy?: () => void;
}

/** The user's choice: the tabs to reopen (none: Start without tabs) and the report's answer. */
export interface RecoveredChoice {
  readonly tokens: readonly string[];
  /** Send the report (always `false` with no report part). */
  readonly send: boolean;
  /** Send every later report too (only with `send`). */
  readonly always: boolean;
}

/** Props of `CrashRecoveredDialog`. */
export interface CrashRecoveredDialogProps {
  readonly tabs: readonly RecoveredTab[];
  /** The report part, when the app may ask about this crash. */
  readonly report?: RecoveredReport;
  readonly onChoose: (choice: RecoveredChoice) => void;
}

/** Which tabs reopen, and whether to send a report, after the app quit unexpectedly. */
export function CrashRecoveredDialog({ tabs, report, onChoose }: CrashRecoveredDialogProps): ReactNode {
  const [chosen, setChosen] = useState(() => new Set(tabs.filter((tab) => !tab.front).map((tab) => tab.token)));
  const [send, setSend] = useState(false);
  const [always, setAlways] = useState(false);
  const [preview, setPreview] = useState(false);
  const toggle = (token: string, on: boolean) =>
    setChosen((was) => {
      const next = new Set(was);
      if (on) next.add(token);
      else next.delete(token);
      return next;
    });
  const answer = (tokens: readonly string[]) => onChoose({ tokens, send, always: send && always });
  // From the report: Send report ticks the box, Don't send clears it, and both go back to the tabs.
  const decide = (yes: boolean) => {
    setSend(yes);
    if (!yes) setAlways(false);
    setPreview(false);
  };
  const count = chosen.size;
  return (
    // A required question: no close button, and Escape and the scrim leave it open (S08).
    <Dialog
      isOpen
      isRequired
      onOpenChange={() => undefined}
      size={preview ? "wide" : "form"}
      title={message("crash.title.main")}
      description={preview ? message("crash.server") : message("crash.chooseTabs")}
      actions={
        preview && report !== undefined ? (
          <>
            {report.onCopy === undefined ? null : (
              <span className="paper-crash-copy">
                <Button variant="ghost" icon="copy" onPress={report.onCopy}>
                  {message("crash.copy")}
                </Button>
              </span>
            )}
            <Button onPress={() => decide(false)}>{message("crash.dontSend")}</Button>
            <Button variant="primary" onPress={() => decide(true)}>
              {message("crash.send")}
            </Button>
          </>
        ) : (
          <>
            <Button onPress={() => answer([])}>{message("crash.startEmpty")}</Button>
            <Button
              variant="primary"
              autoFocus
              isDisabled={count === 0}
              onPress={() => answer(tabs.filter((tab) => chosen.has(tab.token)).map((tab) => tab.token))}
            >
              {count === 1 ? message("crash.reopen.one") : message("crash.reopen.other", { count })}
            </Button>
          </>
        )
      }
    >
      {preview && report !== undefined ? (
        <ReportPreview report={report.json} onBack={() => setPreview(false)} />
      ) : (
        <>
          <fieldset className="paper-crash-tabs">
            <legend className="paper-visually-hidden">{message("crash.tabs")}</legend>
            {tabs.map((tab) => (
              <Checkbox key={tab.token} isSelected={chosen.has(tab.token)} onChange={(on) => toggle(tab.token, on)}>
                <span className="paper-crash-tab">
                  <span className="paper-crash-tab-name">{tab.name}</span>
                  {tab.page === undefined ? null : (
                    <span className="paper-crash-tab-page">{message("crash.page", { page: tab.page })}</span>
                  )}
                  {tab.front ? (
                    <Badge tone="warning" icon="warning">
                      {message("crash.frontTab")}
                    </Badge>
                  ) : null}
                </span>
              </Checkbox>
            ))}
          </fieldset>
          {report === undefined ? null : (
            <div className="paper-crash-report-part">
              <p className="paper-crash-about">{message("crash.about")}</p>
              <Checkbox
                isSelected={send}
                onChange={(on) => {
                  setSend(on);
                  if (!on) setAlways(false);
                }}
              >
                {message("crash.sendThis")}
              </Checkbox>
              <Checkbox isSelected={always} isDisabled={!send} onChange={setAlways}>
                {message("crash.always")}
              </Checkbox>
              {send ? null : <p className="paper-crash-note">{message("crash.alwaysNeedsSend")}</p>}
              <span className="paper-crash-view">
                <Button variant="ghost" onPress={() => setPreview(true)}>
                  {message("crash.viewReport")}
                </Button>
              </span>
            </div>
          )}
        </>
      )}
    </Dialog>
  );
}
