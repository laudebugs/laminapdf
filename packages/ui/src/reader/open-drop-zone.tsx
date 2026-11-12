// The home screen's way in (R01): a heading, the drop zone that explains the window takes a
// dropped or pasted PDF, and Choose a file. While files are dragged over the window, an overlay
// says a drop will open them. A file that isn't a PDF, by its name or its header, is refused with a
// message, and the reader stays on the home screen. The recent list and the privacy line come from the shell, under it.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useEffectEvent, useId, useLayoutEffect, useRef, useState } from "react";
import { Button } from "../components/button.js";
import { type KeyPlatform, ShortcutChip } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { usePlatform } from "../platform/context.js";
import type { PlatformFile } from "../platform/types.js";
import type { ReaderController } from "../state/reader-controller.js";

/** Props of `OpenDropZone`. */
export interface OpenDropZoneProps {
  /** The reader that opens what is chosen, dropped or pasted. */
  reader: ReaderController;
  /** How the paste shortcut is written. */
  keys: KeyPlatform;
  /** What follows the zone on the home screen: recent files, the privacy line. */
  children?: ReactNode;
}

const PASTE = { mac: "Mod+V", other: "Mod+V" };

/** The home screen's drop zone and Choose a file; the whole window takes drops and pastes while it shows. */
export function OpenDropZone({ reader, keys, children }: OpenDropZoneProps): ReactNode {
  const platform = usePlatform();
  const heading = useId();
  const [dragging, setDragging] = useState(false);
  const [problem, setProblem] = useState("");
  // Each choice, drop or paste supersedes the last; a slow check of an earlier one opens nothing.
  const latest = useRef(0);
  const accept = async (files: PlatformFile[]) => {
    const [file] = files;
    if (file === undefined) return;
    latest.current += 1;
    const turn = latest.current;
    const refused = await refusal(file);
    if (turn !== latest.current || reader.getState().phase !== "empty") return;
    setProblem(refused ?? "");
    if (refused === undefined) await reader.open(file);
  };
  // Only getting the files can fail here: the reader reports its own failures (an open never rejects).
  const take = (files: Promise<PlatformFile[]>) =>
    void files.then(accept, () => setProblem(message("reader.open.unreadable")));
  // The platform reads the transfer before anything awaits, while its items are still there.
  const onFiles = useEffectEvent((data: DataTransfer) => take(platform.filesFromDataTransfer(data)));
  // Before the home screen paints: once it says the window takes a drop or a paste, it does. A
  // listener added after the paint (useEffect) left a moment, long on a slow start, when a drop
  // went to the browser, which opened the file in place of the app.
  useLayoutEffect(() => listenForFiles(window, { onDragging: setDragging, onFiles }), []);
  return (
    <section className="paper-open" aria-labelledby={heading}>
      <h1 id={heading} className="paper-open-heading">
        {message("reader.open.heading")}
      </h1>
      <div className="paper-open-zone">
        <Icon name="file" size={22} />
        <p className="paper-open-drop">{message("reader.open.drop")}</p>
        <p className="paper-open-hint">
          {message("reader.open.paste")} <ShortcutChip keys={PASTE} platform={keys} />
        </p>
        {/* The picker's files take the same check as a drop's, so a file is never treated two ways. */}
        <Button variant="primary" onPress={() => take(platform.openFiles({ multiple: false }))}>
          {message("reader.open.choose")}
        </Button>
        {problem === "" ? null : (
          <p role="alert" className="paper-open-problem">
            <Icon name="warning" size={16} />
            {problem}
          </p>
        )}
      </div>
      {children}
      {dragging ? <DropOverlay>{message("reader.open.dropping")}</DropOverlay> : null}
    </section>
  );
}

/**
 * The overlay over the whole window while files are dragged over it, saying what a drop will do.
 * A shell whose OS reports drags itself (the desktop reader) shows it over any screen.
 */
export function DropOverlay({ children }: { children: ReactNode }): ReactNode {
  return (
    <div className="paper-open-overlay" aria-hidden="true">
      {children}
    </div>
  );
}

/**
 * Listens on `target` for files dragged over it, dropped on it or pasted into it. `onDragging`
 * follows whether files are over the window (enter and leave events come for every child, so
 * they are counted); `onFiles` gets each drop's or paste's transfer. Returns the unsubscribe.
 */
function listenForFiles(
  target: Window,
  handlers: { onDragging: (dragging: boolean) => void; onFiles: (data: DataTransfer) => void },
): () => void {
  let depth = 0;
  const files = (data: DataTransfer | null): data is DataTransfer => data?.types.includes("Files") === true;
  const enter = (event: DragEvent) => {
    if (!files(event.dataTransfer)) return;
    depth += 1;
    handlers.onDragging(true);
  };
  const over = (event: DragEvent) => {
    if (files(event.dataTransfer)) event.preventDefault();
  };
  const leave = (event: DragEvent) => {
    if (!files(event.dataTransfer)) return;
    depth = Math.max(0, depth - 1);
    if (depth === 0) handlers.onDragging(false);
  };
  const drop = (event: DragEvent) => {
    if (!files(event.dataTransfer)) return;
    event.preventDefault();
    depth = 0;
    handlers.onDragging(false);
    handlers.onFiles(event.dataTransfer);
  };
  const paste = (event: ClipboardEvent) => {
    if (!files(event.clipboardData)) return;
    event.preventDefault();
    handlers.onFiles(event.clipboardData);
  };
  const listeners = [
    ["dragenter", enter],
    ["dragover", over],
    ["dragleave", leave],
    ["drop", drop],
    ["paste", paste],
  ] as const;
  for (const [type, listener] of listeners) target.addEventListener(type, listener as EventListener);
  return () => {
    for (const [type, listener] of listeners) target.removeEventListener(type, listener as EventListener);
  };
}

/** Why `file` can't be opened from the home screen, or undefined when it can. */
async function refusal(file: PlatformFile): Promise<string | undefined> {
  try {
    return (await looksLikePdf(file)) ? undefined : message("reader.open.notPdf");
  } catch {
    return message("reader.open.unreadable");
  }
}

/**
 * Whether `file` looks like a PDF: named `.pdf`, or with "%PDF-" in its first 1,024 bytes (ISO
 * 32000-2 §7.5.2 allows leading bytes). A `.pdf` without a header still goes to the reader, which
 * repairs what it can (ADR 0003, `missing-header`).
 */
async function looksLikePdf(file: PlatformFile): Promise<boolean> {
  if (/\.pdf$/i.test(file.name)) return true;
  const head = await file.read(0, Math.min(1024, file.size));
  return String.fromCharCode(...head).includes("%PDF-");
}
