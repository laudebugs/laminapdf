// The desktop app's Settings dialog (S07): for now its Files section, with the choices the main
// process keeps in preferences.json and acts on (src-tauri/src/prefs.rs): remembering recent
// files, which turned off clears the app's and the OS's recent lists, reloading open files that
// change on disk, opening files in tabs or windows, and opening a file that is open already. The
// full settings page adds its other sections to the same registry.

import { message, SettingsDialog, SettingsRegistry, Switch, useSettingsSection } from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";

/** The main process's preferences (`Preferences` in src-tauri/src/prefs.rs). */
interface Preferences {
  readonly rememberRecent: boolean;
  readonly reloadOnChange: boolean;
  /** Where the files the user opens go: tabs in the frontmost window, or a window each. */
  readonly openIn: "tabs" | "windows";
  /** Whether a file that is open already opens again, rather than showing its tab. */
  readonly allowDuplicates: boolean;
}

/** Until the main process answers, and outside the shell: its defaults. */
const DEFAULTS: Preferences = { rememberRecent: true, reloadOnChange: true, openIn: "tabs", allowDuplicates: false };

/** What the settings give the app. */
export interface DesktopSettings {
  readonly preferences: Preferences;
  /** Opens the Settings dialog (`app.settings`). */
  readonly open: () => void;
  /** The dialog, rendered once by the app. */
  readonly dialog: ReactNode;
}

/**
 * The preferences, read from the main process and set through it, and the Settings dialog with
 * the Files section. `onChange` runs after each change the main process has applied.
 */
export function useDesktopSettings(shell: DesktopShell | undefined, onChange: () => void): DesktopSettings {
  const [preferences, setPreferences] = useState(DEFAULTS);
  const [isOpen, setOpen] = useState(false);
  const [registry] = useState(() => new SettingsRegistry());
  useEffect(() => {
    shell?.invoke<Preferences>("preferences").then(setPreferences, reportError);
  }, [shell]);
  const change = useCallback(
    (value: Preferences) => {
      setPreferences(value);
      shell?.invoke("set_preferences", { value }).then(onChange, reportError);
    },
    [shell, onChange],
  );
  useSettingsSection(
    registry,
    "files",
    message("settings.files"),
    () => <FileSettings preferences={preferences} onChange={change} />,
    { shows: [preferences], summary: () => filesSummary(preferences) },
  );
  return {
    preferences,
    open: () => setOpen(true),
    dialog: <SettingsDialog registry={registry} isOpen={isOpen} onOpenChange={setOpen} />,
  };
}

/** Settings › Files: remembering recent files, reloading changed ones, and where files open. */
function FileSettings({
  preferences,
  onChange,
}: {
  preferences: Preferences;
  onChange: (value: Preferences) => void;
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
