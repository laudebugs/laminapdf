// Split document (Paper O08), which also makes Export ranges: every n pages, at page ranges, or at
// the top-level bookmarks; a live preview of the files it makes and their names; how they are
// named; the progress while they are written. The parts are new files: the document doesn't
// change, nothing is undone, and a signed file's parts aren't signed, which the dialog says.

import { type ReactNode, useState } from "react";
import { Button } from "../components/button.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { Select, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { ProgressBar } from "../components/progress.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import { pageRanges } from "./moves.js";
import { useOnOpen } from "./on-open.js";
import {
  type Chapter,
  type Part,
  type PartNaming,
  type PartsResult,
  partNames,
  partsAtBookmarks,
  partsEvery,
  partsOfRanges,
} from "./parts.js";

/** How Split cuts the document. */
export type SplitBy = "count" | "ranges" | "bookmarks";

/** One file Split writes: its pages (0-based) and its name, without `.pdf`. */
export interface NamedPart extends Part {
  readonly name: string;
}

/** Props of `SplitDialog`. */
export interface SplitDialogProps {
  /** Whether the dialog shows. */
  readonly isOpen: boolean;
  /** Called when it closes. */
  readonly onClose: () => void;
  /** `ranges` opens it as Export ranges, with the ranges chosen and the selection filled in. */
  readonly start: SplitBy;
  /** The pages' labels, in order. */
  readonly labels: readonly string[];
  /** The selected pages (0-based), which Export ranges starts from. */
  readonly selected: readonly number[];
  /** The top-level bookmarks, in outline order. */
  readonly chapters: readonly Chapter[];
  /** The open file's name without `.pdf`, which the parts are named from. */
  readonly stem: string;
  /** Where `parts` files go, said under the preview: a folder the person picks, one zip, each file on its own, or the one file. */
  readonly output: (parts: number) => "folder" | "zip" | "files" | "file";
  /** Whether the file is signed or certified: its parts aren't. */
  readonly signed: boolean;
  /** How far the writing is: undefined until it starts. */
  readonly progress: { readonly done: number; readonly total: number } | undefined;
  /** Writes the parts; the shell closes the dialog once they are written. */
  readonly onSplit: (parts: readonly NamedPart[]) => void;
}

/** The parts `by` makes with these inputs. */
function partsBy(
  by: SplitBy,
  count: string,
  ranges: string,
  labels: readonly string[],
  chapters: readonly Chapter[],
): PartsResult {
  if (by === "bookmarks") return partsAtBookmarks(chapters, labels.length);
  if (by === "ranges") return partsOfRanges(ranges, labels.length, labels);
  const n = /^\d+$/.test(count.trim()) ? Number(count.trim()) : Number.NaN;
  return partsEvery(n, labels.length);
}

/** Split document, and Export ranges. */
export function SplitDialog(props: SplitDialogProps): ReactNode {
  const { labels, chapters, progress } = props;
  const [by, setBy] = useState<SplitBy>("count");
  const [count, setCount] = useState("1");
  const [ranges, setRanges] = useState("");
  const [naming, setNaming] = useState<PartNaming>("name-number");
  useOnOpen(props.isOpen, () => {
    const start = props.start === "bookmarks" && chapters.length === 0 ? "count" : props.start;
    setBy(start);
    setCount("1");
    const label = (i: number) => labels[i] ?? String(i + 1);
    setRanges(props.selected.length > 0 ? pageRanges(props.selected, label) : "");
    setNaming(start === "bookmarks" ? "number-title" : start === "ranges" ? "name-pages" : "name-number");
  });
  const result = partsBy(by, count, ranges, labels, chapters);
  const label = (i: number) => labels[i] ?? String(i + 1);
  const parts = result.ok ? result.parts : [];
  const names = partNames(parts, naming, props.stem, label);
  const named = parts.map((part, i) => ({ ...part, name: names[i] ?? "" }));
  const busy = progress !== undefined;
  const typed = by === "ranges" ? ranges.trim() !== "" : by === "count" ? count.trim() !== "" : true;
  const error = !result.ok && typed ? result.error : undefined;
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open && !busy) props.onClose();
      }}
      title={laterMessage(props.start === "ranges" ? "documents.split.title.ranges" : "documents.split.title")}
      closeLabel={laterMessage("organise.close")}
      size="form"
      actions={
        <>
          <Button onPress={props.onClose} isDisabled={busy}>
            {laterMessage("organise.move.cancel")}
          </Button>
          <Button variant="primary" isDisabled={!result.ok || busy} onPress={() => props.onSplit(named)}>
            {laterCountMessage(parts.length, { one: "documents.split.go.one", many: "documents.split.go" })}
          </Button>
        </>
      }
    >
      <SplitOptions
        by={by}
        onBy={(value) => {
          setBy(value);
          if (value === "bookmarks") setNaming("number-title");
        }}
        count={count}
        onCount={setCount}
        ranges={ranges}
        onRanges={setRanges}
        chapters={chapters.length}
        error={error}
        busy={busy}
      />
      <Select<PartNaming>
        label={laterMessage("documents.split.naming")}
        value={naming}
        onChange={setNaming}
        isDisabled={busy}
        options={[
          { value: "number-title", label: laterMessage("documents.split.naming.title") },
          { value: "name-number", label: laterMessage("documents.split.naming.number") },
          { value: "name-pages", label: laterMessage("documents.split.naming.pages") },
        ]}
      />
      <SplitPreview parts={named} result={result} label={label} />
      <p className="paper-documents-result">{laterMessage(`documents.split.output.${props.output(parts.length)}`)}</p>
      {props.signed ? (
        <Notice tone="warning" title={laterMessage("documents.split.signed.title")}>
          {laterMessage("documents.split.signed")}
        </Notice>
      ) : null}
      {progress === undefined ? null : (
        <ProgressBar
          label={laterMessage("documents.split.progress", {
            file: Math.min(progress.done + 1, progress.total),
            total: progress.total,
          })}
          value={progress.total === 0 ? 0 : (progress.done / progress.total) * 100}
        />
      )}
    </Dialog>
  );
}

/** Split by: every n pages, page ranges or top-level bookmarks, the inactive options' inputs disabled. */
function SplitOptions({
  by,
  onBy,
  count,
  onCount,
  ranges,
  onRanges,
  chapters,
  error,
  busy,
}: {
  by: SplitBy;
  onBy: (by: SplitBy) => void;
  count: string;
  onCount: (count: string) => void;
  ranges: string;
  onRanges: (ranges: string) => void;
  /** How many top-level bookmarks there are. */
  chapters: number;
  error: string | undefined;
  busy: boolean;
}): ReactNode {
  return (
    <RadioGroup
      label={laterMessage("documents.split.by")}
      value={by}
      onChange={(value) => onBy(value as SplitBy)}
      isDisabled={busy}
    >
      <Radio value="count">{laterMessage("documents.split.by.count")}</Radio>
      <div className="paper-documents-inline">
        <TextInput
          label={laterMessage("documents.split.count")}
          value={count}
          onChange={onCount}
          inputMode="numeric"
          isDisabled={by !== "count" || busy}
          {...(by === "count" && error !== undefined ? { errorMessage: error } : {})}
        />
      </div>
      <Radio value="ranges">{laterMessage("documents.split.by.ranges")}</Radio>
      <div className="paper-documents-inline">
        <TextInput
          label={laterMessage("documents.split.ranges")}
          description={laterMessage("documents.split.ranges.hint")}
          value={ranges}
          onChange={onRanges}
          isDisabled={by !== "ranges" || busy}
          {...(by === "ranges" && error !== undefined ? { errorMessage: error } : {})}
        />
      </div>
      <Radio value="bookmarks" isDisabled={chapters === 0}>
        {chapters === 0
          ? laterMessage("documents.split.by.bookmarks.none")
          : laterCountMessage(chapters, {
              one: "documents.split.by.bookmarks.one",
              many: "documents.split.by.bookmarks",
            })}
      </Radio>
    </RadioGroup>
  );
}

/** The files the split makes, named, with their pages, and the pages two of them share. */
function SplitPreview({
  parts,
  result,
  label,
}: {
  parts: readonly NamedPart[];
  result: PartsResult;
  label: (index: number) => string;
}): ReactNode {
  return (
    <>
      <h3 className="paper-documents-heading" id="paper-split-preview">
        {laterCountMessage(parts.length, { one: "documents.split.preview.one", many: "documents.split.preview" })}
      </h3>
      <ol className="paper-documents-parts" aria-labelledby="paper-split-preview">
        {parts.map((part, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: the preview is the list as it stands; it holds no state.
          <li key={i}>
            <span className="paper-documents-file-name">{part.name}.pdf</span>
            <span className="paper-documents-file-facts">{pageRanges(part.pages, label)}</span>
          </li>
        ))}
      </ol>
      <p className="paper-visually-hidden" role="status">
        {result.ok
          ? laterCountMessage(parts.length, { one: "documents.split.preview.one", many: "documents.split.preview" })
          : ""}
      </p>
      {result.ok && result.shared.length > 0 ? (
        <p className="paper-documents-result">
          {laterMessage("documents.split.shared", { pages: pageRanges(result.shared, label) })}
        </p>
      ) : null}
    </>
  );
}
