// Updates (S06): the status the main process keeps (src-tauri/src/update), Settings › Updates, and
// the status bar's "Update ready" item with its details. The main process checks, downloads and
// installs; this side shows where it is and sends the user's choices, never a URL or the install ID.

import {
  Badge,
  Button,
  IconButton,
  message,
  Notice,
  Popover,
  PopoverTrigger,
  ProgressBar,
  Radio,
  RadioGroup,
  StatusBarItem,
  Switch,
  type ToastQueue,
} from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useMemo, useState } from "react";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";
import { words } from "./update-words.ts";

/** A channel an install follows. */
type Channel = "stable" | "beta" | "nightly";

/** Why automatic updates are off: the user, policy, an MSI install, a Linux package manager or the Mac App Store. */
type Off = "user" | "policy" | "msi" | "package-manager" | "app-store";

/** Where the update is (`State` in src-tauri/src/update/mod.rs). */
type UpdateState =
  | { readonly state: "idle" }
  | { readonly state: "checking" }
  | { readonly state: "downloading"; readonly version: string; readonly percent: number | null }
  | { readonly state: "ready"; readonly version: string; readonly notes: string | null }
  | { readonly state: "failed" };

/** The main process's update status (`UpdateStatus` in src-tauri/src/update/mod.rs). */
export interface UpdateStatus {
  readonly state: UpdateState;
  readonly current: string;
  readonly channel: Channel;
  readonly nightly: boolean;
  readonly enabled: boolean;
  readonly off: Off | null;
  readonly managed: boolean;
  /** Whether Check now may run: the main process's rule. */
  readonly canCheck: boolean;
  readonly stableAvailable: boolean;
  /** Seconds since the Unix epoch. */
  readonly lastChecked: number | null;
}

/** The event the main process sends every window when the status changes. */
const STATUS_EVENT = "update://status";

/** The status and the user's controls. */
export interface DesktopUpdates {
  /** Undefined until the main process answers, and outside the shell. */
  readonly status: UpdateStatus | undefined;
  readonly checkNow: () => void;
  readonly setEnabled: (enabled: boolean) => void;
  readonly setChannel: (channel: Channel) => void;
  readonly resetId: () => void;
  readonly restart: () => void;
  readonly openLink: (link: "privacy" | "whats-new") => void;
}

/** Follows the main process's update status and sends the user's choices to it. */
export function useDesktopUpdates(shell: DesktopShell | undefined, toasts: ToastQueue): DesktopUpdates {
  const [status, setStatus] = useState<UpdateStatus>();
  useEffect(() => {
    if (shell === undefined) return;
    let stop: (() => void) | undefined;
    let gone = false;
    shell.listen<UpdateStatus>(STATUS_EVENT, setStatus).then((unlisten) => {
      if (gone) unlisten();
      else stop = unlisten;
    }, reportError);
    shell.invoke<UpdateStatus>("update_status").then(setStatus, reportError);
    return () => {
      gone = true;
      stop?.();
    };
  }, [shell]);
  const call = useCallback(
    (command: string, args?: Record<string, unknown>) =>
      shell?.invoke<UpdateStatus | null>(command, args).then((next) => {
        if (next !== null && next !== undefined) setStatus(next);
      }, reportError),
    [shell],
  );
  return useMemo(
    () => ({
      status,
      checkNow: () => void call("update_check_now"),
      setEnabled: (enabled) => void call("update_set_enabled", { enabled }),
      setChannel: (channel) => void call("update_set_channel", { channel }),
      resetId: () => void call("update_reset_install_id")?.then(() => toasts.add({ message: words("resetDone") })),
      restart: () => void shell?.invoke("update_restart_to_install").catch(reportError),
      openLink: (link) => void shell?.invoke("update_open_link", { link }).catch(reportError),
    }),
    [status, call, shell, toasts],
  );
}

function reportError(error: unknown): void {
  console.error("an update call failed", error);
}

/** The channel's name, as Settings writes it. */
export function channelName(channel: Channel): string {
  return words(`channel.${channel}`);
}

/** When the last check ran, in words; "Not checked yet" before the first. */
function lastChecked(seconds: number | null): string {
  if (seconds === null) return words("neverChecked");
  const when = new Date(seconds * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  return words("lastChecked", { when });
}

/** Settings › Updates in one line, for a narrow window's list of sections. */
export function updatesSummary(status: UpdateStatus | undefined): string {
  if (status === undefined) return "";
  if (status.off === "package-manager") return words("summary.packageManager");
  if (status.off === "app-store") return words("summary.appStore");
  if (status.managed) return words("summary.managed");
  if (status.off !== null) return words("summary.off");
  return words("summary.on", { channel: channelName(status.channel) });
}

/** The card at the top of Settings › Updates: where the update is, and what the user can do now. */
function StatusCard({ status, updates }: { status: UpdateStatus; updates: DesktopUpdates }): ReactNode {
  const { state } = status;
  switch (state.state) {
    case "ready":
      return (
        <div className="desktop-update-card">
          <p className="desktop-update-title">{words("ready", { version: state.version })}</p>
          <p className="desktop-settings-help">{words("readyHelp")}</p>
          <Button variant="primary" onPress={updates.restart}>
            {message("command.app.restart-to-update")}
          </Button>
        </div>
      );
    case "downloading":
      return (
        <div className="desktop-update-card">
          <p className="desktop-update-title">{words("downloading", { version: state.version })}</p>
          <ProgressBar
            label={words("downloading", { version: state.version })}
            {...(state.percent === null ? {} : { value: state.percent })}
          />
          <p className="desktop-settings-help">{words("downloadingHelp", { current: status.current })}</p>
        </div>
      );
    case "checking":
      return (
        <div className="desktop-update-card">
          <ProgressBar label={words("checking")} />
        </div>
      );
    default:
      return (
        <div className="desktop-update-card">
          {state.state === "failed" ? <Notice tone="danger" title={words("failed")} /> : null}
          {status.off === null || status.off === "package-manager" ? (
            <p className="desktop-update-title">
              {/* "Up to date" only once a check has answered. */}
              {words(status.lastChecked === null ? "current" : "upToDate", {
                version: status.current,
              })}
            </p>
          ) : (
            <>
              <p className="desktop-update-title">{words("off")}</p>
              {status.off === "user" ? (
                <p className="desktop-settings-help">{words("offHelp", { current: status.current })}</p>
              ) : null}
            </>
          )}
        </div>
      );
  }
}

/** Settings › Updates (S06-update-settings). */
export function UpdateSettings({ updates }: { updates: DesktopUpdates }): ReactNode {
  const { status } = updates;
  if (status === undefined) return null;
  const busy = status.state.state === "checking" || status.state.state === "downloading";
  if (status.off === "app-store") {
    // The store updates this copy and the app checks nothing, so there is nothing to send either.
    return (
      <div className="paper-settings-fields">
        <p className="desktop-update-title">{words("appStore")}</p>
        <p className="desktop-settings-help">{words("appStoreHelp")}</p>
      </div>
    );
  }
  if (status.off === "package-manager") {
    return (
      <div className="paper-settings-fields">
        <p className="desktop-update-title">{words("packageManager")}</p>
        <p className="desktop-settings-help">{words("packageManagerHelp")}</p>
        <Sends updates={updates} />
      </div>
    );
  }
  return (
    <div className="paper-settings-fields">
      {status.managed ? (
        <div>
          <Badge tone="neutral">{words("managed")}</Badge>
          <p className="desktop-settings-help">
            {status.off === "policy" || status.off === "msi"
              ? words("managedOff")
              : words("managedChannel", { channel: channelName(status.channel) })}
          </p>
        </div>
      ) : null}
      <StatusCard status={status} updates={updates} />
      <Switch isSelected={status.enabled && !status.managed} isDisabled={status.managed} onChange={updates.setEnabled}>
        {words("automatic")}
      </Switch>
      <p className="desktop-settings-help">{words("automatic.help")}</p>
      {status.nightly ? (
        <div>
          <p className="paper-field-label">{words("channel")}</p>
          <p>{channelName("nightly")}</p>
          <p className="desktop-settings-help">{words("channel.nightlyHelp")}</p>
        </div>
      ) : (
        <RadioGroup
          label={words("channel")}
          orientation="horizontal"
          value={status.channel}
          isDisabled={status.managed}
          onChange={(value) => updates.setChannel(value as Channel)}
          description={words("nightlyNote")}
        >
          <Radio value="beta">{channelName("beta")}</Radio>
          <Radio value="stable" isDisabled={!status.stableAvailable}>
            {status.stableAvailable ? channelName("stable") : words("channel.stableNotYet")}
          </Radio>
        </RadioGroup>
      )}
      <div className="desktop-update-check">
        <span className="desktop-settings-help">{lastChecked(status.lastChecked)}</span>
        <Button isDisabled={busy || !status.canCheck} onPress={updates.checkNow}>
          {words("checkNow")}
        </Button>
      </div>
      <Sends updates={updates} />
    </div>
  );
}

/** What each check sends, the privacy page, and Reset anonymous ID. */
function Sends({ updates }: { updates: DesktopUpdates }): ReactNode {
  return (
    <div>
      <p className="paper-field-label">{words("sends")}</p>
      <p className="desktop-settings-help">
        {words("sendsHelp")}{" "}
        <button type="button" className="desktop-link" onClick={() => updates.openLink("privacy")}>
          {words("privacy")}
        </button>
      </p>
      <Button onPress={updates.resetId}>{words("resetId")}</Button>
    </div>
  );
}

/**
 * The status bar's "Update ready" (S06-update-ready), once an update is downloaded and verified,
 * with its details: the version, the channel, Later and Restart to update. The arrival is announced
 * once, politely.
 */
export function UpdateReadyItem({ updates }: { updates: DesktopUpdates }): ReactNode {
  const [isOpen, setOpen] = useState(false);
  const { status } = updates;
  const ready = status?.state.state === "ready" ? status.state : undefined;
  const title = ready === undefined ? "" : words("ready", { version: ready.version });
  return (
    <>
      {/* Always the same element, empty until the update is ready: screen readers announce the
          words when they appear, and since they then stay, they speak once. */}
      <span className="paper-visually-hidden" aria-live="polite">
        {title}
      </span>
      {status === undefined || ready === undefined ? null : (
        <span className="desktop-update-ready">
          <PopoverTrigger isOpen={isOpen} onOpenChange={setOpen}>
            <StatusBarItem icon="download">{words("readyItem")}</StatusBarItem>
            <Popover aria-label={title} placement="top">
              <div className="desktop-update-popover">
                <div className="desktop-update-popover-head">
                  <p className="desktop-update-title">{title}</p>
                  <IconButton label={words("closeDetails")} icon="close" onPress={() => setOpen(false)} />
                </div>
                <p className="desktop-settings-help">
                  {words("readyChannel", {
                    channel: channelName(status.channel),
                    current: status.current,
                  })}
                </p>
                <p className="desktop-settings-help">{words("readyHelp")}</p>
                <button type="button" className="desktop-link" onClick={() => updates.openLink("whats-new")}>
                  {words("whatsNew", { version: ready.version })}
                </button>
                <div className="desktop-update-actions">
                  <Button onPress={() => setOpen(false)}>{words("later")}</Button>
                  <Button variant="primary" onPress={updates.restart}>
                    {message("command.app.restart-to-update")}
                  </Button>
                </div>
              </div>
            </Popover>
          </PopoverTrigger>
        </span>
      )}
    </>
  );
}
