// The app's look and its settings (R10, plan §5.5): the appearance is read from localStorage,
// applied to the document element, saved again on every change, and set before the first paint by
// public/appearance.js, which reads the same key. The View settings popover, the Settings dialog
// and the commands that open or change them.

import {
  type Accent,
  type Appearance,
  AppearanceSettings,
  applyAppearance,
  type CommandId,
  message,
  parseAppearance,
  SettingsDialog,
  SettingsRegistry,
  ShortcutsDialog,
  type UiTheme,
  usePhone,
  useSettingsSection,
  ViewSettingsButton,
} from "@laminapdf/ui";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import type { PageAppearanceControls } from "./page-appearance.tsx";

/** Where the appearance is kept; public/appearance.js reads the same key before the first paint. */
const APPEARANCE_KEY = "lamina.appearance";

/** The stored appearance, or the default where storage is blocked or holds nothing. */
function readAppearance(): Appearance {
  try {
    return parseAppearance(localStorage.getItem(APPEARANCE_KEY));
  } catch {
    return parseAppearance(null);
  }
}

function saveAppearance(appearance: Appearance): void {
  try {
    localStorage.setItem(APPEARANCE_KEY, JSON.stringify(appearance));
  } catch {
    // Storage blocked (a private window, a policy): the choice lasts until the tab closes.
  }
}

/** The settings' state, the commands' handlers and what the app renders for them. */
export interface Settings {
  readonly appearance: Appearance;
  readonly actions: Readonly<Partial<Record<CommandId, (args: unknown) => void>>>;
  /** The top bar's View button with its popover. */
  readonly viewButton: ReactNode;
  /** The Settings and Keyboard shortcuts dialogs. */
  readonly dialogs: ReactNode;
  /** The Settings dialog's sections, which other parts of the app add to. */
  readonly registry: SettingsRegistry;
}

/**
 * The appearance, the View settings popover (with `page`'s Page appearance section), the Settings
 * dialog with its Appearance section and the Keyboard shortcuts dialog, which offers `openPalette`
 * when nothing matches.
 */
export function useSettings(openPalette: () => void, page: PageAppearanceControls): Settings {
  const [appearance, setAppearance] = useState(readAppearance);
  const [viewOpen, setViewOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [registry] = useState(() => new SettingsRegistry());
  // A phone's top bar has no View button to anchor the popover: View settings opens Settings.
  const phone = useRef(false);
  phone.current = usePhone();
  useEffect(() => {
    applyAppearance(document.documentElement, appearance);
    saveAppearance(appearance);
  }, [appearance]);
  useSettingsSection(registry, "appearance", message("settings.appearance"), () => (
    <AppearanceSettings appearance={appearance} onChange={setAppearance} />
  ));
  useSettingsSection(registry, "about", message("about.title"), About);
  const actions = useMemo(
    () => ({
      "view.settings": () => (phone.current ? setSettingsOpen(true) : setViewOpen(true)),
      "app.settings": () => setSettingsOpen(true),
      "help.shortcuts": () => setShortcutsOpen(true),
      "view.theme": (theme: unknown) => setAppearance((a) => ({ ...a, theme: theme as UiTheme })),
      "view.accent": (accent: unknown) => setAppearance((a) => ({ ...a, accent: accent as Accent })),
      ...page.actions,
    }),
    [page.actions],
  );
  const viewButton = (
    <ViewSettingsButton
      appearance={appearance}
      onChange={setAppearance}
      isOpen={viewOpen}
      onOpenChange={setViewOpen}
      onMoreSettings={() => setSettingsOpen(true)}
      page={page.section}
    />
  );
  const dialogs = (
    <>
      <SettingsDialog registry={registry} isOpen={settingsOpen} onOpenChange={setSettingsOpen} />
      <ShortcutsDialog
        isOpen={shortcutsOpen}
        onOpenChange={setShortcutsOpen}
        singleKeys={appearance.singleKeys}
        onSingleKeysChange={(singleKeys) => setAppearance((a) => ({ ...a, singleKeys }))}
        onOpenPalette={openPalette}
      />
    </>
  );
  return { appearance, actions, viewButton, dialogs, registry };
}

/** Settings › About: the reader's version and where its privacy promises are written down. */
function About(): ReactNode {
  return (
    <div className="paper-settings-fields">
      <p>{message("about.version", { version: __APP_VERSION__ })}</p>
      <p>{message("about.free")}</p>
      <p>
        <a href="https://laminapdf.com/privacy/" target="_blank" rel="noopener noreferrer">
          {message("about.privacy")}
        </a>
      </p>
    </div>
  );
}
