// The desktop app's Settings dialog (S07) and View settings popover (R10): its Files section,
// with the choices the main process keeps in preferences.json and acts on (src-tauri/src/prefs.rs):
// remembering recent files, reopening the last session, reloading open files that change on disk,
// opening files in tabs or windows, and opening a file that is open already; Appearance and
// accessibility (theme, accent, page appearance, motion, single-key shortcuts), kept in
// preferences.json so every window and tab shares them and applied through `@laminapdf/ui`'s
// `applyAppearance` and `usePageAppearanceValue`; Updates (S06), Privacy (crash reports), on Linux
// Graphics (graphics.tsx), and About (about.tsx).

import type { PageAppearance } from "@laminapdf/tokens";
import {
  ACCENTS,
  type Accent,
  type Appearance,
  AppearanceSettings,
  appearanceSummary,
  applyAppearance,
  Button,
  type CommandContext,
  isPageAppearanceName,
  type Motion,
  message,
  PageAppearanceFields,
  type PageAppearanceTarget,
  type PlatformStorage,
  pageAppearanceName,
  SettingsDialog,
  SettingsRegistry,
  Switch,
  type ToastQueue,
  UI_THEMES,
  type UiTheme,
  usePageAppearanceValue,
  usePhone,
  useSettingsSection,
  ViewSettingsButton,
} from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useAboutSection } from "./about.tsx";
import { PrivacySettings, privacySummary, useCrashSetting } from "./crashes.tsx";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";
import { type Graphics, useGraphicsSection } from "./graphics.tsx";
import { useMotion } from "./motion.ts";
import { clearRecentList, setPreferences as setPreferencesIn } from "./recent-picker.ts";
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
  /** Settings › Appearance: the UI theme (`light`, `dark`, `high-contrast`, or `system`). */
  readonly theme: UiTheme;
  /** Settings › Appearance: the accent colour (`teal`, `blue`, `plum`, or `green`). */
  readonly accent: Accent;
  /** Settings › Appearance: how document pages are drawn (`original`, `sepia`, `dark`, or `high-contrast`). */
  readonly pageAppearance: PageAppearance;
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
const DEFAULT_PREFERENCES: Preferences = {
  rememberRecent: true,
  reloadOnChange: true,
  openIn: "tabs",
  allowDuplicates: false,
  reopenLast: true,
  theme: "system",
  accent: "teal",
  pageAppearance: "original",
  motion: "system",
  graphics: "automatic",
};

/** Whether `value` is a known UI theme. */
function isUiTheme(value: unknown): value is UiTheme {
  return UI_THEMES.includes(value as UiTheme);
}

/** Whether `value` is a known accent colour. */
function isAccent(value: unknown): value is Accent {
  return ACCENTS.includes(value as Accent);
}

/** What the settings give the app. */
export interface DesktopSettings {
  readonly preferences: Preferences;
  /** Opens the Settings dialog (`app.settings`), at `section` when given (`updates`, `appearance`). */
  readonly open: (section?: string) => void;
  /** View and appearance command handlers (`view.settings`, `view.theme`, `view.accent`, `view.page-appearance`). */
  readonly actions: NonNullable<CommandContext["actions"]>;
  /** Checked state for the native View menu's choice submenus. */
  readonly checked: NonNullable<CommandContext["checked"]>;
  /** Single-key shortcuts setting, shared with the Keyboard shortcuts dialog (WCAG 2.1.4). */
  readonly singleKeys: boolean;
  readonly setSingleKeys: (on: boolean) => void;
  /** The top bar's View settings button and popover. */
  readonly viewButton: ReactNode;
  /** The dialog, rendered once by the app. */
  readonly dialog: ReactNode;
}

/**
 * The preferences, read from the main process and set through it, the View settings popover,
 * command handlers and checked state for View ▸ Theme, Accent and Page appearance, and the Settings
 * dialog. `onChange` runs after each change the main process has applied.
 */
export function useDesktopSettings(
  shell: DesktopShell | undefined,
  storage: PlatformStorage,
  onChange: () => void,
  updates: DesktopUpdates,
  toasts: ToastQueue,
  viewer: PageAppearanceTarget | undefined,
): DesktopSettings {
  const prefs = usePreferences(shell, storage, onChange);
  const { preferences, change, patch } = prefs;
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
    if (shell !== undefined) clearRecentList(shell, storage).then(onChange, reportError);
  }, [shell, storage, onChange]);

  useSettingsSection(
    registry,
    "files",
    message("settings.files"),
    () => <FileSettings preferences={preferences} onChange={change} onClearRecent={clearRecent} />,
    { shows: [preferences], summary: () => filesSummary(preferences) },
  );
  const look = useDesktopAppearance(shell, registry, prefs, viewer, toasts, open);
  useShellSections({ shell, registry, unshown, updates, toasts, open, graphics: preferences.graphics, patch });

  return {
    preferences,
    open,
    actions: look.actions,
    checked: look.checked,
    singleKeys: look.singleKeys,
    setSingleKeys: look.setSingleKeys,
    viewButton: look.viewButton,
    dialog: <SettingsDialog registry={registry} isOpen={isOpen} onOpenChange={setOpen} section={section} />,
  };
}

/**
 * Registers the desktop-shell-only Settings sections (Updates, Linux Graphics, Privacy, About),
 * routing them to `unshown` outside Tauri.
 */
function useShellSections({
  shell,
  registry,
  unshown,
  updates,
  toasts,
  open,
  graphics,
  patch,
}: {
  shell: DesktopShell | undefined;
  registry: SettingsRegistry;
  unshown: SettingsRegistry;
  updates: DesktopUpdates;
  toasts: ToastQueue;
  open: (section?: string) => void;
  graphics: Graphics;
  patch: (update: Partial<Preferences>) => void;
}): void {
  const target = shell === undefined ? unshown : registry;
  useSettingsSection(target, "updates", words("title"), () => <UpdateSettings updates={updates} />, {
    shows: [updates],
    summary: () => updatesSummary(updates.status),
  });
  useGraphicsSection({ shell, shown: registry, unshown, toasts, open }, graphics, (next) => patch({ graphics: next }));
  const [crashReports, setCrashReports] = useCrashSetting(shell);
  useSettingsSection(
    target,
    "privacy",
    message("settings.privacy"),
    () =>
      shell === undefined || crashReports === undefined ? null : (
        <PrivacySettings shell={shell} setting={crashReports} onChange={setCrashReports} />
      ),
    { shows: [crashReports], summary: () => privacySummary(crashReports) },
  );
  useAboutSection(shell, target, updates.status);
}

/**
 * Applies theme, accent, motion and page appearance to the document root and `viewer`, registers
 * Settings › Appearance and accessibility, builds the View popover, and exposes the view commands
 * and their checked state.
 */
function useDesktopAppearance(
  shell: DesktopShell | undefined,
  registry: SettingsRegistry,
  { preferences, patch, latestRef }: PreferencesHandle,
  viewer: PageAppearanceTarget | undefined,
  toasts: ToastQueue,
  openSettings: (section?: string) => void,
): Pick<DesktopSettings, "actions" | "checked" | "singleKeys" | "setSingleKeys" | "viewButton"> {
  const [viewOpen, setViewOpen] = useState(false);
  const [singleKeys, setSingleKeys] = useState(true);
  const phone = useRef(false);
  phone.current = usePhone();
  const appearance = useMemo<Appearance>(
    () => ({ theme: preferences.theme, accent: preferences.accent, motion: preferences.motion, singleKeys }),
    [preferences.theme, preferences.accent, preferences.motion, singleKeys],
  );
  const effectiveMotion = useMotion(shell, preferences.motion);
  useEffect(() => {
    applyAppearance(document.documentElement, { ...appearance, motion: effectiveMotion });
  }, [appearance, effectiveMotion]);
  usePageAppearanceValue(viewer, preferences.pageAppearance);
  const changeAppearance = useCallback(
    (next: Appearance) => {
      setSingleKeys(next.singleKeys);
      const cur = latestRef.current;
      if (next.theme !== cur.theme || next.accent !== cur.accent || next.motion !== cur.motion) {
        patch({ theme: next.theme, accent: next.accent, motion: next.motion });
      }
    },
    [latestRef, patch],
  );
  const pickPageAppearance = usePageAppearancePicker(latestRef, patch, toasts);
  const pageSection = <PageAppearanceFields value={preferences.pageAppearance} onChange={pickPageAppearance} />;
  useSettingsSection(
    registry,
    "appearance",
    message("settings.appearance"),
    () => (
      <div className="paper-settings-fields">
        <AppearanceSettings appearance={appearance} onChange={changeAppearance} />
        {pageSection}
      </div>
    ),
    { shows: [appearance, preferences.pageAppearance], summary: () => appearanceSummary(appearance) },
  );
  const actions = useMemo<NonNullable<CommandContext["actions"]>>(
    () => ({
      "view.settings": () => (phone.current ? openSettings("appearance") : setViewOpen(true)),
      "view.theme": (theme: unknown) => {
        if (isUiTheme(theme)) patch({ theme });
      },
      "view.accent": (accent: unknown) => {
        if (isAccent(accent)) patch({ accent });
      },
      "view.page-appearance": (next: unknown) => {
        if (isPageAppearanceName(next)) pickPageAppearance(next);
      },
    }),
    [openSettings, patch, pickPageAppearance],
  );
  const checked = useMemo<NonNullable<CommandContext["checked"]>>(
    () => ({
      "view.theme": preferences.theme,
      "view.accent": preferences.accent,
      "view.page-appearance": preferences.pageAppearance,
    }),
    [preferences.theme, preferences.accent, preferences.pageAppearance],
  );
  const viewButton = (
    <ViewSettingsButton
      appearance={appearance}
      onChange={changeAppearance}
      isOpen={viewOpen}
      onOpenChange={setViewOpen}
      onMoreSettings={() => openSettings("appearance")}
      page={pageSection}
    />
  );
  return { actions, checked, singleKeys, setSingleKeys, viewButton };
}

/** Updates `pageAppearance` in preferences and posts an Undo toast (R10). */
function usePageAppearancePicker(
  latestRef: { readonly current: Preferences },
  patch: (update: Partial<Preferences>) => void,
  toasts: ToastQueue,
): (next: PageAppearance) => void {
  return useCallback(
    (next: PageAppearance) => {
      const before = latestRef.current.pageAppearance;
      if (before === next) return;
      patch({ pageAppearance: next });
      toasts.add({
        message: message("pageAppearance.toast.all", { name: pageAppearanceName(next) }),
        action: {
          label: message("pageAppearance.undo"),
          onPress: () => {
            if (latestRef.current.pageAppearance === next) patch({ pageAppearance: before });
          },
        },
      });
    },
    [latestRef, patch, toasts],
  );
}

interface PreferencesHandle {
  readonly preferences: Preferences;
  readonly change: (value: Preferences) => void;
  readonly patch: (update: Partial<Preferences>) => void;
  readonly latestRef: { readonly current: Preferences };
}

/**
 * The main process's preferences, kept current when another window changes them, and `change` /
 * `patch`, which set them there; `onChange` runs once the main process has applied a change.
 */
function usePreferences(
  shell: DesktopShell | undefined,
  storage: PlatformStorage,
  onChange: () => void,
): PreferencesHandle {
  const [preferences, setPreferences] = useState(DEFAULT_PREFERENCES);
  const latestRef = useRef(preferences);
  latestRef.current = preferences;
  useEffect(() => {
    if (shell === undefined) return;
    const apply = (next: Preferences) => {
      latestRef.current = next;
      setPreferences(next);
    };
    shell.invoke<Preferences>("preferences").then(apply, reportError);
    // A change made in another window (src-tauri's PREFERENCES_EVENT) shows here too.
    const unlisten = shell.listen<Preferences>(PREFERENCES_EVENT, apply);
    return () => void unlisten.then((stop) => stop(), reportError);
  }, [shell]);
  const change = useCallback(
    (value: Preferences) => {
      const was = latestRef.current;
      latestRef.current = value;
      setPreferences(value);
      if (shell !== undefined) setPreferencesIn(shell, storage, was, value).then(onChange, reportError);
    },
    [shell, storage, onChange],
  );
  const patch = useCallback(
    (update: Partial<Preferences>) => {
      change({ ...latestRef.current, ...update });
    },
    [change],
  );
  return { preferences, change, patch, latestRef };
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
