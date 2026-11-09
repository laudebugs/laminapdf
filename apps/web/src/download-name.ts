// The name Download a copy suggests (R09): the open file's name, made safe to save on any system.
// A remote file's name comes from the last part of its address, which anyone can write, so it may
// hold a path, characters Windows refuses, control characters or a name Windows reserves.

/** Names Windows reserves for devices, whatever the extension. */
const RESERVED = /^(con|prn|aux|nul|com\d|lpt\d)$/i;

/** The longest name kept, in characters, before `.pdf`. */
const MAX_STEM = 200;

/** `name` as a safe file name ending in `.pdf`: `document.pdf` when nothing is left of it. */
export function downloadName(name: string): string {
  const base = name.split(/[\\/]/).at(-1) ?? "";
  const cleaned = base
    // Control characters and the ones Windows refuses in a name.
    // biome-ignore lint/suspicious/noControlCharactersInRegex: control characters are what this removes.
    .replace(/[\u0000-\u001f\u007f<>:"|?*]/g, "_")
    .replace(/\s+/g, " ")
    .trim()
    // Windows drops trailing dots and spaces; a leading dot hides the file elsewhere.
    .replace(/^[.\s]+|[.\s]+$/g, "");
  const stem = cleaned
    .replace(/\.pdf$/i, "")
    .slice(0, MAX_STEM)
    .trim();
  if (stem === "" || /^_+$/.test(stem)) return "document.pdf";
  return `${RESERVED.test(stem.split(".")[0] ?? "") ? `_${stem}` : stem}.pdf`;
}
