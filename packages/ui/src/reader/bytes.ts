// Sizes in the units a reader expects, for every panel that shows one.

import { message } from "../messages.js";

const KB = 1024;
const MB = 1024 * KB;

/** `bytes` in megabytes with one decimal, without the unit: "1.8". */
export function megabytes(bytes: number): string {
  return (bytes / MB).toFixed(1);
}

/** `bytes` with its unit: "512 B", "3.2 KB", "1.8 MB". */
export function formatBytes(bytes: number): string {
  if (bytes < KB) return message("network.bytes", { value: String(bytes) });
  if (bytes < MB) return message("network.kb", { value: (bytes / KB).toFixed(1) });
  return message("network.mb", { value: megabytes(bytes) });
}
