// Several PDFs the operating system opened together with the installed reader (Paper O07; E2m):
// the reader shows one document per tab, so it asks whether to merge them into one new document,
// opened unsaved, or to open the first. Loaded with later.ts only when such a launch comes, and
// shown in a root of its own, since it can come before the app has rendered.

import { Button, Dialog, Notice, type Platform, type PlatformFile, PlatformProvider } from "@laminapdf/ui";
import { laterMessage as word, laterCountMessage as words } from "@laminapdf/ui/editing";
import { mergeFiles } from "@laminapdf/viewer";
import { type ReactNode, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Engine } from "./session.ts";

/** A file held in memory, as the reader opens one: a merged document that has never been saved. */
function memoryFile(name: string, bytes: Uint8Array): PlatformFile {
  return {
    name,
    size: bytes.length,
    read: async (offset, length) => bytes.slice(offset, offset + length),
    handle: new File([bytes as BlobPart], name, { type: "application/pdf" }),
    writable: false,
  };
}

/** Merges `files`, in the order the system gave them, into one new document named after the first. */
async function mergeAll(files: readonly PlatformFile[], engine: Engine): Promise<PlatformFile> {
  const inputs = await Promise.all(
    files.map(async (file) => ({ bytes: await file.read(0, file.size), title: file.name.replace(/\.pdf$/i, "") })),
  );
  const { client } = await engine.view();
  const bytes = await mergeFiles(client, inputs);
  const first = files[0]?.name.replace(/\.pdf$/i, "") ?? "document";
  return memoryFile(word("documents.several.name", { name: first }), bytes);
}

/** Asks what to do with `files`, opened together from the system, and does it. */
export function askAboutSeveral(files: readonly PlatformFile[], engine: Engine, platform: Platform): void {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const done = () => {
    root.unmount();
    host.remove();
  };
  root.render(
    <PlatformProvider platform={platform}>
      <SeveralFilesDialog files={files} engine={engine} onDone={done} />
    </PlatformProvider>,
  );
}

/** The question: Merge into one document, or Open the first. */
function SeveralFilesDialog({
  files,
  engine,
  onDone,
}: {
  files: readonly PlatformFile[];
  engine: Engine;
  onDone: () => void;
}): ReactNode {
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const first = files[0];
  const open = async (file: PlatformFile | undefined) => {
    onDone();
    if (file !== undefined) await engine.reader.open(file).catch(reportError);
  };
  const merge = async () => {
    setBusy(true);
    try {
      await open(await mergeAll(files, engine));
    } catch {
      setBusy(false);
      setFailed(true);
    }
  };
  return (
    <Dialog
      isOpen
      onOpenChange={(isOpen) => {
        if (!isOpen && !busy) onDone();
      }}
      title={words(files.length, { one: "documents.several.title", many: "documents.several.title" })}
      description={word("documents.several.body")}
      closeLabel={word("organise.close")}
      actions={
        <>
          <Button isDisabled={busy} onPress={() => void open(first)}>
            {word("documents.several.first", { name: first?.name ?? "" })}
          </Button>
          <Button variant="primary" isDisabled={busy} autoFocus onPress={() => void merge()}>
            {word("documents.several.merge")}
          </Button>
        </>
      }
    >
      <ol className="paper-documents-parts">
        {files.map((file, i) => (
          <li key={i}>
            <span className="paper-documents-file-name">{file.name}</span>
          </li>
        ))}
      </ol>
      {failed ? (
        <Notice tone="danger" title={word("documents.failed.merge")}>
          {word("documents.several.failed")}
        </Notice>
      ) : null}
    </Dialog>
  );
}
