// Crash reports in the desktop app (Paper S08's report question, S07's Privacy section). The main
// process keeps every report and decides whether one may be asked about (src-tauri/src/crash/):
// this window only shows the question, passes the answer back, and hands the main process its own
// uncaught errors. A crash of the app itself is asked about at the next launch, in a dialog; one of
// the document view, while the window runs, in a notice, since the view has already restarted.
// After the app quit unexpectedly with tabs to reopen (src-tauri/src/session/marker.rs), the main
// window's launch dialog is S08's tab choice, with the report question under it when there is one.

import {
  Button,
  CrashNotice,
  type CrashProcess,
  CrashRecoveredDialog,
  CrashReportDialog,
  message,
  type RecoveredTab,
  Switch,
} from "@laminapdf/ui";
import type { ViewPosition } from "@laminapdf/viewer";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";

/** The event the main process sends this window when it may ask about a crash (crash/mod.rs). */
const ASK_EVENT = "crash://ask";

/** A question about a crash (`Question` in crash/mod.rs). */
interface Question {
  readonly id: string;
  readonly process: CrashProcess;
  /** The report's JSON: exactly what would be sent. */
  readonly report: string;
}

/** A tab to choose after an unexpected quit (`RecoveryTab` in src-tauri/src/session/restore.rs). */
interface RecoveryTab {
  readonly token: string;
  readonly name: string;
  readonly view: ViewPosition | null;
  readonly front: boolean;
}

/** The dialog's tab, its page counted from 1. */
function recoveredTab(tab: RecoveryTab): RecoveredTab {
  const page = tab.view?.point.page;
  return {
    token: tab.token,
    name: tab.name,
    front: tab.front,
    ...(typeof page === "number" ? { page: page + 1 } : {}),
  };
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
  const [recovery, chosen] = useRecovery(shell);
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
  if (recovery !== undefined) {
    // The tab choice holds the launch's report question, if any; a notice can wait for it.
    const main = question?.process === "main" ? question : undefined;
    return { dialog: recoveredDialog(shell, recovery, main, answer, chosen), notice: null };
  }
  if (question === undefined || shown === undefined) return { dialog: null, notice: null };
  if (shown.as === "notice") {
    return {
      dialog: null,
      notice: (
        <CrashNotice
          process={question.process === "webview" ? "webview" : "engine"}
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

/** After an unexpected quit, the tabs to choose from (the main window only), and that they were chosen. */
function useRecovery(shell: DesktopShell | undefined): readonly [readonly RecoveredTab[] | undefined, () => void] {
  const [recovery, setRecovery] = useState<readonly RecoveredTab[]>();
  useEffect(() => {
    shell?.invoke<RecoveryTab[] | null>("session_recovery").then((tabs) => {
      if (tabs !== null && tabs.length > 0) setRecovery(tabs.map(recoveredTab));
    }, reportError);
  }, [shell]);
  return [recovery, useCallback(() => setRecovery(undefined), [])];
}

/** S08's tab choice, with `main`'s report question under it when there is one. */
function recoveredDialog(
  shell: DesktopShell | undefined,
  tabs: readonly RecoveredTab[],
  main: Question | undefined,
  answer: (send: boolean, always: boolean) => void,
  chosen: () => void,
): ReactNode {
  const report =
    main === undefined
      ? {}
      : {
          report: {
            json: main.report,
            onCopy: () => void navigator.clipboard.writeText(main.report).catch(reportError),
          },
        };
  return (
    <CrashRecoveredDialog
      tabs={tabs}
      {...report}
      onChoose={({ tokens, send, always }) => {
        shell?.invoke("session_reopen", { chosen: tokens }).catch(reportError);
        if (main !== undefined) answer(send, always);
        chosen();
      }}
    />
  );
}

/**
 * Hands this window's uncaught errors and unhandled rejections to the main process. Only the
 * error's type and its stack go: a message can quote anything the page holds. The main process
 * keeps the type if it is a known one and only the stack's bundle files.
 */
export function reportUiErrors(shell: DesktopShell): void {
  const send = (error: unknown, rejection: boolean) => {
    const name = error instanceof Error ? error.name : "";
    const stack = error instanceof Error ? (error.stack ?? "") : "";
    // Never a report about a report: a failure here is dropped.
    shell.invoke("report_ui_error", { name, stack, rejection }).catch(() => undefined);
  };
  window.addEventListener("error", (event) => send(event.error, false));
  window.addEventListener("unhandledrejection", (event) => send(event.reason, true));
}

/** What the main process says of crash reports: sent or not, and whether this build sends any. */
export interface CrashSetting {
  readonly enabled: boolean;
  /** False in the Mac App Store build, whose crash reports go through macOS. */
  readonly available: boolean;
}

/** Whether crash reports are sent, from the main process, and its switch. */
export function useCrashSetting(shell: DesktopShell | undefined): readonly [CrashSetting, (on: boolean) => void] {
  const [setting, setSetting] = useState<CrashSetting>({ enabled: false, available: true });
  useEffect(() => {
    shell?.invoke<CrashSetting>("crash_settings").then(setSetting, reportError);
  }, [shell]);
  const change = useCallback(
    (on: boolean) => {
      setSetting((now) => ({ ...now, enabled: on }));
      shell?.invoke<CrashSetting>("crash_set_enabled", { enabled: on }).then(setSetting, reportError);
    },
    [shell],
  );
  return [setting, change];
}

/** Settings › Privacy (S07): the Send crash reports switch and a sample report. */
export function PrivacySettings({
  shell,
  setting,
  onChange,
}: {
  readonly shell: DesktopShell;
  readonly setting: CrashSetting;
  readonly onChange: (on: boolean) => void;
}): ReactNode {
  const [sample, setSample] = useState<string>();
  if (!setting.available) {
    return (
      <div className="paper-settings-fields">
        <p className="desktop-settings-help">{message("settings.privacy.appStore")}</p>
      </div>
    );
  }
  const { enabled } = setting;
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
export function privacySummary(setting: CrashSetting): string {
  if (!setting.available) return message("settings.privacy.summary.appStore");
  return message(setting.enabled ? "settings.privacy.summary.on" : "settings.privacy.summary.off");
}

function reportError(error: unknown): void {
  console.error("crash reports failed", error);
}
