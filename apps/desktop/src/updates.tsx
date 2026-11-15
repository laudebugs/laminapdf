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

/** A channel an install follows. */
type Channel = "stable" | "beta" | "nightly";

/** Why automatic updates are off: the user, policy, an MSI install or a Linux package manager. */
type Off = "user" | "policy" | "msi" | "package-manager";

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
      resetId: () =>
        void call("update_reset_install_id")?.then(() =>
          toasts.add({ message: message("settings.updates.resetDone") }),
        ),
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
function channelName(channel: Channel): string {
  return message(`settings.updates.channel.${channel}`);
}

/** When the last check ran, in words; "Not checked yet" before the first. */
function lastChecked(seconds: number | null): string {
  if (seconds === null) return message("settings.updates.neverChecked");
  const when = new Date(seconds * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  return message("settings.updates.lastChecked", { when });
}

/** Settings › Updates in one line, for a narrow window's list of sections. */
export function updatesSummary(status: UpdateStatus | undefined): string {
  if (status === undefined) return "";
  if (status.off === "package-manager") return message("settings.updates.summary.packageManager");
  if (status.managed) return message("settings.updates.summary.managed");
  if (status.off !== null) return message("settings.updates.summary.off");
  return message("settings.updates.summary.on", { channel: channelName(status.channel) });
}

/** The card at the top of Settings › Updates: where the update is, and what the user can do now. */
function StatusCard({ status, updates }: { status: UpdateStatus; updates: DesktopUpdates }): ReactNode {
  const { state } = status;
  switch (state.state) {
    case "ready":
      return (
        <div className="desktop-update-card">
          <p className="desktop-update-title">{message("settings.updates.ready", { version: state.version })}</p>
          <p className="desktop-settings-help">{message("settings.updates.readyHelp")}</p>
          <Button variant="primary" onPress={updates.restart}>
            {message("command.app.restart-to-update")}
          </Button>
        </div>
      );
    case "downloading":
      return (
        <div className="desktop-update-card">
          <p className="desktop-update-title">{message("settings.updates.downloading", { version: state.version })}</p>
          <ProgressBar
            label={message("settings.updates.downloading", { version: state.version })}
            {...(state.percent === null ? {} : { value: state.percent })}
          />
          <p className="desktop-settings-help">
            {message("settings.updates.downloadingHelp", { current: status.current })}
          </p>
        </div>
      );
    case "checking":
      return (
        <div className="desktop-update-card">
          <ProgressBar label={message("settings.updates.checking")} />
        </div>
      );
    default:
      return (
        <div className="desktop-update-card">
          {state.state === "failed" ? <Notice tone="danger" title={message("settings.updates.failed")} /> : null}
          {status.off === null || status.off === "package-manager" ? (
            <p className="desktop-update-title">
              {/* "Up to date" only once a check has answered. */}
              {message(status.lastChecked === null ? "settings.updates.current" : "settings.updates.upToDate", {
                version: status.current,
              })}
            </p>
          ) : (
            <>
              <p className="desktop-update-title">{message("settings.updates.off")}</p>
              {status.off === "user" ? (
                <p className="desktop-settings-help">
                  {message("settings.updates.offHelp", { current: status.current })}
                </p>
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
  if (status.off === "package-manager") {
    return (
      <div className="paper-settings-fields">
        <p className="desktop-update-title">{message("settings.updates.packageManager")}</p>
        <p className="desktop-settings-help">{message("settings.updates.packageManagerHelp")}</p>
        <Sends updates={updates} />
      </div>
    );
  }
  return (
    <div className="paper-settings-fields">
      {status.managed ? (
        <div>
          <Badge tone="neutral">{message("settings.updates.managed")}</Badge>
          <p className="desktop-settings-help">
            {status.off === "policy" || status.off === "msi"
              ? message("settings.updates.managedOff")
              : message("settings.updates.managedChannel", { channel: channelName(status.channel) })}
          </p>
        </div>
      ) : null}
      <StatusCard status={status} updates={updates} />
      <Switch isSelected={status.enabled && !status.managed} isDisabled={status.managed} onChange={updates.setEnabled}>
        {message("settings.updates.automatic")}
      </Switch>
      <p className="desktop-settings-help">{message("settings.updates.automatic.help")}</p>
      {status.nightly ? (
        <div>
          <p className="paper-field-label">{message("settings.updates.channel")}</p>
          <p>{channelName("nightly")}</p>
          <p className="desktop-settings-help">{message("settings.updates.channel.nightlyHelp")}</p>
        </div>
      ) : (
        <RadioGroup
          label={message("settings.updates.channel")}
          orientation="horizontal"
          value={status.channel}
          isDisabled={status.managed}
          onChange={(value) => updates.setChannel(value as Channel)}
          description={message("settings.updates.nightlyNote")}
        >
          <Radio value="beta">{channelName("beta")}</Radio>
          <Radio value="stable" isDisabled={!status.stableAvailable}>
            {status.stableAvailable ? channelName("stable") : message("settings.updates.channel.stableNotYet")}
          </Radio>
        </RadioGroup>
      )}
      <div className="desktop-update-check">
        <span className="desktop-settings-help">{lastChecked(status.lastChecked)}</span>
        <Button isDisabled={busy || !status.canCheck} onPress={updates.checkNow}>
          {message("settings.updates.checkNow")}
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
      <p className="paper-field-label">{message("settings.updates.sends")}</p>
      <p className="desktop-settings-help">
        {message("settings.updates.sendsHelp")}{" "}
        <button type="button" className="desktop-link" onClick={() => updates.openLink("privacy")}>
          {message("settings.updates.privacy")}
        </button>
      </p>
      <Button onPress={updates.resetId}>{message("settings.updates.resetId")}</Button>
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
  const title = ready === undefined ? "" : message("settings.updates.ready", { version: ready.version });
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
            <StatusBarItem icon="download">{message("settings.updates.readyItem")}</StatusBarItem>
            <Popover aria-label={title} placement="top">
              <div className="desktop-update-popover">
                <div className="desktop-update-popover-head">
                  <p className="desktop-update-title">{title}</p>
                  <IconButton
                    label={message("settings.updates.closeDetails")}
                    icon="close"
                    onPress={() => setOpen(false)}
                  />
                </div>
                <p className="desktop-settings-help">
                  {message("settings.updates.readyChannel", {
                    channel: channelName(status.channel),
                    current: status.current,
                  })}
                </p>
                <p className="desktop-settings-help">{message("settings.updates.readyHelp")}</p>
                <button type="button" className="desktop-link" onClick={() => updates.openLink("whats-new")}>
                  {message("settings.updates.whatsNew", { version: ready.version })}
                </button>
                <div className="desktop-update-actions">
                  <Button onPress={() => setOpen(false)}>{message("settings.updates.later")}</Button>
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
