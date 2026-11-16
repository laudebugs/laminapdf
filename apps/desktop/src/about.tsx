// Settings › About (S07): the version and channel the update status reports (updates.tsx), and
// View licences, which opens the third-party notices release builds bundle (src-tauri's menu.rs,
// as Help ▸ View licences does). Its words stay in the desktop app, as Updates' do.

import { Button, type SettingsRegistry, useSettingsSection } from "@laminapdf/ui";
import { type ReactNode, useState } from "react";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";
import { fill } from "./update-words.ts";
import { channelName, type UpdateStatus } from "./updates.tsx";

const WORDS = {
  title: "About",
  version: "Lamina PDF {version}",
  channel: "{channel} channel",
  licence: "Open source, under the MIT or Apache-2.0 licence.",
  licences: "View licences",
  noLicences: "This build has no licence file: release builds do.",
} as const;

function words(id: keyof typeof WORDS, values: Readonly<Record<string, string>> = {}): string {
  return fill(WORDS[id], values);
}

/** Registers Settings › About in `registry` (an unshown one outside the shell). */
export function useAboutSection(
  shell: DesktopShell | undefined,
  registry: SettingsRegistry,
  status: UpdateStatus | undefined,
): void {
  useSettingsSection(
    registry,
    "about",
    words("title"),
    () => (shell === undefined ? null : <AboutSettings shell={shell} status={status} />),
    {
      shows: [status],
      summary: () => (status === undefined ? words("title") : words("version", { version: status.current })),
    },
  );
}

/** Settings › About. */
function AboutSettings({ shell, status }: { shell: DesktopShell; status: UpdateStatus | undefined }): ReactNode {
  const [missing, setMissing] = useState(false);
  return (
    <div className="paper-settings-fields">
      {status === undefined ? null : (
        <p className="desktop-update-title">
          {words("version", { version: status.current })} · {words("channel", { channel: channelName(status.channel) })}
        </p>
      )}
      <p className="desktop-settings-help">{words("licence")}</p>
      <div>
        <Button
          onPress={() => {
            shell.invoke<boolean>("open_licences").then(
              (opened) => setMissing(!opened),
              (error: unknown) => console.error("the licences failed", error),
            );
          }}
        >
          {words("licences")}
        </Button>
      </div>
      {missing ? <p className="desktop-settings-help">{words("noLicences")}</p> : null}
    </div>
  );
}
