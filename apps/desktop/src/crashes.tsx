// Crash reports in the desktop app (D7a; S08's report question, S07's Privacy section). The main
// process keeps every report and decides whether one may be asked about (src-tauri/src/crash/):
// this window only shows the question, passes the answer back, and hands the main process its own
// uncaught errors. A crash of the app itself is asked about at the next launch, in a dialog; one of
// the document view, while the window runs, in a notice, since the view has already restarted.

import { Button, CrashNotice, CrashReportDialog, message, Switch } from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";

/** The event the main process sends this window when it may ask about a crash (crash/mod.rs). */
export const ASK_EVENT = "crash://ask";

/** A question about a crash (`Question` in crash/mod.rs). */
interface Question {
  readonly id: string;
  readonly process: "main" | "engine" | "webview";
  /** The report's JSON: exactly what would be sent. */
  readonly report: string;
}

/** How the question shows: a dialog (on the question or the report) or a notice. */
type Shown = { readonly as: "dialog"; readonly view: "question" | "preview" } | { readonly as: "notice" };

/** The question's dialog and notice, for the app to place. */
export interface DesktopCrashes {
  /** The dialog, while one asks. */
  readonly dialog: ReactNode;
  /** The notice for the window's notice slot, while one asks. */
  readonly notice: ReactNode;
}

/**
 * Asks about the newest crash when the main process may: at launch (a crash of the app, in a
 * dialog) and when it sends {@link ASK_EVENT} (the document view's, in a notice).
 */
export function useCrashReports(shell: DesktopShell | undefined): DesktopCrashes {
  const [question, setQuestion] = useState<Question>();
  const [shown, setShown] = useState<Shown>();
  const ask = useCallback(() => {
    shell?.invoke<Question | null>("crash_question").then((asked) => {
      if (asked === null) return;
      setQuestion(asked);
      setShown(asked.process === "main" ? { as: "dialog", view: "question" } : { as: "notice" });
    }, reportError);
  }, [shell]);
  useEffect(() => {
    if (shell === undefined) return;
    ask();
    const stop = shell.listen<null>(ASK_EVENT, ask);
    return () => void stop.then((unlisten) => unlisten());
  }, [shell, ask]);
  const answer = useCallback(
    (send: boolean, always: boolean) => {
      if (question !== undefined) {
        shell?.invoke("crash_answer", { id: question.id, send, always }).catch(reportError);
      }
      setQuestion(undefined);
      setShown(undefined);
    },
    [shell, question],
  );
  if (question === undefined || shown === undefined) return { dialog: null, notice: null };
  if (shown.as === "notice") {
    return {
      dialog: null,
      notice: (
        <CrashNotice
          onSend={() => answer(true, false)}
          onViewReport={() => setShown({ as: "dialog", view: "preview" })}
          onDismiss={() => answer(false, false)}
        />
      ),
    };
  }
  return {
    dialog: (
      <CrashReportDialog
        key={question.id}
        process={question.process}
        report={question.report}
        initialView={shown.view}
        onAnswer={({ send, always }) => answer(send, always)}
        onCopy={() => void navigator.clipboard.writeText(question.report).catch(reportError)}
      />
    ),
    notice: null,
  };
}

/**
 * Hands this window's uncaught errors and unhandled rejections to the main process, which scrubs
 * them into reports. Only the message and the stack go; the main process keeps only the stack's
 * bundle files.
 */
export function reportUiErrors(shell: DesktopShell): void {
  const send = (error: unknown, fallback: string, rejection: boolean) => {
    const message = error instanceof Error ? error.message : fallback;
    const stack = error instanceof Error ? (error.stack ?? "") : "";
    // Never a report about a report: a failure here is dropped.
    shell.invoke("report_ui_error", { message, stack, rejection }).catch(() => undefined);
  };
  window.addEventListener("error", (event) => send(event.error, event.message, false));
  window.addEventListener("unhandledrejection", (event) => send(event.reason, String(event.reason), true));
}

/** Whether crash reports are sent, from the main process, and its switch. */
export function useCrashSetting(shell: DesktopShell | undefined): readonly [boolean, (on: boolean) => void] {
  const [enabled, setEnabled] = useState(false);
  useEffect(() => {
    shell?.invoke<{ enabled: boolean }>("crash_settings").then((settings) => setEnabled(settings.enabled), reportError);
  }, [shell]);
  const change = useCallback(
    (on: boolean) => {
      setEnabled(on);
      shell?.invoke("crash_set_enabled", { enabled: on }).catch(reportError);
    },
    [shell],
  );
  return [enabled, change];
}

/** Settings › Privacy (S07): the Send crash reports switch and a sample report. */
export function PrivacySettings({
  shell,
  enabled,
  onChange,
}: {
  readonly shell: DesktopShell;
  readonly enabled: boolean;
  readonly onChange: (on: boolean) => void;
}): ReactNode {
  const [sample, setSample] = useState<string>();
  const toggleSample = () => {
    if (sample !== undefined) return setSample(undefined);
    shell.invoke<string>("crash_sample").then(setSample, reportError);
  };
  return (
    <div className="paper-settings-fields">
      <Switch isSelected={enabled} onChange={onChange}>
        {message("settings.privacy.crashReports")}
      </Switch>
      <p className="desktop-settings-help">{message("settings.privacy.crashReports.help")}</p>
      <Button variant="ghost" onPress={toggleSample} aria-expanded={sample !== undefined}>
        {message(sample === undefined ? "settings.privacy.sample.show" : "settings.privacy.sample.hide")}
      </Button>
      {sample === undefined ? null : (
        // biome-ignore lint/a11y/useSemanticElements: a scrollable block of text, named for screen readers
        // biome-ignore lint/a11y/noNoninteractiveTabindex: focusable so the keyboard can scroll it (S07)
        <pre className="paper-crash-json" tabIndex={0} role="region" aria-label={message("settings.privacy.sample")}>
          {sample}
        </pre>
      )}
    </div>
  );
}

/** The Privacy section in one line. */
export function privacySummary(enabled: boolean): string {
  return message(enabled ? "settings.privacy.summary.on" : "settings.privacy.summary.off");
}

function reportError(error: unknown): void {
  console.error("crash reports failed", error);
}
