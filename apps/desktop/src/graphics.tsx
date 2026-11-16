// Settings › Graphics, on Linux only (S07's renderer switch): which WebKitGTK workarounds the app
// starts with (src-tauri/src/graphics.rs). They take effect at the next start, so a change offers
// Restart now. When two starts in a row failed, the main process switched to Compatibility: the
// first window says so once at start, with a way to Settings, and this section says so for the
// rest of the run. The words stay in the desktop app, as Updates' do.

import {
  Button,
  Notice,
  Radio,
  RadioGroup,
  type SettingsRegistry,
  type ToastQueue,
  useSettingsSection,
} from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";

/** Settings › Graphics' choices, in the order it shows them (`Graphics` in src-tauri/src/graphics.rs,
 * whose test holds this list to the Rust enum). */
const GRAPHICS = ["automatic", "compatibility", "safe"] as const;
export type Graphics = (typeof GRAPHICS)[number];

/** What this start runs with (`Status` in src-tauri/src/graphics.rs). */
export interface GraphicsStatus {
  /** Whether the section applies: on Linux only. */
  readonly applies: boolean;
  /** The choice this start runs with. */
  readonly running: Graphics;
  /** The variables the app set for it. */
  readonly set: readonly string[];
  /** The crash-loop guard switched to Compatibility after two failed starts. */
  readonly fellBack: boolean;
}

const WORDS = {
  title: "Graphics",
  choice: "Drawing",
  automatic: "Automatic",
  "automatic.help": "Lamina PDF turns on the fixes your graphics driver is known to need.",
  compatibility: "Compatibility",
  "compatibility.help": "Draws without the DMABUF renderer. Try it if windows stay blank or flicker.",
  safe: "Safe mode",
  "safe.help": "Also turns compositing off. Slower; use it only if Compatibility doesn't help.",
  restartNeeded: "The change takes effect when Lamina PDF restarts.",
  restart: "Restart now",
  unsaved: "Save or close the documents with unsaved edits first, then restart.",
  fellBackTitle: "Lamina PDF switched to Compatibility.",
  fellBack: "It closed twice just after starting, so it now draws without the DMABUF renderer.",
  flag: "Started with --safe-graphics: Safe mode is on until you quit Lamina PDF.",
  openSettings: "Graphics settings",
  later: "Later",
  "summary.automatic": "Automatic",
  "summary.compatibility": "Compatibility",
  "summary.safe": "Safe mode",
} as const;

function words(id: keyof typeof WORDS): string {
  return WORDS[id];
}

/** The status from the main process; `undefined` outside the shell and until it answers. */
function useGraphicsStatus(shell: DesktopShell | undefined): GraphicsStatus | undefined {
  const [status, setStatus] = useState<GraphicsStatus>();
  useEffect(() => {
    shell?.invoke<GraphicsStatus>("graphics_status").then(setStatus, reportError);
  }, [shell]);
  return status;
}

/** Where Settings › Graphics goes, and what it may use. */
export interface GraphicsSectionHost {
  readonly shell: DesktopShell | undefined;
  /** The dialog's registry, where the section goes when it applies (Linux). */
  readonly shown: SettingsRegistry;
  /** The registry no dialog shows, where it goes elsewhere. */
  readonly unshown: SettingsRegistry;
  /** The queue the start notice goes to. */
  readonly toasts: ToastQueue;
  /** Opens Settings at a section, for the start notice's action. */
  readonly open: (section: string) => void;
}

/**
 * Registers Settings › Graphics, and says once at start when the crash-loop guard switched to
 * Compatibility. `saved` is the preferences' choice; `choose` saves another.
 */
export function useGraphicsSection(
  host: GraphicsSectionHost,
  saved: Graphics,
  choose: (value: Graphics) => void,
): void {
  const { shell, shown, unshown, toasts, open } = host;
  const status = useGraphicsStatus(shell);
  const restart = useCallback(async () => (await shell?.invoke<boolean>("restart_app")) ?? false, [shell]);
  useEffect(() => {
    if (status?.fellBack !== true) return;
    shell?.invoke<boolean>("graphics_take_notice").then((tell) => {
      if (!tell) return;
      toasts.add({
        message: `${words("fellBackTitle")} ${words("fellBack")}`,
        action: { label: words("openSettings"), onPress: () => open("graphics") },
        dismissLabel: words("later"),
        timeout: Number.POSITIVE_INFINITY,
      });
    }, reportError);
  }, [status, shell, toasts, open]);
  useSettingsSection(
    status?.applies === true ? shown : unshown,
    "graphics",
    words("title"),
    () => (status === undefined ? null : <GraphicsSettings controls={{ status, saved, choose, restart }} />),
    { shows: [saved, status], summary: () => words(`summary.${saved}`) },
  );
}

/** What the section does with the shell. */
export interface GraphicsControls {
  /** This start's status. */
  readonly status: GraphicsStatus;
  /** The saved choice, for the next start. */
  readonly saved: Graphics;
  /** Saves another choice. */
  readonly choose: (value: Graphics) => void;
  /** Restarts the app: `false` while a window holds unsaved edits. */
  readonly restart: () => Promise<boolean>;
}

/** Settings › Graphics: the three choices, Restart now after a change, and the guard's notice. */
export function GraphicsSettings({ controls }: { controls: GraphicsControls }): ReactNode {
  const { status, saved, choose, restart } = controls;
  const [blocked, setBlocked] = useState(false);
  const { fellBack } = status;
  // `--safe-graphics` runs Safe mode without saving it; a restart keeps the flag, so none is offered.
  const flagged = status.running === "safe" && saved !== "safe";
  const changed = !flagged && saved !== status.running;
  return (
    <div className="paper-settings-fields">
      {fellBack ? (
        <Notice tone="warning" title={words("fellBackTitle")}>
          {words("fellBack")}
        </Notice>
      ) : null}
      {flagged ? <p className="desktop-settings-help">{words("flag")}</p> : null}
      <RadioGroup label={words("choice")} value={saved} onChange={(value) => choose(value as Graphics)}>
        {GRAPHICS.map((choice) => (
          <Radio key={choice} value={choice}>
            {words(choice)}
            <span className="desktop-radio-help">{words(`${choice}.help`)}</span>
          </Radio>
        ))}
      </RadioGroup>
      {changed ? (
        <div className="desktop-update-check">
          <span className="desktop-settings-help">{words(blocked ? "unsaved" : "restartNeeded")}</span>
          <Button
            variant="primary"
            onPress={() => {
              restart().then((restarted) => setBlocked(!restarted), reportError);
            }}
          >
            {words("restart")}
          </Button>
        </div>
      ) : null}
    </div>
  );
}

function reportError(error: unknown): void {
  console.error("graphics settings failed", error);
}
