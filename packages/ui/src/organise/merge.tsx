// Merge documents (Paper O07): the ordered list of files (this document first and fixed), each
// moved with buttons as well as by order, "Add files…", whether the result is this document or a
// new one, what happens to bookmarks and to form fields of the same name, and a notice when the
// files claim different PDF/A or PDF/UA conformance, which the result can't keep for them all.

import { type ReactNode, useState } from "react";
import { Button, IconButton } from "../components/button.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { Notice } from "../components/notice.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import { FileCard, type SourceFile } from "./from-file.js";
import { useOnOpen } from "./on-open.js";

/** One file of the merge besides this document: as the shell has read it, and what its metadata claims. */
export interface MergeFile {
  /** Its key in the list, stable while it is there. */
  readonly key: string;
  readonly source: SourceFile;
  /** `PDF/A-2B`, `PDF/UA-1`: the conformance its metadata claims, if any. */
  readonly claims?: readonly string[];
}

/** Where the merged pages go: added to this document (one step, undoable) or into a new document. */
export type MergeResult = "add" | "new";

/** What Merge does. */
export interface MergeChoice {
  readonly result: MergeResult;
  /** One bookmark per added file holding its bookmarks, its bookmarks as they are, or none of them. */
  readonly bookmarks: "perFile" | "keep" | "none";
  /** A field whose name is taken: renamed, or linked to the field of that name. */
  readonly fields: "rename" | "link";
}

/** Props of `MergeDialog`. */
export interface MergeDialogProps {
  /** Whether the dialog shows. */
  readonly isOpen: boolean;
  /** Called when it closes, merged or not. */
  readonly onClose: () => void;
  /** This document: its name and what it claims; undefined when the merge starts from files alone. */
  readonly current: { readonly name: string; readonly claims: readonly string[] } | undefined;
  /** The other files, in order. */
  readonly files: readonly MergeFile[];
  /** Asks for files to add at the end. */
  readonly onAdd: () => void;
  /** Moves file `key` up (-1) or down (1). */
  readonly onMove: (key: string, delta: -1 | 1) => void;
  /** Takes file `key` out of the list. */
  readonly onRemove: (key: string) => void;
  /** Tries `password` on file `key`. */
  readonly onPassword: (key: string, password: string) => void;
  /** Whether the merge can make a new document (the default): false adds the files to this one without asking. */
  readonly canMakeNew?: boolean;
  /** Merges; the dialog closes. */
  readonly onMerge: (choice: MergeChoice) => void;
}

/** The claims that not every file of the merge makes, for the conformance notice. */
export function unsharedClaims(lists: readonly (readonly string[])[]): string[] {
  const all = new Set(lists.flat());
  return [...all].filter((claim) => lists.some((list) => !list.includes(claim))).sort();
}

/** Merge documents. */
export function MergeDialog(props: MergeDialogProps): ReactNode {
  const { current, files } = props;
  const [result, setResult] = useState<MergeResult>("add");
  const [bookmarks, setBookmarks] = useState<MergeChoice["bookmarks"]>("perFile");
  const [fields, setFields] = useState<MergeChoice["fields"]>("rename");
  const [passwords, setPasswords] = useState<ReadonlyMap<string, string>>(new Map());
  useOnOpen(props.isOpen, () => {
    setResult(current === undefined ? "new" : "add");
    setPasswords(new Map());
  });
  const count = files.length + (current === undefined ? 0 : 1);
  const ready = files.every((file) => file.source.status === "ready");
  const canMerge = ready && count >= 2;
  const claims = unsharedClaims([
    ...(current === undefined ? [] : [current.claims]),
    ...files.map((file) => file.claims ?? []),
  ]);
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      title={laterMessage("documents.merge.title")}
      closeLabel={laterMessage("organise.close")}
      size="form"
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("organise.move.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={!canMerge}
            onPress={() => {
              props.onMerge({ result, bookmarks, fields });
              props.onClose();
            }}
          >
            {laterMessage("documents.merge.go", { count })}
          </Button>
        </>
      }
    >
      <MergeOrder current={current} files={files} passwords={passwords} onPasswords={setPasswords} actions={props} />
      <Button size={30} icon="file" onPress={props.onAdd}>
        {laterMessage("documents.merge.add")}
      </Button>
      {current === undefined || props.canMakeNew === false ? null : (
        <RadioGroup
          label={laterMessage("documents.merge.result")}
          value={result}
          onChange={(value) => setResult(value as MergeResult)}
        >
          <Radio value="add">{laterMessage("documents.merge.result.add", { name: current.name })}</Radio>
          <Radio value="new">{laterMessage("documents.merge.result.new")}</Radio>
        </RadioGroup>
      )}
      <RadioGroup
        label={laterMessage("documents.merge.bookmarks")}
        value={bookmarks}
        onChange={(value) => setBookmarks(value as MergeChoice["bookmarks"])}
      >
        <Radio value="perFile">{laterMessage("documents.merge.bookmarks.perFile")}</Radio>
        <Radio value="keep">{laterMessage("documents.merge.bookmarks.keep")}</Radio>
        <Radio value="none">{laterMessage("documents.merge.bookmarks.none")}</Radio>
      </RadioGroup>
      <RadioGroup
        label={laterMessage("documents.merge.fields")}
        value={fields}
        onChange={(value) => setFields(value as MergeChoice["fields"])}
      >
        <Radio value="rename">{laterMessage("documents.merge.fields.rename")}</Radio>
        <Radio value="link">{laterMessage("documents.merge.fields.link")}</Radio>
      </RadioGroup>
      {claims.length === 0 ? null : (
        <Notice tone="warning" title={laterMessage("documents.merge.claims.title")}>
          {laterCountMessage(
            claims.length,
            { one: "documents.merge.claims.one", many: "documents.merge.claims" },
            { claims: claims.join(", ") },
          )}
        </Notice>
      )}
    </Dialog>
  );
}

/** The files in order: this document first and fixed, each other with its card and its move buttons. */
function MergeOrder({
  current,
  files,
  passwords,
  onPasswords,
  actions: props,
}: {
  current: MergeDialogProps["current"];
  files: readonly MergeFile[];
  passwords: ReadonlyMap<string, string>;
  onPasswords: (passwords: ReadonlyMap<string, string>) => void;
  actions: Pick<MergeDialogProps, "onAdd" | "onMove" | "onRemove" | "onPassword">;
}): ReactNode {
  return (
    <>
      <h3 className="paper-documents-heading" id="paper-merge-files">
        {laterMessage("documents.merge.files")}
      </h3>
      <ol className="paper-documents-order" aria-labelledby="paper-merge-files">
        {current === undefined ? null : (
          <li className="paper-documents-order-item" data-fixed>
            <span className="paper-documents-file-name">{current.name}</span>
            <span className="paper-documents-file-facts">{laterMessage("documents.merge.this")}</span>
          </li>
        )}
        {files.map((file, i) => (
          <li key={file.key} className="paper-documents-order-item">
            <FileCard
              source={file.source}
              password={passwords.get(file.key) ?? ""}
              onPassword={(value) => onPasswords(new Map(passwords).set(file.key, value))}
              onTry={() => props.onPassword(file.key, passwords.get(file.key) ?? "")}
              onChoose={props.onAdd}
            />
            {/* biome-ignore lint/a11y/useSemanticElements: a group of buttons for one list item; <fieldset> holds form controls. */}
            <span className="paper-documents-order-moves" role="group" aria-label={file.source.name}>
              <IconButton
                icon="chevron-up"
                size={26}
                label={laterMessage("documents.merge.up", { name: file.source.name })}
                isDisabled={i === 0}
                onPress={() => props.onMove(file.key, -1)}
              />
              <IconButton
                icon="chevron-down"
                size={26}
                label={laterMessage("documents.merge.down", { name: file.source.name })}
                isDisabled={i === files.length - 1}
                onPress={() => props.onMove(file.key, 1)}
              />
              <IconButton
                icon="close"
                size={26}
                label={laterMessage("documents.merge.remove", { name: file.source.name })}
                onPress={() => props.onRemove(file.key)}
              />
            </span>
          </li>
        ))}
      </ol>
    </>
  );
}
