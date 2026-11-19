// The desktop app's Settings dialog (S07): its Files section, with the choices the main process
// keeps in preferences.json and acts on (src-tauri/src/prefs.rs): remembering recent files, which
// turned off clears the app's and the OS's recent lists and the last session, reopening the last
// session (which needs the recent files remembered), reloading open files that change on disk,
// opening files in tabs or windows, and opening a file that is open already; Appearance and
// accessibility's Motion, kept beside them so every window shares it (motion.ts applies it); its
// Updates section (S06), its Privacy section (crash reports), on Linux Graphics (graphics.tsx),
// which the next start reads, and About (about.tsx). The full settings page adds its
// other sections to the same registry.

import {
  Button,
  type Motion,
  MotionField,
  message,
  SettingsDialog,
  SettingsRegistry,
  Switch,
  type ToastQueue,
  useSettingsSection,
} from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import { useAboutSection } from "./about.tsx";
import { PrivacySettings, privacySummary, useCrashSetting } from "./crashes.tsx";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";
import { type Graphics, useGraphicsSection } from "./graphics.tsx";
import { useMotion } from "./motion.ts";
import { words } from "./update-words.ts";
import { type DesktopUpdates, UpdateSettings, updatesSummary } from "./updates.tsx";

/** The main process's preferences (`Preferences` in src-tauri/src/prefs.rs). */
interface Preferences {
  readonly rememberRecent: boolean;
  readonly reloadOnChange: boolean;
  /** Where the files the user opens go: tabs in the frontmost window, or a window each. */
  readonly openIn: "tabs" | "windows";
  /** Whether a file that is open already opens again, rather than showing its tab. */
  readonly allowDuplicates: boolean;
  /** Whether the last session's windows and tabs come back on launch; kept only with `rememberRecent`. */
  readonly reopenLast: boolean;
  /** Settings › Motion: the system's, or reduced whatever it says. */
  readonly motion: Motion;
  /** Settings › Graphics (Linux): the WebKitGTK workarounds the next start runs with. */
  readonly graphics: Graphics;
}

/** Settings › Files' button; desktop-only words stay here, out of the shared messages. */
const CLEAR_RECENT = "Clear recent files";

/** The main process's event with the preferences, after any window changed them (`commands.rs`). */
const PREFERENCES_EVENT = "preferences://changed";

/** Until the main process answers, and outside the shell: its defaults. */
const DEFAULTS: Preferences = {
  rememberRecent: true,
  reloadOnChange: true,
  openIn: "tabs",
  allowDuplicates: false,
  reopenLast: true,
  motion: "system",
  graphics: "automatic",
};

/** What the settings give the app. */
export interface DesktopSettings {
  readonly preferences: Preferences;
  /** Opens the Settings dialog (`app.settings`), at `section` when given (`updates`). */
  readonly open: (section?: string) => void;
  /** The dialog, rendered once by the app. */
  readonly dialog: ReactNode;
}

/**
 * The preferences, read from the main process and set through it, and the Settings dialog with
 * the Files section. `onChange` runs after each change the main process has applied.
 */
export function useDesktopSettings(
  shell: DesktopShell | undefined,
  onChange: () => void,
  updates: DesktopUpdates,
  toasts: ToastQueue,
): DesktopSettings {
  const { preferences, change } = usePreferences(shell, onChange);
  const [isOpen, setOpen] = useState(false);
  const [section, setSection] = useState<string>();
  const [registry] = useState(() => new SettingsRegistry());
  // Outside the shell nothing updates, so the Updates section goes to a registry nobody shows.
  const [unshown] = useState(() => new SettingsRegistry());
  const open = useCallback((at?: string) => {
    setSection(at);
    setOpen(true);
  }, []);
  const clearRecent = useCallback(() => {
    shell?.invoke("recent_clear").then(onChange, reportError);
  }, [shell, onChange]);
  useSettingsSection(
    registry,
    "files",
    message("settings.files"),
    () => <FileSettings preferences={preferences} onChange={change} onClearRecent={clearRecent} />,
    { shows: [preferences], summary: () => filesSummary(preferences) },
  );
  useMotion(shell, preferences.motion);
  useSettingsSection(
    registry,
    "appearance",
    message("settings.appearance"),
    () => (
      <div className="paper-settings-fields">
        <MotionField value={preferences.motion} onChange={(motion) => change({ ...preferences, motion })} />
      </div>
    ),
    { shows: [preferences], summary: () => message(`appearance.motion.${preferences.motion}`) },
  );
  // S06: Settings › Updates, inside the shell only (the main process owns updates).
  useSettingsSection(
    shell === undefined ? unshown : registry,
    "updates",
    words("title"),
    () => <UpdateSettings updates={updates} />,
    { shows: [updates], summary: () => updatesSummary(updates.status) },
  );
  // Settings › Graphics, on Linux only (the main process says whether it applies).
  useGraphicsSection({ shell, shown: registry, unshown, toasts, open }, preferences.graphics, (graphics) =>
    change({ ...preferences, graphics }),
  );
  // S07: Settings › Privacy, inside the shell only (the main process keeps the crash reports).
  const [crashReports, setCrashReports] = useCrashSetting(shell);
  useSettingsSection(
    shell === undefined ? unshown : registry,
    "privacy",
    message("settings.privacy"),
    () =>
      shell === undefined || crashReports === undefined ? null : (
        <PrivacySettings shell={shell} setting={crashReports} onChange={setCrashReports} />
      ),
    { shows: [crashReports], summary: () => privacySummary(crashReports) },
  );
  // S07: Settings › About, inside the shell only.
  useAboutSection(shell, shell === undefined ? unshown : registry, updates.status);
  return {
    preferences,
    open,
    dialog: <SettingsDialog registry={registry} isOpen={isOpen} onOpenChange={setOpen} section={section} />,
  };
}

/**
 * The main process's preferences, kept current when another window changes them, and `change`,
 * which sets them there; `onChange` runs once the main process has applied a change.
 */
function usePreferences(
  shell: DesktopShell | undefined,
  onChange: () => void,
): { preferences: Preferences; change: (value: Preferences) => void } {
  const [preferences, setPreferences] = useState(DEFAULTS);
  useEffect(() => {
    if (shell === undefined) return;
    shell.invoke<Preferences>("preferences").then(setPreferences, reportError);
    // A change made in another window (src-tauri's PREFERENCES_EVENT) shows here too.
    const unlisten = shell.listen<Preferences>(PREFERENCES_EVENT, setPreferences);
    return () => void unlisten.then((stop) => stop(), reportError);
  }, [shell]);
  const change = useCallback(
    (value: Preferences) => {
      setPreferences(value);
      shell?.invoke("set_preferences", { value }).then(onChange, reportError);
    },
    [shell, onChange],
  );
  return { preferences, change };
}

/** Settings › Files: remembering recent files, reloading changed ones, and where files open. */
function FileSettings({
  preferences,
  onChange,
  onClearRecent,
}: {
  preferences: Preferences;
  onChange: (value: Preferences) => void;
  onClearRecent: () => void;
}): ReactNode {
  return (
    <div className="paper-settings-fields">
      <Switch
        isSelected={preferences.rememberRecent}
        onChange={(on) => onChange({ ...preferences, rememberRecent: on })}
      >
        {message("settings.files.rememberRecent")}
      </Switch>
      <p className="desktop-settings-help">{message("settings.files.rememberRecent.help")}</p>
      <div>
        <Button isDisabled={!preferences.rememberRecent} onPress={onClearRecent}>
          {CLEAR_RECENT}
        </Button>
      </div>
      <Switch
        isSelected={preferences.rememberRecent && preferences.reopenLast}
        isDisabled={!preferences.rememberRecent}
        onChange={(on) => onChange({ ...preferences, reopenLast: on })}
      >
        {message("settings.files.reopenLast")}
      </Switch>
      <p className="desktop-settings-help">
        {message(
          preferences.rememberRecent ? "settings.files.reopenLast.help" : "settings.files.reopenLast.needsRecent",
        )}
      </p>
      <Switch
        isSelected={preferences.reloadOnChange}
        onChange={(on) => onChange({ ...preferences, reloadOnChange: on })}
      >
        {message("settings.files.reloadOnChange")}
      </Switch>
      <p className="desktop-settings-help">{message("settings.files.reloadOnChange.help")}</p>
      <Switch
        isSelected={preferences.openIn === "windows"}
        onChange={(on) => onChange({ ...preferences, openIn: on ? "windows" : "tabs" })}
      >
        {message("settings.files.openInWindows")}
      </Switch>
      <p className="desktop-settings-help">{message("settings.files.openInWindows.help")}</p>
      <Switch
        isSelected={preferences.allowDuplicates}
        onChange={(on) => onChange({ ...preferences, allowDuplicates: on })}
      >
        {message("settings.files.allowDuplicates")}
      </Switch>
      <p className="desktop-settings-help">{message("settings.files.allowDuplicates.help")}</p>
    </div>
  );
}

/** The Files section in one line, for a narrow window's list of sections. */
function filesSummary({ rememberRecent, reloadOnChange }: Preferences): string {
  if (rememberRecent && reloadOnChange) return message("settings.files.summary.both");
  if (rememberRecent) return message("settings.files.summary.recent");
  return message(reloadOnChange ? "settings.files.summary.reload" : "settings.files.summary.none");
}

function reportError(error: unknown): void {
  console.error("the preferences failed", error);
}
