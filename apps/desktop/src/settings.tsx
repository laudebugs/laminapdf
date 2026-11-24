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
  type Accent,
  type Appearance,
  AppearanceSettings,
  appearanceSummary,
  applyAppearance,
  Button,
  type CommandContext,
  isAccent,
  isPageAppearanceName,
  isUiTheme,
  type Motion,
  message,
  PageAppearanceFields,
  type PageAppearanceTarget,
  type PlatformStorage,
  pageAppearanceName,
  resolvePageAppearance,
  SettingsDialog,
  SettingsRegistry,
  Switch,
  type ToastQueue,
  type UiTheme,
  useMediaQuery,
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
  /** Settings › Appearance: how document pages are drawn (`null` before the user chooses). */
  readonly pageAppearance: PageAppearance | null;
  /** Settings › Motion: the system's, or reduced whatever it says. */
  readonly motion: Motion;
  /** Settings › Appearance: whether single-key shortcuts are on (WCAG 2.1.4). */
  readonly singleKeys: boolean;
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
  pageAppearance: null,
  motion: "system",
  singleKeys: true,
  graphics: "automatic",
};

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
  const { preferences, patch } = prefs;
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
    () => <FileSettings preferences={preferences} onPatch={patch} onClearRecent={clearRecent} />,
    { shows: [preferences], summary: () => filesSummary(preferences) },
  );
  const look = useDesktopAppearance(shell, registry, prefs, viewer, toasts, open);
  useShellSections({ shell, registry, unshown, updates, toasts, open, graphics: preferences.graphics, patch });

  return {
    preferences,
    open,
    actions: look.actions,
    checked: look.checked,
    singleKeys: preferences.singleKeys,
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
): Pick<DesktopSettings, "actions" | "checked" | "setSingleKeys" | "viewButton"> {
  const [viewOpen, setViewOpen] = useState(false);
  const forced = useMediaQuery("(forced-colors: active)");
  const pageAppearance = resolvePageAppearance(
    { all: preferences.pageAppearance ?? undefined, documents: {} },
    undefined,
    forced,
  );
  const appearance = useMemo<Appearance>(
    () => ({
      theme: preferences.theme,
      accent: preferences.accent,
      motion: preferences.motion,
      singleKeys: preferences.singleKeys,
    }),
    [preferences.theme, preferences.accent, preferences.motion, preferences.singleKeys],
  );
  const effectiveMotion = useMotion(shell, preferences.motion);
  useEffect(() => {
    applyAppearance(document.documentElement, { ...appearance, motion: effectiveMotion });
  }, [appearance, effectiveMotion]);
  usePageAppearanceValue(viewer, pageAppearance);
  const changeAppearance = useCallback(
    (next: Appearance) =>
      patch({ theme: next.theme, accent: next.accent, motion: next.motion, singleKeys: next.singleKeys }),
    [patch],
  );
  const setSingleKeys = useCallback((singleKeys: boolean) => patch({ singleKeys }), [patch]);
  const pickPageAppearance = usePageAppearancePicker(latestRef, patch, toasts, forced);
  useSettingsSection(
    registry,
    "appearance",
    message("settings.appearance"),
    () => (
      <div className="paper-settings-fields">
        <AppearanceSettings appearance={appearance} onChange={changeAppearance} />
        <PageAppearanceFields value={pageAppearance} onChange={(next) => pickPageAppearance(next, false)} />
      </div>
    ),
    { shows: [appearance, pageAppearance], summary: () => appearanceSummary(appearance) },
  );
  const { actions, checked } = useViewCommands({
    theme: preferences.theme,
    accent: preferences.accent,
    pageAppearance,
    openSettings,
    setViewOpen,
    patch,
    pickPageAppearance,
  });
  const viewButton = (
    <ViewSettingsButton
      appearance={appearance}
      onChange={changeAppearance}
      isOpen={viewOpen}
      onOpenChange={setViewOpen}
      onMoreSettings={() => openSettings("appearance")}
      page={<PageAppearanceFields value={pageAppearance} onChange={(next) => pickPageAppearance(next, true)} />}
    />
  );
  return { actions, checked, setSingleKeys, viewButton };
}

/** Command handlers and native menu checked state for View ▸ Theme, Accent and Page appearance. */
function useViewCommands({
  theme,
  accent,
  pageAppearance,
  openSettings,
  setViewOpen,
  patch,
  pickPageAppearance,
}: {
  theme: UiTheme;
  accent: Accent;
  pageAppearance: PageAppearance;
  openSettings: (section?: string) => void;
  setViewOpen: (open: boolean) => void;
  patch: (update: Partial<Preferences>) => void;
  pickPageAppearance: (next: PageAppearance, showToast: boolean) => void;
}): Pick<DesktopSettings, "actions" | "checked"> {
  const isPhone = usePhone();
  const actions = useMemo<NonNullable<CommandContext["actions"]>>(
    () => ({
      "view.settings": () => (isPhone ? openSettings("appearance") : setViewOpen(true)),
      "view.theme": (next: unknown) => {
        if (isUiTheme(next)) patch({ theme: next });
      },
      "view.accent": (next: unknown) => {
        if (isAccent(next)) patch({ accent: next });
      },
      "view.page-appearance": (next: unknown) => {
        if (isPageAppearanceName(next)) pickPageAppearance(next, true);
      },
    }),
    [isPhone, openSettings, setViewOpen, patch, pickPageAppearance],
  );
  const checked = useMemo<NonNullable<CommandContext["checked"]>>(
    () => ({
      "view.theme": theme,
      "view.accent": accent,
      "view.page-appearance": pageAppearance,
    }),
    [theme, accent, pageAppearance],
  );
  return { actions, checked };
}

/** Updates `pageAppearance` in preferences and, outside the modal Settings dialog, posts an Undo toast (R10). */
function usePageAppearancePicker(
  latestRef: { readonly current: Preferences },
  patch: (update: Partial<Preferences>) => void,
  toasts: ToastQueue,
  forced: boolean,
): (next: PageAppearance, showToast: boolean) => void {
  return useCallback(
    (next: PageAppearance, showToast: boolean) => {
      const beforeStored = latestRef.current.pageAppearance;
      const beforeShown = resolvePageAppearance({ all: beforeStored ?? undefined, documents: {} }, undefined, forced);
      if (beforeShown === next && beforeStored !== null) return;
      patch({ pageAppearance: next });
      if (!showToast) return;
      toasts.add({
        message: message("pageAppearance.toast.all", { name: pageAppearanceName(next) }),
        action: {
          label: message("pageAppearance.undo"),
          onPress: () => {
            if (latestRef.current.pageAppearance === next) patch({ pageAppearance: beforeStored });
          },
        },
      });
    },
    [latestRef, patch, toasts, forced],
  );
}

interface PreferencesHandle {
  readonly preferences: Preferences;
  readonly patch: (update: Partial<Preferences>) => void;
  readonly latestRef: { readonly current: Preferences };
}

/**
 * The main process's preferences, kept current when another window changes them, and `patch`,
 * which merges a change onto the latest preferences and writes it through the main process.
 * Patches made before the initial read resolves are queued and merged onto the loaded file so
 * startup actions never overwrite persisted preferences with defaults.
 */
function usePreferences(
  shell: DesktopShell | undefined,
  storage: PlatformStorage,
  onChange: () => void,
): PreferencesHandle {
  const [preferences, setPreferences] = useState(DEFAULT_PREFERENCES);
  const latestRef = useRef(DEFAULT_PREFERENCES);
  const loadedRef = useRef(shell === undefined);
  const pendingRef = useRef<Partial<Preferences> | null>(null);
  const onChangeRef = useRef(onChange);
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);
  useEffect(() => {
    if (shell === undefined) {
      loadedRef.current = true;
      return;
    }
    let cancelled = false;
    let receivedEvent = false;
    const apply = (next: Preferences) => {
      if (cancelled) return;
      latestRef.current = next;
      setPreferences(next);
    };
    shell.invoke<Preferences>("preferences").then((disk) => {
      if (cancelled || receivedEvent) return;
      loadedRef.current = true;
      const pending = pendingRef.current;
      pendingRef.current = null;
      if (pending === null) {
        apply(disk);
        return;
      }
      const merged = { ...disk, ...pending };
      apply(merged);
      setPreferencesIn(shell, storage, disk, merged).then(() => onChangeRef.current(), reportError);
    }, reportError);
    const unlisten = shell.listen<Preferences>(PREFERENCES_EVENT, (next) => {
      receivedEvent = true;
      loadedRef.current = true;
      pendingRef.current = null;
      apply(next);
    });
    return () => {
      cancelled = true;
      void unlisten.then((stop) => stop(), reportError);
    };
  }, [shell, storage]);
  const patch = useCallback(
    (update: Partial<Preferences>) => {
      const was = latestRef.current;
      const next = { ...was, ...update };
      latestRef.current = next;
      setPreferences(next);
      if (shell === undefined) return;
      if (!loadedRef.current) {
        pendingRef.current = { ...pendingRef.current, ...update };
        return;
      }
      setPreferencesIn(shell, storage, was, next).then(() => onChangeRef.current(), reportError);
    },
    [shell, storage],
  );
  return { preferences, patch, latestRef };
}

/** Settings › Files: remembering recent files, reloading changed ones, and where files open. */
function FileSettings({
  preferences,
  onPatch,
  onClearRecent,
}: {
  preferences: Preferences;
  onPatch: (update: Partial<Preferences>) => void;
  onClearRecent: () => void;
}): ReactNode {
  return (
    <div className="paper-settings-fields">
      <Switch isSelected={preferences.rememberRecent} onChange={(rememberRecent) => onPatch({ rememberRecent })}>
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
        onChange={(reopenLast) => onPatch({ reopenLast })}
      >
        {message("settings.files.reopenLast")}
      </Switch>
      <p className="desktop-settings-help">
        {message(
          preferences.rememberRecent ? "settings.files.reopenLast.help" : "settings.files.reopenLast.needsRecent",
        )}
      </p>
      <Switch isSelected={preferences.reloadOnChange} onChange={(reloadOnChange) => onPatch({ reloadOnChange })}>
        {message("settings.files.reloadOnChange")}
      </Switch>
      <p className="desktop-settings-help">{message("settings.files.reloadOnChange.help")}</p>
      <Switch
        isSelected={preferences.openIn === "windows"}
        onChange={(on) => onPatch({ openIn: on ? "windows" : "tabs" })}
      >
        {message("settings.files.openInWindows")}
      </Switch>
      <p className="desktop-settings-help">{message("settings.files.openInWindows.help")}</p>
      <Switch isSelected={preferences.allowDuplicates} onChange={(allowDuplicates) => onPatch({ allowDuplicates })}>
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
