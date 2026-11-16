// The question after a crash (desktop shell S08, its report part): whether to send an anonymous
// report, with the exact JSON one click away. It asks once, without pressure: a modal dialog after
// the app itself quit (the next launch shows it), or, when only the document view restarted, a
// notice in the status bar (`CrashNotice`) whose View report opens the same dialog.
//
// As built: S08's drawings name the app "lamina"; these words say "Lamina PDF", the app's own name
// everywhere else. This dialog is S08's "No session to restore" state: only the report part, under
// the same title. After an unexpected quit with tabs to reopen, crash-recovered.tsx shows the tab
// choice with this report part under it.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useEffect, useState } from "react";
import { Badge } from "../components/badge.js";
import { Button } from "../components/button.js";
import { Checkbox } from "../components/choice.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";

/** The user's answer to the report question. */
export interface CrashAnswer {
  /** Send this report. */
  readonly send: boolean;
  /** Send every later report too, without asking (only with `send`). */
  readonly always: boolean;
}

/** Which process crashed: the app itself, its document process, or a window's page. */
export type CrashProcess = "main" | "engine" | "webview";

/** Props of `CrashReportDialog`. */
export interface CrashReportDialogProps {
  /** Which process stopped: the app itself ("main", the next launch asks), the document process ("engine") or the window's page ("webview"). */
  readonly process: CrashProcess;
  /** The exact JSON that would be sent, already pretty-printed by the caller. */
  readonly report: string;
  /** The user's answer: send this report (and, with `always`, every later one), or not. */
  readonly onAnswer: (answer: CrashAnswer) => void;
  /** Copies the report; the caller writes to the clipboard. Left out, there is no Copy report. */
  readonly onCopy?: () => void;
  /** The view it opens on: `preview` for the notice's View report. `question` when left out. */
  readonly initialView?: "question" | "preview";
}

/** Which view shows: the question, the report, or the question again after Back. */
type View = "question" | "preview" | "returned";

/** Asks whether to send an anonymous crash report, and shows exactly what it holds. */
export function CrashReportDialog({
  process,
  report,
  onAnswer,
  onCopy,
  initialView = "question",
}: CrashReportDialogProps): ReactNode {
  const [view, setView] = useState<View>(initialView);
  const [always, setAlways] = useState(false);
  const restarted = process !== "main";
  const preview = view === "preview";
  const send = () => onAnswer({ send: true, always });
  const dontSend = () => onAnswer({ send: false, always: false });
  const description = preview
    ? message("crash.server")
    : restarted
      ? message("crash.restarted")
      : message("crash.about");
  return (
    // A required question: no close button, and Escape and the scrim leave it open (S08).
    <Dialog
      isOpen
      isRequired
      onOpenChange={() => undefined}
      size={preview ? "wide" : "form"}
      title={message(restarted ? "crash.title.restarted" : "crash.title.main")}
      description={description}
      actions={
        <ReportAnswers
          {...(preview && onCopy !== undefined ? { onCopy } : {})}
          onDontSend={dontSend}
          onSend={send}
          focusSend={view === "question"}
        />
      }
    >
      {preview ? (
        <ReportPreview report={report} onBack={() => setView("returned")} />
      ) : (
        <Question
          restarted={restarted}
          always={always}
          onAlways={setAlways}
          returned={view === "returned"}
          onView={() => setView("preview")}
        />
      )}
    </Dialog>
  );
}

/** The report's answers: Copy report (when `onCopy`), Don't send and Send report. */
export function ReportAnswers({
  onCopy,
  onDontSend,
  onSend,
  focusSend = false,
}: {
  onCopy?: () => void;
  onDontSend: () => void;
  onSend: () => void;
  focusSend?: boolean;
}): ReactNode {
  return (
    <>
      {onCopy === undefined ? null : (
        <span className="paper-crash-copy">
          <Button variant="ghost" icon="copy" onPress={onCopy}>
            {message("crash.copy")}
          </Button>
        </span>
      )}
      <Button onPress={onDontSend}>{message("crash.dontSend")}</Button>
      <Button variant="primary" autoFocus={focusSend} onPress={onSend}>
        {message("crash.send")}
      </Button>
    </>
  );
}

/** The question's body: what a report holds, Always send, and View report. */
function Question({
  restarted,
  always,
  onAlways,
  returned,
  onView,
}: {
  restarted: boolean;
  always: boolean;
  onAlways: (always: boolean) => void;
  returned: boolean;
  onView: () => void;
}): ReactNode {
  return (
    <>
      {/* After an engine restart the description says so; what a report holds comes next. */}
      {restarted ? <p className="paper-crash-about">{message("crash.about")}</p> : null}
      <Checkbox isSelected={always} onChange={onAlways}>
        {message("crash.always")}
      </Checkbox>
      <span className="paper-crash-view">
        {/* Back from the report returns focus here, where the user left. */}
        <Button variant="ghost" autoFocus={returned} onPress={onView}>
          {message("crash.viewReport")}
        </Button>
      </span>
    </>
  );
}

/** The report preview's body: Back, what the report leaves out, the JSON and the report ID's note. */
export function ReportPreview({ report, onBack }: { report: string; onBack: () => void }): ReactNode {
  return (
    <>
      {/* First in the tab order; crash-report.css draws it above the title, top left. */}
      <span className="paper-crash-back">
        <Button variant="ghost" icon="back" autoFocus onPress={onBack}>
          {message("crash.back")}
        </Button>
      </span>
      <ul className="paper-crash-badges" aria-label={message("crash.leavesOut")}>
        <li>
          <Badge tone="positive" icon="check">
            {message("crash.noPaths")}
          </Badge>
        </li>
        <li>
          <Badge tone="positive" icon="check">
            {message("crash.noContent")}
          </Badge>
        </li>
        <li>
          <Badge tone="positive" icon="check">
            {message("crash.noUpdateId")}
          </Badge>
        </li>
      </ul>
      {/* Focusable, so a keyboard can scroll it (S08); a region, since a bare pre can't take a name. */}
      {/* biome-ignore lint/a11y/noNoninteractiveTabindex: S08 asks for a focusable report so the keyboard can scroll it. */}
      {/* biome-ignore lint/a11y/useSemanticElements: the scrolled block is the pre itself, which S08 names. */}
      <pre className="paper-crash-json" tabIndex={0} role="region" aria-label={message("crash.report")}>
        {report}
      </pre>
      <p className="paper-crash-note">{message("crash.reportId")}</p>
    </>
  );
}

/** Props of `CrashNotice`. */
export interface CrashNoticeProps {
  /** What went wrong: the document view restarted (`engine`), or the window's page hit an error. */
  readonly process?: Exclude<CrashProcess, "main">;
  /** Send was pressed: send this report. */
  onSend: () => void;
  /** View report was pressed: the app opens `CrashReportDialog` on its report. */
  onViewReport: () => void;
  /** Don't send was pressed. */
  onDismiss: () => void;
}

/** The report question after a problem in a running window: a notice, not a modal (S08). */
export function CrashNotice({ process = "engine", onSend, onViewReport, onDismiss }: CrashNoticeProps): ReactNode {
  // Mounted empty, then filled, so a screen reader says it (as RestoreNotice does).
  const [said, setSaid] = useState(false);
  useEffect(() => setSaid(true), []);
  return (
    <div className="paper-crash-notice">
      <Icon name="warning" />
      <p role="status">{said ? message(process === "engine" ? "crash.notice" : "crash.notice.webview") : null}</p>
      {/* Together, so a narrow window wraps the buttons below the sentence as one row. */}
      <div className="paper-crash-notice-actions">
        <Button variant="ghost" onPress={onViewReport}>
          {message("crash.viewReport")}
        </Button>
        <Button onPress={onDismiss}>{message("crash.dontSend")}</Button>
        <Button variant="primary" onPress={onSend}>
          {message("crash.notice.send")}
        </Button>
      </div>
    </div>
  );
}
