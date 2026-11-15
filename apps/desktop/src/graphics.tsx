// Settings › Graphics, on Linux only (S07's renderer switch): which WebKitGTK workarounds the app
// starts with (src-tauri/src/graphics.rs). They take effect at the next start, so a change offers
// Restart now; and when two starts in a row failed, the main process switched to Compatibility and
// this section says so, once. The words stay in the desktop app, as Updates' do.

import { Button, Notice, Radio, RadioGroup, type SettingsRegistry, useSettingsSection } from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";

/** Settings › Graphics (`Graphics` in src-tauri/src/graphics.rs). */
export type Graphics = "automatic" | "compatibility" | "safe";

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
  flag: "Started with --safe-graphics: Safe mode is on until the next start.",
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

/**
 * Registers Settings › Graphics in `registries.shown` where it applies (Linux), else in
 * `registries.unshown`, which no dialog shows. `saved` is the preferences' choice; `choose` saves another.
 */
export function useGraphicsSection(
  shell: DesktopShell | undefined,
  registries: { readonly shown: SettingsRegistry; readonly unshown: SettingsRegistry },
  saved: Graphics,
  choose: (value: Graphics) => void,
): void {
  const status = useGraphicsStatus(shell);
  const restart = useCallback(async () => (await shell?.invoke<boolean>("restart_app")) ?? false, [shell]);
  const noticeSeen = useCallback(() => {
    shell?.invoke("graphics_notice_seen").catch(reportError);
  }, [shell]);
  useSettingsSection(
    status?.applies === true ? registries.shown : registries.unshown,
    "graphics",
    words("title"),
    () =>
      status === undefined ? null : <GraphicsSettings controls={{ status, saved, choose, restart, noticeSeen }} />,
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
  /** The crash-loop notice has been shown. */
  readonly noticeSeen: () => void;
}

/** Settings › Graphics: the three choices, Restart now after a change, and the guard's notice. */
export function GraphicsSettings({ controls }: { controls: GraphicsControls }): ReactNode {
  const { status, saved, choose, restart, noticeSeen } = controls;
  const [blocked, setBlocked] = useState(false);
  const { fellBack } = status;
  useEffect(() => {
    if (fellBack) noticeSeen();
  }, [fellBack, noticeSeen]);
  // `--safe-graphics` runs Safe mode without saving it.
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
        {(["automatic", "compatibility", "safe"] as const).map((choice) => (
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
