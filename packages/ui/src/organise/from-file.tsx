// Insert pages from a PDF and Replace pages (Paper O07): the chosen file's card (with its password
// asked for inline when it is encrypted, R03), the pages to bring in by range or by their
// thumbnails, where they go, and what comes with them. The shell reads the file, draws its pages and
// applies the edit; the dialog says what it will do before Apply.

import { type ReactNode, useEffect, useRef, useState } from "react";
import { Button } from "../components/button.js";
import { Checkbox } from "../components/choice.js";
import { Select, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { Skeleton } from "../components/progress.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import { formatBytes } from "../reader/bytes.js";
import { parsePrintPages } from "../reader/print-pages.js";
import { pageNamed } from "./dialogs.js";
import { pageRanges } from "./moves.js";
import { useOnOpen } from "./on-open.js";

/** A file a dialog reads pages from, as the shell has read it so far. */
export type SourceFile = { readonly name: string; readonly size: number } & (
  | { readonly status: "loading" }
  /** Encrypted: its password is asked for in the card; `wrong` after a password that didn't open it. */
  | { readonly status: "password"; readonly wrong: boolean }
  | { readonly status: "unreadable"; readonly error: string }
  | {
      readonly status: "ready";
      /** Each page's displayed size, in points. */
      readonly pages: readonly { readonly width: number; readonly height: number }[];
    }
);

/** Where the pages go: beside a page of this document, at either end, or in place of the selection. */
export type FromFileWhere = "after" | "before" | "start" | "end" | "replace";

/** What Insert or Replace does. */
export interface FromFileChoice {
  /** The file's pages to bring in (0-based, in order). */
  readonly pages: readonly number[];
  readonly where: FromFileWhere;
  /** For `after` and `before`: the page of this document (0-based). */
  readonly page: number;
  /** Whether the pages' comments and links come along. */
  readonly annotations: boolean;
  /** Whether a form field whose name is taken shares the value of the field that has it, rather than being renamed. */
  readonly linkFields: boolean;
}

/** Props of `PagesFromFileDialog`. */
export interface PagesFromFileDialogProps {
  /** Whether the dialog shows. */
  readonly isOpen: boolean;
  /** Called when it closes, applied or not. */
  readonly onClose: () => void;
  /** `insert` opens it as Insert pages, `replace` with Replace selected pages chosen. */
  readonly mode: "insert" | "replace";
  /** The chosen file, or undefined before one is chosen. */
  readonly source: SourceFile | undefined;
  /** Asks for another file. */
  readonly onChoose: () => void;
  /** Tries `password` on the encrypted file. */
  readonly onPassword: (password: string) => void;
  /** Draws page `page` of the file into `canvas`, at its size in device pixels. */
  readonly draw: (page: number, canvas: HTMLCanvasElement, signal: AbortSignal) => Promise<void>;
  /** This document's pages' labels, in order. */
  readonly labels: readonly string[];
  /** The selected pages of this document (0-based, in order): replaced, or the page they go beside. */
  readonly selected: readonly number[];
  /** Applies the choice; the dialog closes. */
  readonly onApply: (choice: FromFileChoice) => void;
}

/** "Insert 4 pages", "Replace 2 pages with 4": the primary button's words. */
function applyLabel(where: FromFileWhere, count: number, replaced: number): string {
  if (where !== "replace")
    return laterCountMessage(count, { one: "documents.insert.go.one", many: "documents.insert.go" });
  return laterMessage(replaced === 1 ? "documents.replace.go.one" : "documents.replace.go", { replaced, count });
}

/** The 0-based place the first new page takes. */
function firstPlace(where: FromFileWhere, page: number, total: number, selected: readonly number[]): number {
  if (where === "start") return 0;
  if (where === "end") return total;
  if (where === "replace") return selected[0] ?? 0;
  return where === "before" ? page : page + 1;
}

/** The pages a range names in a file of `total` pages (each once, in order), or why it names none. */
function readRange(range: string, total: number): { chosen: number[]; error?: string } {
  if (total === 0 || range.trim() === "") return { chosen: [] };
  const read = parsePrintPages(range, total);
  return read.ok ? { chosen: [...new Set(read.pages)] } : { chosen: [], error: read.error };
}

/** Where the pages go, and whether that is a place: a page named for `after` and `before`. */
function readPlace(where: FromFileWhere, page: string, labels: readonly string[]) {
  const beside = where === "after" || where === "before";
  const target = beside ? pageNamed(page, labels) : 0;
  const named = !beside || page.trim() !== "";
  const error =
    beside && named && target === undefined ? laterMessage("organise.move.page.error", { page }) : undefined;
  return { beside, target, named, error };
}

/** The form's state, seeded as the dialog opens and as a file is read. */
function useFromFileForm(props: PagesFromFileDialogProps) {
  const { source, labels, selected } = props;
  const total = source?.status === "ready" ? source.pages.length : 0;
  const [range, setRange] = useState("");
  const [where, setWhere] = useState<FromFileWhere>("after");
  const [page, setPage] = useState("");
  const [annotations, setAnnotations] = useState(true);
  const [linkFields, setLinkFields] = useState(false);
  const [password, setPassword] = useState("");
  const [seededFor, setSeededFor] = useState<SourceFile | undefined>(undefined);
  useOnOpen(props.isOpen, () => {
    const some = selected.length > 0;
    setWhere(props.mode === "replace" && some ? "replace" : some ? "after" : "end");
    setPage(some ? (labels[selected.at(-1) as number] ?? "") : "");
    setAnnotations(true);
    setLinkFields(false);
    setPassword("");
  });
  // A newly read file starts with every page chosen.
  if (source !== seededFor) {
    setSeededFor(source);
    if (source?.status === "ready") setRange(total === 1 ? "1" : `1–${total}`);
    if (source?.status !== "password") setPassword("");
  }
  return {
    total,
    range,
    setRange,
    where,
    setWhere,
    page,
    setPage,
    annotations,
    setAnnotations,
    linkFields,
    setLinkFields,
    password,
    setPassword,
  };
}

/** Insert pages from a PDF, and Replace selected pages. */
export function PagesFromFileDialog(props: PagesFromFileDialogProps): ReactNode {
  const { source, labels, selected } = props;
  const form = useFromFileForm(props);
  const { chosen, error: rangeError } = readRange(form.range, form.total);
  const place = readPlace(form.where, form.page, labels);
  const ready = source?.status === "ready" && chosen.length > 0 && place.target !== undefined;
  const canApply = ready && place.named && (form.where !== "replace" || selected.length > 0);
  const at = firstPlace(form.where, place.target ?? 0, labels.length, selected);
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      title={laterMessage(props.mode === "replace" ? "documents.replace.title" : "documents.insert.title")}
      closeLabel={laterMessage("organise.close")}
      size="form"
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("organise.move.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={!canApply}
            onPress={() => {
              const { where, annotations, linkFields } = form;
              props.onApply({ pages: chosen, where, page: place.target ?? 0, annotations, linkFields });
              props.onClose();
            }}
          >
            {applyLabel(form.where, chosen.length, selected.length)}
          </Button>
        </>
      }
    >
      <FileCard
        source={source}
        password={form.password}
        onPassword={form.setPassword}
        onTry={() => props.onPassword(form.password)}
        onChoose={props.onChoose}
      />
      {source?.status === "ready" ? (
        <>
          <PagesPicker form={form} pages={source.pages} chosen={chosen} error={rangeError} draw={props.draw} />
          <PlaceFields form={form} labels={labels} canReplace={selected.length > 0} place={place} />
          <Checkbox isSelected={form.annotations} onChange={form.setAnnotations}>
            {laterMessage("documents.insert.annotations")}
          </Checkbox>
          <Checkbox isSelected={form.linkFields} onChange={form.setLinkFields}>
            {laterMessage("documents.insert.linkFields")}
          </Checkbox>
          {ready ? (
            <p className="paper-documents-result">
              {laterCountMessage(
                chosen.length,
                { one: "documents.insert.result.one", many: "documents.insert.result" },
                { first: at + 1, last: at + chosen.length },
              )}
            </p>
          ) : null}
        </>
      ) : null}
    </Dialog>
  );
}

type FromFileForm = ReturnType<typeof useFromFileForm>;

/** The file's pages to bring in: the range, Select all, and the checkable thumbnails, kept in step. */
function PagesPicker({
  form,
  pages,
  chosen,
  error,
  draw,
}: {
  form: FromFileForm;
  pages: readonly { readonly width: number; readonly height: number }[];
  chosen: readonly number[];
  error: string | undefined;
  draw: PagesFromFileDialogProps["draw"];
}): ReactNode {
  const { total, range, setRange } = form;
  const toggle = (index: number) => {
    const next = new Set(chosen);
    if (next.has(index)) next.delete(index);
    else next.add(index);
    setRange(
      pageRanges(
        [...next].sort((a, b) => a - b),
        (i) => String(i + 1),
      ),
    );
  };
  return (
    <>
      <div className="paper-documents-range">
        <TextInput
          label={laterMessage("documents.insert.pages")}
          description={laterMessage("documents.insert.pages.hint", { selected: chosen.length, total })}
          value={range}
          onChange={setRange}
          {...(error === undefined ? {} : { errorMessage: error })}
        />
        <Button size={30} onPress={() => setRange(total === 1 ? "1" : `1–${total}`)}>
          {laterMessage("documents.insert.selectAll")}
        </Button>
      </div>
      <SourceThumbnails pages={pages} chosen={new Set(chosen)} onToggle={toggle} draw={draw} />
    </>
  );
}

/** Where the pages go: the position, and the page beside which. */
function PlaceFields({
  form,
  labels,
  canReplace,
  place,
}: {
  form: FromFileForm;
  labels: readonly string[];
  canReplace: boolean;
  place: ReturnType<typeof readPlace>;
}): ReactNode {
  return (
    <div className="paper-documents-where">
      <Select<FromFileWhere>
        label={laterMessage("documents.insert.where")}
        value={form.where}
        onChange={form.setWhere}
        options={[
          { value: "after", label: laterMessage("organise.move.after") },
          { value: "before", label: laterMessage("organise.move.before") },
          { value: "start", label: laterMessage("organise.move.start") },
          { value: "end", label: laterMessage("organise.move.end") },
          ...(canReplace ? [{ value: "replace" as const, label: laterMessage("documents.insert.where.replace") }] : []),
        ]}
      />
      {place.beside ? (
        <TextInput
          label={laterMessage("organise.move.page")}
          description={laterMessage("organise.move.page.hint", { count: labels.length })}
          value={form.page}
          onChange={form.setPage}
          {...(place.error === undefined ? {} : { errorMessage: place.error })}
        />
      ) : null}
    </div>
  );
}

/** Props of `FileCard`. */
interface FileCardProps {
  readonly source: SourceFile | undefined;
  readonly password: string;
  readonly onPassword: (password: string) => void;
  readonly onTry: () => void;
  readonly onChoose: () => void;
}

/** The chosen file: its name, pages and size, "Choose another…", and its password or why it can't be read. */
export function FileCard({ source, password, onPassword, onTry, onChoose }: FileCardProps): ReactNode {
  if (source === undefined) {
    return (
      <div className="paper-documents-file">
        <Button onPress={onChoose}>{laterMessage("documents.file.choose")}</Button>
      </div>
    );
  }
  const facts =
    source.status === "ready"
      ? laterCountMessage(
          source.pages.length,
          { one: "documents.file.facts.one", many: "documents.file.facts" },
          { size: formatBytes(source.size) },
        )
      : formatBytes(source.size);
  return (
    <section className="paper-documents-file" aria-label={laterMessage("documents.file.label", { name: source.name })}>
      <div className="paper-documents-file-head">
        <span className="paper-documents-file-name">{source.name}</span>
        <span className="paper-documents-file-facts">{facts}</span>
        <Button size={30} onPress={onChoose}>
          {laterMessage("documents.file.another")}
        </Button>
      </div>
      {source.status === "password" ? (
        <form
          className="paper-documents-password"
          // The field shows a wrong password as its error, which the browser would take for a
          // reason to stop the next submit: the form leaves checking to the dialog.
          noValidate
          onSubmit={(event) => {
            event.preventDefault();
            onTry();
          }}
        >
          <TextInput
            label={laterMessage("documents.file.password")}
            type="password"
            value={password}
            onChange={onPassword}
            autoFocus
            {...(source.wrong ? { errorMessage: laterMessage("documents.file.password.wrong") } : {})}
          />
          <Button type="submit" size={30} isDisabled={password === ""}>
            {laterMessage("documents.file.password.open")}
          </Button>
        </form>
      ) : null}
      {source.status === "unreadable" ? (
        <Notice tone="danger" title={laterMessage("documents.file.unreadable")}>
          {source.error}
        </Notice>
      ) : null}
    </section>
  );
}

/** The file's pages as labelled checkboxes, each with its thumbnail. */
function SourceThumbnails({
  pages,
  chosen,
  onToggle,
  draw,
}: {
  pages: readonly { readonly width: number; readonly height: number }[];
  chosen: ReadonlySet<number>;
  onToggle: (index: number) => void;
  draw: PagesFromFileDialogProps["draw"];
}): ReactNode {
  return (
    <ul className="paper-documents-thumbs" aria-label={laterMessage("documents.insert.thumbs")}>
      {pages.map((page, index) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: a file's pages are fixed while it is shown.
        <li key={index} className="paper-documents-thumb" data-chosen={chosen.has(index) ? true : undefined}>
          <Checkbox isSelected={chosen.has(index)} onChange={() => onToggle(index)}>
            <SourcePage index={index} width={page.width} height={page.height} draw={draw} />
            <span>{laterMessage("documents.insert.thumb", { page: index + 1 })}</span>
          </Checkbox>
        </li>
      ))}
    </ul>
  );
}

/** One page's thumbnail: a skeleton until the shell has drawn it. */
function SourcePage({
  index,
  width,
  height,
  draw,
}: {
  index: number;
  width: number;
  height: number;
  draw: PagesFromFileDialogProps["draw"];
}): ReactNode {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [drawn, setDrawn] = useState(false);
  useEffect(() => {
    const element = canvas.current;
    if (element === null) return;
    const abort = new AbortController();
    draw(index, element, abort.signal).then(
      () => {
        if (!abort.signal.aborted) setDrawn(true);
      },
      () => undefined,
    );
    return () => abort.abort();
  }, [index, draw]);
  const ratio = height > 0 ? width / height : 1;
  return (
    <span className="paper-documents-thumb-page" data-landscape={ratio > 1 ? true : undefined}>
      <canvas ref={canvas} hidden={!drawn} />
      {drawn ? null : <Skeleton inlineSize="100%" blockSize="100%" />}
    </span>
  );
}
