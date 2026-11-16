// Export (Paper O16): new files made from the PDF, which itself doesn't change. Four kinds as radio
// cards (pages as images, text, the pictures in the file, links and bookmarks), the pages to read,
// and each kind's options; the primary names the result. A kind the file has none of is disabled
// with "None in this file"; while the file is still being read the cards stay usable. Markdown from
// a file without tags warns that its structure is guessed. The web has no Save to row: the browser
// downloads the result.

import { type ReactNode, useId, useRef, useState } from "react";
import { Radio as AriaRadio, RadioGroup as AriaRadioGroup, Label } from "react-aria-components";
import { Button } from "../components/button.js";
import { Checkbox, Radio, RadioGroup } from "../components/choice.js";
import { Select, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { type LaterMessageId, laterCountMessage, laterMessage } from "../later-messages.js";
import { useOnOpen } from "../organise/on-open.js";
import { Dialog } from "../overlays/dialog.js";
import { type PagesChoice, usePagesChoice } from "./pages-choice.js";
import { chosenPages, currentLabel, type PagesMode } from "./print-pages.js";

/** What to export: pages as images, text, the file's pictures, or its links and bookmarks. */
export type ExportKind = "pages" | "text" | "pictures" | "links";

/** What the user chose to export; `pages` are 0-based, in the order given. */
export type ExportChoice =
  | {
      readonly kind: "pages";
      readonly pages: readonly number[];
      readonly format: "png" | "jpeg" | "tiff";
      readonly dpi: 72 | 150 | 300;
      /** Draw comments and filled-in fields on the pages. */
      readonly annotations: boolean;
    }
  | {
      readonly kind: "text";
      readonly pages: readonly number[];
      readonly format: "text" | "markdown" | "json";
      /** Keep the reading order from the file's tags when it has them. */
      readonly tagged: boolean;
    }
  | {
      readonly kind: "pictures";
      readonly pages: readonly number[];
      /** Keep a JPEG or JPEG 2000 picture's own bytes rather than encoding it again. */
      readonly keepOriginals: boolean;
      /** Pixels on the longer side below which a picture is left out; 0 keeps every one. */
      readonly minimumSize: number;
    }
  | { readonly kind: "links"; readonly format: "json" | "csv" };

/** What the file holds, for the cards and the primary's count. */
export interface ExportCounts {
  readonly pictures: number;
  readonly links: number;
  readonly bookmarks: number;
  /** Whether the file is tagged (has a structure tree). */
  readonly tagged: boolean;
}

/** Props of `ExportDialog`. */
export interface ExportDialogProps {
  isOpen: boolean;
  /** The kind selected when it opens (a palette command opens on its own kind); pages when left out. */
  kind?: ExportKind | undefined;
  pageCount: number;
  /** The page in view, 0-based. */
  currentPage: number;
  /** Each page's label, when the document labels its pages. */
  labels?: readonly string[] | undefined;
  /** Each page's size in points, for the pixel sizes beside each resolution; undefined while unknown. */
  pageSize?: { width: number; height: number } | undefined;
  /** What the file holds; undefined while it is being read (the cards show as loading, not disabled). */
  counts?: ExportCounts | undefined;
  /** An initial page range (from the palette's argument), 0-based. */
  pages?: readonly number[] | undefined;
  /** Export with a valid choice; the shell closes the dialog and shows the progress. */
  onExport: (choice: ExportChoice) => void;
  /** Cancel, Close, Escape or the scrim. */
  onClose: () => void;
}

type ImageFormat = Extract<ExportChoice, { kind: "pages" }>["format"];
type TextFormat = Extract<ExportChoice, { kind: "text" }>["format"];
type LinksFormat = Extract<ExportChoice, { kind: "links" }>["format"];
type Dpi = "72" | "150" | "300";

const KINDS: readonly ExportKind[] = ["pages", "text", "pictures", "links"];
const MINIMUM_SIZES = ["0", "32", "64", "128", "256"] as const;

/** `3–6, 9` for 0-based pages: the numbers people type, consecutive pages joined into a range. */
export function rangeText(pages: readonly number[]): string {
  const parts: string[] = [];
  let start = pages[0];
  for (let i = 1; i <= pages.length; i++) {
    const page = pages[i];
    const previous = pages[i - 1];
    if (start === undefined || previous === undefined) break;
    if (page === previous + 1) continue;
    parts.push(start === previous ? String(start + 1) : `${start + 1}–${previous + 1}`);
    start = page;
  }
  return parts.join(", ");
}

/** Whether the file has nothing of `kind` to export; never while the counts are unknown. */
function isEmpty(kind: ExportKind, counts: ExportCounts | undefined): boolean {
  if (counts === undefined) return false;
  if (kind === "pictures") return counts.pictures === 0;
  if (kind === "links") return counts.links + counts.bookmarks === 0;
  return false;
}

/** Each kind's own options, kept while the dialog is open. */
interface KindOptions {
  readonly imageFormat: ImageFormat;
  readonly dpi: Dpi;
  readonly annotations: boolean;
  readonly textFormat: TextFormat;
  readonly tagged: boolean;
  readonly keepOriginals: boolean;
  readonly minimumSize: (typeof MINIMUM_SIZES)[number];
  readonly linksFormat: LinksFormat;
}

const DEFAULT_OPTIONS: KindOptions = {
  imageFormat: "png",
  dpi: "150",
  annotations: true,
  textFormat: "text",
  tagged: true,
  keepOriginals: true,
  minimumSize: "0",
  linksFormat: "json",
};

/** The dialog's choices and what changes them. */
interface ExportForm extends PagesChoice {
  readonly kind: ExportKind;
  readonly options: KindOptions;
  setKind(kind: ExportKind): void;
  set(options: Partial<KindOptions>): void;
}

/** The pages chosen now, or the range's error; an empty range asks for pages to export. */
function pagesOf(form: Pick<ExportForm, "mode" | "range">, props: ExportDialogProps) {
  if (form.mode === "range" && form.range.trim() === "") return { error: laterMessage("export.range.empty") };
  return chosenPages(form.mode, form.range, props);
}

/** The choice the form makes for `pages`. */
function choiceOf(kind: ExportKind, pages: readonly number[], o: KindOptions): ExportChoice {
  switch (kind) {
    case "pages":
      return { kind, pages, format: o.imageFormat, dpi: Number(o.dpi) as 72 | 150 | 300, annotations: o.annotations };
    case "text":
      return { kind, pages, format: o.textFormat, tagged: o.tagged };
    case "pictures":
      return { kind, pages, keepOriginals: o.keepOriginals, minimumSize: Number(o.minimumSize) };
    case "links":
      return { kind, format: o.linksFormat };
  }
}

/**
 * The choices, and `submit`: the choice they make, or the range's error (shown, and the field
 * focused). Each opening starts on `props.kind` and on `props.pages` when given, else every page.
 */
function useExportForm(
  props: ExportDialogProps,
  focusRange: () => void,
): ExportForm & { submit(): ExportChoice | undefined } {
  const { showError, reset, ...pages } = usePagesChoice();
  const [kind, setKind] = useState<ExportKind>("pages");
  const [options, setOptions] = useState(DEFAULT_OPTIONS);
  useOnOpen(props.isOpen, () => {
    setKind(props.kind ?? "pages");
    const given = props.pages !== undefined && props.pages.length > 0;
    reset(given ? "range" : "all", given ? rangeText(props.pages ?? []) : "");
  });
  return {
    ...pages,
    kind,
    options,
    setKind,
    set: (next) => setOptions((now) => ({ ...now, ...next })),
    submit: () => {
      if (kind === "links") return choiceOf(kind, [], options);
      const chosen = pagesOf(pages, props);
      if ("error" in chosen) {
        showError(chosen.error);
        focusRange();
        return undefined;
      }
      return choiceOf(kind, chosen.pages, options);
    },
  };
}

/** The four kinds as radio cards in a 2 × 2 grid: real radios inside their labels. */
function KindCards({ form, counts }: { form: ExportForm; counts: ExportCounts | undefined }): ReactNode {
  const id = useId();
  return (
    <AriaRadioGroup
      className="paper-export-kinds"
      value={form.kind}
      onChange={(value) => form.setKind(value as ExportKind)}
      aria-busy={counts === undefined ? true : undefined}
    >
      <Label className="paper-field-label">{laterMessage("export.what")}</Label>
      <div className="paper-export-cards">
        {KINDS.map((kind) => {
          const empty = isEmpty(kind, counts);
          return (
            <AriaRadio
              key={kind}
              value={kind}
              isDisabled={empty}
              aria-describedby={`${id}-${kind}`}
              className="paper-choice paper-radio paper-export-card"
            >
              <span className="paper-radio-circle" aria-hidden="true" />
              <span className="paper-export-card-text">
                <span className="paper-export-card-title">{laterMessage(`export.kind.${kind}`)}</span>
                {/* The card's title names its radio; this line describes it, read after the name. */}
                <span className="paper-export-card-help" id={`${id}-${kind}`} aria-hidden="true">
                  {laterMessage(empty ? "export.kind.none" : `export.kind.${kind}.help`)}
                </span>
              </span>
            </AriaRadio>
          );
        })}
      </div>
    </AriaRadioGroup>
  );
}

/** All N · This page (n) · Pages, with the range field under them. */
function PagesFields({ form, props }: { form: ExportForm; props: ExportDialogProps }): ReactNode {
  return (
    <>
      <RadioGroup label={laterMessage("export.pages")} value={form.mode} onChange={(v) => form.setMode(v as PagesMode)}>
        <Radio value="all">{laterMessage("export.pages.all", { count: props.pageCount })}</Radio>
        <Radio value="current">{laterMessage("export.pages.current", { page: currentLabel(props) })}</Radio>
        <Radio value="range">{laterMessage("export.pages.range")}</Radio>
      </RadioGroup>
      <div className="paper-export-range">
        <TextInput
          label={laterMessage("export.pages.rangeField")}
          placeholder={laterMessage("print.pages.rangePlaceholder")}
          description={laterMessage("print.pages.rangeHelp")}
          value={form.range}
          onChange={form.setRange}
          onFocus={form.chooseRange}
          {...(form.error === undefined ? {} : { errorMessage: form.error })}
        />
      </div>
    </>
  );
}

/** A horizontal radio group of `values`, each named by its message. */
function FormatGroup<Value extends string>(props: {
  value: Value;
  values: readonly Value[];
  words: (value: Value) => LaterMessageId;
  onChange: (value: Value) => void;
}): ReactNode {
  return (
    <div className="paper-export-row">
      <RadioGroup
        label={laterMessage("export.format")}
        orientation="horizontal"
        value={props.value}
        onChange={(v) => props.onChange(v as Value)}
      >
        {props.values.map((value) => (
          <Radio key={value} value={value}>
            {laterMessage(props.words(value))}
          </Radio>
        ))}
      </RadioGroup>
    </div>
  );
}

/** `150 dpi · 1,275 × 1,650 px`, or `150 dpi` while the page's size is unknown. */
function resolutionLabel(dpi: Dpi, size: ExportDialogProps["pageSize"]): string {
  if (size === undefined) return laterMessage("export.resolution.dpi", { dpi });
  const px = (points: number) => new Intl.NumberFormat().format(Math.round((points * Number(dpi)) / 72));
  return laterMessage("export.resolution.size", { dpi, width: px(size.width), height: px(size.height) });
}

/** Pages as images: the format, the resolution and whether comments are drawn. */
function ImageOptions({ form, pageSize }: { form: ExportForm; pageSize: ExportDialogProps["pageSize"] }): ReactNode {
  const o = form.options;
  return (
    <>
      <FormatGroup<ImageFormat>
        value={o.imageFormat}
        values={["png", "jpeg", "tiff"]}
        words={(v) => `export.format.${v}`}
        onChange={(imageFormat) => form.set({ imageFormat })}
      />
      <Select<Dpi>
        label={laterMessage("export.resolution")}
        value={o.dpi}
        onChange={(dpi) => form.set({ dpi })}
        options={(["72", "150", "300"] as const).map((dpi) => ({ value: dpi, label: resolutionLabel(dpi, pageSize) }))}
      />
      <Checkbox isSelected={o.annotations} onChange={(annotations) => form.set({ annotations })}>
        {laterMessage("export.annotations")}
      </Checkbox>
    </>
  );
}

/** Text: the format, the tags' reading order, and the notice for Markdown from an untagged file. */
function TextOptions({ form, counts }: { form: ExportForm; counts: ExportCounts | undefined }): ReactNode {
  const o = form.options;
  return (
    <>
      <FormatGroup<TextFormat>
        value={o.textFormat}
        values={["text", "markdown", "json"]}
        words={(v) => `export.format.${v}`}
        onChange={(textFormat) => form.set({ textFormat })}
      />
      {o.textFormat === "markdown" && counts?.tagged === false ? (
        <Notice tone="warning" title={laterMessage("export.untagged.title")}>
          {laterMessage("export.untagged")}
        </Notice>
      ) : null}
      <Checkbox isSelected={o.tagged} onChange={(tagged) => form.set({ tagged })}>
        {laterMessage("export.tagged")}
      </Checkbox>
    </>
  );
}

/** Pictures: keep their own bytes, and the smallest worth exporting. */
function PictureOptions({ form }: { form: ExportForm }): ReactNode {
  const o = form.options;
  return (
    <>
      <Checkbox isSelected={o.keepOriginals} onChange={(keepOriginals) => form.set({ keepOriginals })}>
        {laterMessage("export.originals")}
      </Checkbox>
      <Select<KindOptions["minimumSize"]>
        label={laterMessage("export.minimum")}
        description={laterMessage("export.minimum.help")}
        value={o.minimumSize}
        onChange={(minimumSize) => form.set({ minimumSize })}
        options={MINIMUM_SIZES.map((size) => ({
          value: size,
          label: size === "0" ? laterMessage("export.minimum.any") : laterMessage("export.minimum.px", { size }),
        }))}
      />
    </>
  );
}

/** The selected kind's options. */
function KindOptionsFields({ form, props }: { form: ExportForm; props: ExportDialogProps }): ReactNode {
  switch (form.kind) {
    case "pages":
      return <ImageOptions form={form} pageSize={props.pageSize} />;
    case "text":
      return <TextOptions form={form} counts={props.counts} />;
    case "pictures":
      return <PictureOptions form={form} />;
    case "links":
      return (
        <FormatGroup<LinksFormat>
          value={form.options.linksFormat}
          values={["json", "csv"]}
          words={(v) => (v === "json" ? "export.format.links.json" : "export.format.csv")}
          onChange={(linksFormat) => form.set({ linksFormat })}
        />
      );
  }
}

/** The primary's words, naming the result: "Export 4 pages", "Export 12 pictures". */
function primaryLabel(form: ExportForm, props: ExportDialogProps): string {
  if (form.kind === "text") return laterMessage("export.go.text");
  if (form.kind === "links") return laterMessage("export.go.links");
  const chosen = pagesOf(form, props);
  if (form.kind === "pages") {
    if ("error" in chosen) return laterMessage("export.go.pages.some");
    return laterCountMessage(chosen.pages.length, { one: "export.go.pages.one", many: "export.go.pages" });
  }
  // The file's count holds only when every page is read.
  const every = "pages" in chosen && chosen.pages.length === props.pageCount;
  if (props.counts === undefined || !every) return laterMessage("export.go.pictures.some");
  return laterCountMessage(props.counts.pictures, { one: "export.go.pictures.one", many: "export.go.pictures" });
}

/** Asks what to export from the PDF, from which pages, and how. */
export function ExportDialog(props: ExportDialogProps): ReactNode {
  const fields = useRef<HTMLFormElement>(null);
  const form = useExportForm(props, () =>
    fields.current?.querySelector<HTMLInputElement>(".paper-export-range input")?.focus(),
  );
  const exportNow = () => {
    if (isEmpty(form.kind, props.counts)) return;
    const choice = form.submit();
    if (choice !== undefined) props.onExport(choice);
  };
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      size="form"
      title={laterMessage("export.title")}
      description={laterMessage("export.description")}
      closeLabel={laterMessage("export.close")}
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("export.cancel")}</Button>
          <Button variant="primary" isDisabled={isEmpty(form.kind, props.counts)} onPress={exportNow}>
            {primaryLabel(form, props)}
          </Button>
        </>
      }
    >
      <form
        className="paper-export-fields"
        ref={fields}
        onSubmit={(event) => {
          event.preventDefault();
          exportNow();
        }}
      >
        <KindCards form={form} counts={props.counts} />
        {form.kind === "links" ? null : <PagesFields form={form} props={props} />}
        <KindOptionsFields form={form} props={props} />
        {/* Enter in a field submits the form, which exports. */}
        <button type="submit" hidden />
      </form>
    </Dialog>
  );
}
