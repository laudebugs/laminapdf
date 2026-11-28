// The Attachments tab's panel (O14), loaded with later.ts once a document is open (#380): the files
// the engine read (and reads again after every edit), the selected one's details, and saving one
// out as a download (or a save dialog where the browser has one), after O14's warning for a
// program. In Organise, Add file… and a drop attach files, the details change a file's
// description and relationship, and Delete attachment takes it out; each is one step of the edit
// session, with a toast offering Undo after a deletion. Attachments never leave the device.

import type { Collection, DocumentAttachment, Op } from "@laminapdf/sdk";
import {
  formatBytes,
  type ModeRegistry,
  message,
  type Platform,
  pageLabel,
  RAIL_DRAWER_QUERY,
  type ShellLayout,
  type ToastQueue,
  useMediaQuery,
  useStoreState,
} from "@laminapdf/ui";
import {
  type AttachmentChange,
  type AttachmentData,
  AttachmentsPanel,
  attachmentName,
  type ChecksumState,
  isProgram,
  mimeOf,
  ProgramWarningDialog,
  pdfDateInstant,
  pdfDateOf,
} from "@laminapdf/ui/dialogs";
import { laterMessage } from "@laminapdf/ui/editing";
import { type ReactNode, useEffect, useState } from "react";
import type { Engine } from "./session.ts";

/** Props of `AttachmentsTab`. */
export interface AttachmentsTabProps {
  readonly engine: Engine;
  readonly layout: ShellLayout;
  readonly platform: Platform;
  readonly toasts: ToastQueue;
  readonly modes: ModeRegistry;
  /** Organise mode is on. */
  readonly organising: boolean;
  /** Save an optimised copy…, which drops what earlier versions still hold. */
  readonly saveOptimised?: (() => void) | undefined;
}

/** Whether `file` is larger than the document can read back out (its `maxStreamDecoded` limit). */
function tooLarge(file: { readonly size?: number }, limit: number | undefined): boolean {
  return limit !== undefined && (file.size ?? 0) > limit;
}

/** `bytes` as a Blob of their own, to save out exactly as attached. */
function blobOf(bytes: Uint8Array, mime: string | undefined): Blob {
  return new Blob([new Uint8Array(bytes)], mime === undefined ? {} : { type: mime });
}

/** The selected file's checksum verdict, read with its bytes when it is selected. */
function useChecksum(
  engine: Engine,
  file: DocumentAttachment | undefined,
  limit: number | undefined,
): ChecksumState | undefined {
  const [checksum, setChecksum] = useState<{ id: string; state: ChecksumState } | undefined>(undefined);
  const id = file?.id;
  const readable = file?.embedded === true && file.hasChecksum && !tooLarge(file, limit);
  useEffect(() => {
    if (id === undefined || !readable) return;
    let live = true;
    setChecksum({ id, state: "checking" });
    engine.attachmentBytes(id).then(
      (read) => {
        if (live) setChecksum({ id, state: read.checksum });
      },
      () => {
        if (live) setChecksum(undefined);
      },
    );
    return () => {
      live = false;
    };
  }, [engine, id, readable]);
  if (file === undefined) return undefined;
  if (!file.hasChecksum) return "absent";
  return checksum?.id === file.id ? checksum.state : undefined;
}

/** The kinds of portfolio column a row shows under its name: those the row doesn't show already. */
const COLUMN_KINDS = new Set(["text", "date", "number", "created"]);

/**
 * A portfolio's visible columns for each file, as "Owner: Finance" lines (O14), in the schema's
 * order; none for a document that isn't a portfolio.
 */
function columnLines(collection: Collection | undefined): ((id: string) => readonly string[]) | undefined {
  if (collection === undefined) return undefined;
  const fields = collection.schema.filter((f) => f.visible && COLUMN_KINDS.has(f.kind));
  return (id) => {
    const values = collection.files.find((f) => f.attachment === id)?.values ?? {};
    return fields.flatMap((f) => {
      const value = values[f.key];
      if (value === undefined) return [];
      const at = f.kind === "date" && typeof value === "string" ? pdfDateInstant(value) : undefined;
      const shown = at === undefined ? String(value) : at.toLocaleDateString(undefined, { dateStyle: "medium" });
      return [`${f.name ?? f.key}: ${shown}`];
    });
  };
}

/** Saves attachment `file`'s bytes out, exactly as attached: a save dialog where there is one, else a download. */
async function saveOut(engine: Engine, platform: Platform, toasts: ToastQueue, file: AttachmentData): Promise<void> {
  const read = await engine.attachmentBytes(file.id);
  const saveExport = platform.saveExport;
  if (saveExport === undefined) return;
  const result = await saveExport.call(platform, blobOf(read.bytes, file.mime), attachmentName(file));
  if (result.saved) toasts.add({ message: message("download.done", { name: result.name }) });
}

/**
 * Attaches `files`, one step of the edit session each, named by the file: associated with the
 * document, with the file's own date and type, which PDF/A-3 asks of every attached file.
 */
async function attach(engine: Engine, toasts: ToastQueue, files: readonly File[]): Promise<void> {
  const limit = engine.getState().attachmentLimit;
  for (const file of files) {
    // A file the document could never read back out isn't attached (O14's memory limit).
    if (tooLarge(file, limit)) {
      toasts.add({
        message: laterMessage("attachments.tooLarge.attach", { name: file.name, limit: formatBytes(limit ?? 0) }),
      });
      continue;
    }
    const blob = await engine.addBlob(new Uint8Array(await file.arrayBuffer()));
    const op = {
      op: "attachFile",
      args: {
        name: file.name,
        file: blob,
        mime: mimeOf(file.name, file.type),
        modified: pdfDateOf(new Date(file.lastModified)),
        attachTo: "document",
        relationship: "unspecified",
      },
    } satisfies Op;
    await engine.applyEdits([op], laterMessage("attachments.label.attach", { name: file.name }));
  }
}

/** The files the picker gives, read whole: an attached file goes into the document. */
async function pick(platform: Platform): Promise<File[]> {
  const picked = await platform.openFiles({ multiple: true, accept: "any" });
  return Promise.all(
    picked.map(async (p) =>
      p.handle instanceof File ? p.handle : new File([new Uint8Array(await p.read(0, p.size))], p.name),
    ),
  );
}

/** Changes `file` as an Organise field asked, as one step named by the file. */
function change(engine: Engine, file: AttachmentData, patch: AttachmentChange): void {
  const op = { op: "updateAttachment", args: { attachment: file.id, ...patch } } satisfies Op;
  void engine
    .applyEdits([op], laterMessage("attachments.label.update", { name: attachmentName(file) }))
    .catch(reportError);
}

/** Deletes `file` as one step named by it, then says so with Undo (O14). */
async function remove(engine: Engine, toasts: ToastQueue, file: AttachmentData): Promise<void> {
  const op = { op: "deleteAttachment", args: { attachment: file.id } } satisfies Op;
  await engine.applyEdits([op], laterMessage("attachments.label.delete", { name: attachmentName(file) }));
  toasts.add({
    message: laterMessage("attachments.deleted"),
    action: { label: message("command.edit.undo"), onPress: () => void engine.getState().viewer?.edits.undo() },
  });
}

/** The files the open document carries, and what can be done with them. */
export function AttachmentsTab(props: AttachmentsTabProps): ReactNode {
  const { engine, layout, platform, toasts, modes, organising } = props;
  const covers = useMediaQuery(RAIL_DRAWER_QUERY);
  const files = useStoreState(engine, (s) => s.attachments);
  const limit = useStoreState(engine, (s) => s.attachmentLimit);
  const collection = useStoreState(engine, (s) => s.collection);
  const pages = useStoreState(engine, (s) => s.pages);
  const viewer = useStoreState(engine, (s) => s.viewer);
  const [selected, setSelected] = useState<string | undefined>(undefined);
  const [program, setProgram] = useState<AttachmentData | undefined>(undefined);
  const [deleted, setDeleted] = useState(false);
  const shown = files.find((f) => f.id === selected) ?? files[0];
  const checksum = useChecksum(engine, shown, limit);
  const save = (file: AttachmentData) => {
    // A program is never opened; saving one waits for O14's warning.
    if (isProgram(file)) setProgram(file);
    else void saveOut(engine, platform, toasts, file).catch(reportError);
  };
  const editing = organising
    ? {
        onAdd: () =>
          void pick(platform)
            .then((picked) => attach(engine, toasts, picked))
            .catch(reportError),
        onDrop: (dropped: readonly File[]) => void attach(engine, toasts, dropped).catch(reportError),
        onChange: (file: AttachmentData, patch: AttachmentChange) => change(engine, file, patch),
        onDelete: (file: AttachmentData) =>
          void remove(engine, toasts, file)
            .then(() => setDeleted(true))
            .catch(reportError),
      }
    : { onEditInOrganise: () => modes.switchTo("organise") };
  return (
    <>
      <AttachmentsPanel
        files={files}
        organise={organising}
        selected={shown?.id}
        onSelect={setSelected}
        pageName={(index) => pageLabel(pages, index)}
        checksum={checksum}
        tooLarge={shown !== undefined && tooLarge(shown, limit) ? formatBytes(limit ?? 0) : undefined}
        saveLabel={laterMessage("attachments.download")}
        columns={columnLines(collection)}
        onSave={save}
        {...editing}
        onGoToPage={(index) => {
          viewer?.goToPage(index);
          if (covers) layout.setRailOpen(false);
        }}
        history={deleted && props.saveOptimised !== undefined ? { onSaveOptimised: props.saveOptimised } : undefined}
      />
      <ProgramWarningDialog
        name={program === undefined ? undefined : attachmentName(program)}
        onAnswer={(yes) => {
          const file = program;
          setProgram(undefined);
          if (yes && file !== undefined) void saveOut(engine, platform, toasts, file).catch(reportError);
        }}
      />
    </>
  );
}
