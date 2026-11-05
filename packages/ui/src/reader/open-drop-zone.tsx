// The home screen's way in (R01): a heading, the drop zone that explains the window takes a
// dropped or pasted PDF, and Choose a file. While files are dragged over the window, an overlay
// says a drop will open them. A file that isn't a PDF is refused with a message, and the reader
// stays on the home screen. The recent list and the privacy line come from the shell, under it.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useEffect, useId, useState } from "react";
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
  useEffect(() => {
    const accept = async (files: PlatformFile[]) => {
      const [file] = files;
      if (file === undefined) return;
      if (!(await isPdf(file))) return setProblem(message("reader.open.notPdf"));
      setProblem("");
      await reader.open(file);
    };
    return listenForFiles(window, {
      onDragging: setDragging,
      // The platform reads the transfer before anything awaits, while its items are still there.
      onFiles: (data) => void platform.filesFromDataTransfer(data).then(accept),
    });
  }, [platform, reader]);
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
        <Button variant="primary" onPress={() => void reader.openFromPicker()}>
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
      {dragging ? (
        <div className="paper-open-overlay" aria-hidden="true">
          {message("reader.open.dropping")}
        </div>
      ) : null}
    </section>
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

/** Whether `file` is a PDF: its header, "%PDF-", within its first 1024 bytes (ISO 32000-2 §7.5.2 allows leading bytes). */
async function isPdf(file: PlatformFile): Promise<boolean> {
  const head = await file.read(0, Math.min(1024, file.size));
  const text = String.fromCharCode(...head);
  return text.includes("%PDF-");
}
