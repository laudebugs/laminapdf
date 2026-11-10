// The shell's report of itself, through the one app command the webview may call (`app_info`).
import { invoke } from "@tauri-apps/api/core";

/** What `app_info` returns (src-tauri/src/commands.rs). */
export interface AppInfo {
  readonly version: string;
  readonly os: string;
  readonly arch: string;
  readonly webviewVersion: string | null;
}

/** Asks the shell for its version, OS, CPU and webview version. */
export function loadAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}
