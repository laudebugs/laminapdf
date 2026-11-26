// Import comments (Paper C06), shown once a file is chosen: its name and format, what the import
// would do in plain lines (new comments and their pages, those already here, what is skipped and
// why), how to treat comments already here, a warning for a file made for another PDF, and the
// error for one that isn't valid. The import is one undo step.

import { type ReactNode, useState } from "react";
import { Button } from "../components/button.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { Notice } from "../components/notice.js";
import { type LaterMessageId, laterCountMessage, laterMessage } from "../later-messages.js";
import { useOnOpen } from "../organise/on-open.js";
import { Dialog } from "../overlays/dialog.js";
import {
  duplicates,
  IMPORT_MODES,
  type ImportMode,
  type ImportRead,
  type ImportReport,
  importCount,
  importFailure,
  mismatch,
  pageList,
  skipped,
} from "./interchange.js";
import { kindName } from "./kind-name.js";
import { useLoaded } from "./use-loaded.js";

/** The file chosen to import. */
export interface ImportFile {
  /** Changes with each file chosen, so a dialog open on another file reads again. */
  readonly key: string;
  readonly name: string;
  /** `XFDF` or `FDF`, from its name. */
  readonly format: string;
}

/** Props of `ImportCommentsDialog`. */
export interface ImportCommentsDialogProps {
  /** Whether the dialog shows. */
  readonly isOpen: boolean;
  /** Called when the dialog closes. */
  readonly onClose: () => void;
  /** The file chosen; undefined while the picker is up. */
  readonly file: ImportFile | undefined;
  /** What importing the file as `mode` would do. Rejects with the library's error for a file it can't read. */
  readonly load: (mode: ImportMode) => Promise<ImportReport>;
  /** Whether the import is running. */
  readonly busy: boolean;
  /** Asks for another file. */
  readonly onChoose: () => void;
  /** Imports as `mode` says. */
  readonly onImport: (mode: ImportMode) => void;
}

/** What each reason an import skips a comment says. */
const SKIP_REASONS: Readonly<Record<string, LaterMessageId>> = {
  unsupported: "import.skipped.unsupported",
  "form-field": "import.skipped.form-field",
  "missing-page": "import.skipped.missing-page",
  "missing-parent": "import.skipped.missing-parent",
  incomplete: "import.skipped.incomplete",
};

/** The lines saying what will be added, what is here already and what is left out. */
function Lines({ summary }: { summary: ImportReport }): ReactNode {
  const { added, matched, changed } = summary;
  const where =
    summary.pages.length === 1
      ? laterMessage("import.pages.one", { pages: pageList(summary.pages, laterMessage("import.and")) })
      : laterMessage("import.pages", { pages: pageList(summary.pages, laterMessage("import.and")) });
  const matchedLine =
    changed > 0
      ? laterCountMessage(matched, { one: "import.matched.changed.one", many: "import.matched.changed" }, { changed })
      : laterCountMessage(matched, { one: "import.matched.one", many: "import.matched" });
  return (
    <ul className="paper-interchange-lines">
      <li>
        {added === 0
          ? laterMessage("import.nothing")
          : laterCountMessage(added, { one: "import.added.one", many: "import.added" }, { pages: where })}
      </li>
      {matched === 0 ? null : <li>{matchedLine}</li>}
    </ul>
  );
}

/** What can't be imported, one line each, then how many comments are unaffected. */
function Skipped({ summary }: { summary: ImportReport }): ReactNode {
  const lines = skipped(summary);
  if (lines.length === 0) return null;
  return (
    <Notice
      tone="warning"
      title={laterCountMessage(lines.length, { one: "import.skipped.title.one", many: "import.skipped.title" })}
    >
      {lines.map((w, i) => {
        const reason = laterMessage(SKIP_REASONS[w.reason ?? ""] ?? "import.skipped.unknown");
        const page = w.page === undefined ? "" : laterMessage("import.skipped.page", { page: w.page + 1 });
        return (
          // The warnings have no ID of their own: their order is their identity.
          // biome-ignore lint/suspicious/noArrayIndexKey: a fixed list of strings
          <span key={i} className="paper-interchange-skip">
            {laterMessage("import.skipped.line", { kind: kindName(w.kind ?? ""), page, reason })}
          </span>
        );
      })}
    </Notice>
  );
}

/** The notices about the file: made for another PDF, and comments that share an ID. */
function FileNotices({ summary }: { summary: ImportReport }): ReactNode {
  const other = mismatch(summary);
  const twice = duplicates(summary);
  return (
    <>
      {other === undefined ? null : (
        <Notice tone="warning" title={laterMessage("import.mismatch.title")}>
          {other === "" ? laterMessage("import.mismatch.unnamed") : laterMessage("import.mismatch", { file: other })}
        </Notice>
      )}
      {twice === 0 ? null : (
        <Notice
          tone="neutral"
          title={laterCountMessage(twice, { one: "import.duplicates.one", many: "import.duplicates" })}
        />
      )}
    </>
  );
}

/** The error for a file that didn't read, with the line it stopped on. */
function Failure({ read, format }: { read: Extract<ImportRead, { state: "failed" }>; format: string }): ReactNode {
  const shown = read.format ?? format;
  const words =
    read.line === undefined
      ? laterMessage(read.format === undefined ? "import.failed.other" : "import.failed.format", { format: shown })
      : laterMessage("import.failed", { format: shown, line: read.line });
  return (
    <Notice tone="danger" title={laterMessage("import.failed.title")}>
      {words}
    </Notice>
  );
}

/** How comments already here are treated. */
function Modes(props: { value: ImportMode; onChange: (mode: ImportMode) => void; disabled: boolean }): ReactNode {
  return (
    <RadioGroup
      label={laterMessage("import.mode")}
      value={props.value}
      isDisabled={props.disabled}
      onChange={(value) => props.onChange(value as ImportMode)}
    >
      {IMPORT_MODES.map((mode) => (
        <Radio key={mode} value={mode}>
          <span>{laterMessage(`import.mode.${mode}`)}</span>
          {mode === "merge" ? (
            <span className="paper-interchange-help">{laterMessage("import.mode.merge.help")}</span>
          ) : null}
        </Radio>
      ))}
    </RadioGroup>
  );
}

/** The import button's words: the count, or "Import anyway" for a file made for another PDF. */
function importLabel(summary: ImportReport | undefined): string {
  if (summary === undefined) return laterMessage("import.go.none");
  if (mismatch(summary) !== undefined) return laterMessage("import.go.anyway");
  const count = importCount(summary);
  return count === 0
    ? laterMessage("import.go.none")
    : laterCountMessage(count, { one: "import.go.one", many: "import.go" });
}

/** Import comments. */
export function ImportCommentsDialog(props: ImportCommentsDialogProps): ReactNode {
  const [mode, setMode] = useState<ImportMode>("merge");
  useOnOpen(props.isOpen, () => setMode("merge"));
  const { file } = props;
  const loaded = useLoaded(props.isOpen && file !== undefined, `${file?.key}:${mode}`, () => props.load(mode));
  const read: ImportRead =
    loaded.state === "ready"
      ? { state: "ready", summary: loaded.value }
      : loaded.state === "failed"
        ? importFailure(loaded.error)
        : { state: "reading" };
  const summary = read.state === "ready" ? read.summary : undefined;
  const nothing = summary !== undefined && importCount(summary) === 0 && summary.matched === 0;
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open && !props.busy) props.onClose();
      }}
      title={laterMessage("import.title")}
      closeLabel={laterMessage("interchange.close")}
      size="form"
      actions={
        <>
          <span className="paper-interchange-hint">{laterMessage("import.undo")}</span>
          <Button onPress={props.onClose} isDisabled={props.busy}>
            {laterMessage("interchange.cancel")}
          </Button>
          <Button
            variant="primary"
            isDisabled={summary === undefined || nothing || props.busy}
            onPress={() => props.onImport(mode)}
          >
            {props.busy ? laterMessage("import.importing") : importLabel(summary)}
          </Button>
        </>
      }
    >
      {file === undefined ? null : (
        <div className="paper-documents-file">
          <div className="paper-documents-file-head">
            <span className="paper-documents-file-name">{file.name}</span>
            <span className="paper-documents-file-facts">{file.format}</span>
            <Button size={30} onPress={props.onChoose} isDisabled={props.busy}>
              {laterMessage("import.choose")}
            </Button>
          </div>
        </div>
      )}
      {read.state === "reading" ? <p className="paper-field-help">{laterMessage("import.reading")}</p> : null}
      {read.state === "failed" && file !== undefined ? <Failure read={read} format={file.format} /> : null}
      {summary === undefined ? null : (
        <>
          <Lines summary={summary} />
          <Modes value={mode} onChange={setMode} disabled={props.busy} />
          <FileNotices summary={summary} />
          <Skipped summary={summary} />
        </>
      )}
    </Dialog>
  );
}
