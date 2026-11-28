// The Page marks dialog (Paper A03): Watermark, Header & footer and Bates numbers tabs, a preview
// of the mark over the page's picture with a page stepper, the pages it goes on, the notice of
// text it would run into, and a footer that says what the primary button does. The same dialog
// edits a mark that was added ("Edit watermark", Update, Remove). It is controlled: the app reads
// the pictures and the text on the pages, makes the ops, and shows the progress; nothing is
// applied until the primary button.

import { type ReactNode, useEffect, useState } from "react";
import { useLoaded } from "../comment/use-loaded.js";
import { Button } from "../components/button.js";
import { Notice } from "../components/notice.js";
import { ProgressBar } from "../components/progress.js";
import { SegmentedTabs, TabPanel } from "../components/tabs.js";
import { SignatureDetails, type SignatureRestriction } from "../edit/save-ui.js";
import { laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import { BatesTab } from "./bates-tab.js";
import { type MarkFont, PagesField } from "./fields.js";
import { type FormState, initialForm, type MarkStart, pagesOf, waitingFor } from "./form.js";
import { HeaderFooterTab } from "./header-footer-tab.js";
import {
  type BatesValues,
  bandTexts,
  batesText,
  formatDate,
  type HeaderFooterValues,
  MARK_TABS,
  type MarkTab,
  type Overlap,
  overlapOf,
  type PageSize,
  type PageText,
  summaryOf,
  textOfTab,
  type WatermarkValues,
} from "./model.js";
import { MarkPreview, type PreviewPicture } from "./preview.js";
import {
  drawSlots,
  drawWatermark,
  type MarkImage,
  type PreviewPage,
  type SlotLine,
  slotLines,
} from "./preview-draw.js";
import { WatermarkTab } from "./watermark-tab.js";

/** What the person chose, handed to the app when they press the primary button. */
export type MarkChoice =
  | {
      readonly tab: "watermark";
      readonly values: WatermarkValues;
      readonly image: MarkImage | undefined;
      readonly pages: readonly number[];
    }
  | {
      readonly tab: "headerFooter";
      readonly values: HeaderFooterValues;
      readonly pages: readonly number[];
      /** What the date token stands for, as the chosen format writes it. */
      readonly date: string;
    }
  | { readonly tab: "bates"; readonly values: BatesValues; readonly pages: readonly number[] };

/** Props of `PageMarksDialog`. */
export interface PageMarksDialogProps {
  readonly isOpen: boolean;
  /** Cancel, the close button, Escape, or the dialog finishing. */
  readonly onClose: () => void;
  /** What to open on: a tab to add a mark on, or a mark to change. */
  readonly start: MarkStart;
  /** Each page's label, in page order; the count is the document's. */
  readonly labels: readonly string[];
  readonly fileName: string;
  readonly today: Date;
  readonly fonts: readonly MarkFont[];
  /** The characters of `text` no font would draw when the mark is set in font `fontId`. */
  readonly lacks: (fontId: string, text: string) => readonly string[];
  /** The CSS font stack the browser draws font `fontId` in. */
  readonly css: (fontId: string) => string;
  /** Whether a mark may be hideable: not on a PDF/A-1 file, which has no optional content. */
  readonly canHide: boolean;
  /** Page `page`'s size in points. */
  readonly sizeOf: (page: number) => PageSize;
  /** Page `page` drawn as it is now. */
  readonly picture: (page: number) => Promise<CanvasImageSource | undefined>;
  /** The text page `page` already shows. */
  readonly existing: (page: number) => Promise<readonly PageText[]>;
  /** Asks for a picture; undefined when none was chosen or it isn't one. */
  readonly pickImage: () => Promise<MarkImage | undefined>;
  /** The mark is being added: how far along (0 to 100) when that is known. */
  readonly working?: { readonly value?: number } | undefined;
  /** What a certification would lose: the restrictions the mark's save would break. */
  readonly signed?: readonly SignatureRestriction[] | undefined;
  /** Adds the mark, or updates the one being edited. */
  readonly onApply: (choice: MarkChoice) => void;
  /** Takes the edited mark off its pages. */
  readonly onRemove?: (() => void) | undefined;
}

/** Counts openings: a new count remounts the form, which drops the last opening's choices. */
export function PageMarksDialog(props: PageMarksDialogProps): ReactNode {
  const [opening, setOpening] = useState(0);
  const [wasOpen, setWasOpen] = useState(props.isOpen);
  if (props.isOpen !== wasOpen) {
    setWasOpen(props.isOpen);
    if (props.isOpen) setOpening(opening + 1);
  }
  return <MarksForm key={opening} {...props} />;
}

/** The names of the tabs. */
const TAB_NAMES: Record<MarkTab, "pagemarks.tab.watermark" | "pagemarks.tab.headerFooter" | "pagemarks.tab.bates"> = {
  watermark: "pagemarks.tab.watermark",
  headerFooter: "pagemarks.tab.headerFooter",
  bates: "pagemarks.tab.bates",
};

/** The picture a watermark's file gives, once the browser has read it. */
function useImage(image: MarkImage | undefined): CanvasImageSource | undefined {
  const [element, setElement] = useState<{ url: string; image: HTMLImageElement } | undefined>(undefined);
  useEffect(() => {
    if (image === undefined) return;
    const next = new Image();
    let current = true;
    next.onload = () => current && setElement({ url: image.url, image: next });
    next.src = image.url;
    return () => {
      current = false;
    };
  }, [image]);
  return image !== undefined && element?.url === image.url ? element.image : undefined;
}

/** The text of the tab being filled in, for the font check. */
function fontOf(form: FormState): string {
  return form.tab === "watermark"
    ? form.watermark.fontId
    : form.tab === "bates"
      ? form.bates.fontId
      : form.headerFooter.fontId;
}

/** What the form works out from its values and the document: the pages, the page the preview draws and what it overlaps. */
function useDerived(props: PageMarksDialogProps, form: FormState) {
  const { start, labels } = props;
  const { tab } = form;
  const read = pagesOf(form, labels, start);
  const date = formatDate(props.today, form.headerFooter.dateFormat);
  const missing = props.lacks(fontOf(form), textOfTab(tab, form[tab], date, props.fileName));
  const lostPicture = start.mode === "edit" && start.edit.tab === "watermark" && start.edit.values.source === "image";
  const imageLost = lostPicture && form.image === undefined;
  const size = props.sizeOf(form.previewPage);
  const page: PreviewPage = {
    size,
    tokens: {
      page: form.previewPage + 1,
      pages: labels.length,
      label: labels[form.previewPage] ?? "",
      date,
      file: props.fileName,
    },
    ordinal: Math.max(read.pages.indexOf(form.previewPage), 0),
    css: props.css(fontOf(form)),
  };
  const lines = tab === "watermark" ? [] : slotLines(tab, tab === "bates" ? form.bates : form.headerFooter, page);
  const text = useLoaded(props.isOpen && tab !== "watermark", `${form.previewPage}`, () =>
    props.existing(form.previewPage),
  );
  const boxes = lines.map((l) => ({ slot: l.slot, box: l.box }));
  const overlap = text.state === "ready" ? overlapOf(boxes, bandTexts(text.value, size)) : undefined;
  return { read, date, missing, imageLost, size, page, lines, overlap };
}

/** The dialog with its choices, from what it opened on. */
function MarksForm(props: PageMarksDialogProps): ReactNode {
  const { start, labels, isOpen } = props;
  const editing = start.mode === "edit";
  const [form, setForm] = useState(() => initialForm(start, 0));
  const change = (patch: Partial<FormState>) => setForm((was) => ({ ...was, ...patch }));
  const { tab } = form;
  const derived = useDerived(props, form);
  const { read } = derived;
  const waiting = waitingFor(form, read.pages, read.error, derived.missing, derived.imageLost);
  const working = props.working !== undefined;
  const picture = usePicture(props, form.previewPage, isOpen);
  const element = useImage(form.image);
  const apply = () => {
    const pages = read.pages;
    if (tab === "watermark") props.onApply({ tab, values: form.watermark, image: form.image, pages });
    else if (tab === "bates") props.onApply({ tab, values: form.bates, pages });
    else props.onApply({ tab, values: form.headerFooter, pages, date: derived.date });
  };
  const close = () => {
    if (!working) props.onClose();
  };
  const step = (delta: number) =>
    change({ previewPage: Math.min(Math.max(form.previewPage + delta, 0), labels.length - 1) });
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) close();
      }}
      title={laterMessage(editing ? `pagemarks.edit.${tab}` : "pagemarks.title")}
      description={laterMessage("pagemarks.description")}
      closeLabel={laterMessage("pagemarks.close")}
      size="wide"
      actions={
        <Actions
          props={props}
          tab={tab}
          waiting={waiting}
          summary={summaryOf(tab, read.pages, labels, editing)}
          onApply={apply}
          onCancel={close}
        />
      }
    >
      <div className="paper-marks">
        <div className="paper-marks-form">
          <Tabs
            dialog={props}
            form={form}
            change={change}
            missing={derived.missing}
            overlap={derived.overlap}
            imageLost={derived.imageLost}
          />
          {editing ? null : (
            <PagesField
              value={form.pages}
              onChange={(pages) => change({ pages })}
              total={labels.length}
              error={read.error}
            />
          )}
        </div>
        <Side dialog={props} form={form} derived={derived} picture={picture} element={element} onStep={step} />
      </div>
    </Dialog>
  );
}

/** Draws the mark of the tab on over the page's picture. */
function drawMark(
  ctx: CanvasRenderingContext2D,
  pixel: number,
  draw: { form: FormState; page: PreviewPage; lines: SlotLine[]; element: CanvasImageSource | undefined },
): void {
  const { form, page, lines, element } = draw;
  if (form.tab === "watermark") {
    const image =
      element === undefined || form.image === undefined
        ? undefined
        : { element, width: form.image.width, height: form.image.height };
    drawWatermark(ctx, page.size, { values: form.watermark, css: page.css, image });
    return;
  }
  const look = { css: page.css, ink: "rgb(38 36 33)", edge: "rgb(110 106 100)", pixel };
  drawSlots(ctx, lines, { ...look, size: form.tab === "bates" ? form.bates.size : form.headerFooter.size });
}

/** The footer: what the primary button does or waits for, Remove when editing, Cancel and the primary. */
function Actions({
  props,
  tab,
  waiting,
  summary,
  onApply,
  onCancel,
}: {
  props: PageMarksDialogProps;
  tab: MarkTab;
  waiting: string | undefined;
  summary: string;
  onApply: () => void;
  onCancel: () => void;
}): ReactNode {
  const editing = props.start.mode === "edit";
  const working = props.working !== undefined;
  return (
    <>
      <Footer
        working={props.working}
        signed={props.signed}
        words={waiting ?? summary}
        waiting={waiting !== undefined}
      />
      {editing && props.onRemove !== undefined ? (
        <Button variant="ghost" isDisabled={working} onPress={props.onRemove}>
          {laterMessage(`pagemarks.remove.${tab}`)}
        </Button>
      ) : null}
      <Button variant="secondary" isDisabled={working} onPress={onCancel}>
        {laterMessage("pagemarks.cancel")}
      </Button>
      <Button variant="primary" isDisabled={waiting !== undefined || working} onPress={onApply}>
        {laterMessage(`pagemarks.${editing ? "update" : "add"}.${tab}`)}
      </Button>
    </>
  );
}

/** The page's picture, read again for each page the preview steps to. */
function usePicture(props: PageMarksDialogProps, page: number, isOpen: boolean): PreviewPicture {
  const loaded = useLoaded(isOpen, `${page}`, async () => props.picture(page));
  if (loaded.state === "loading") return "reading";
  if (loaded.state === "failed" || loaded.value === undefined) return "failed";
  return loaded.value;
}

/** The three tabs, each with its form. */
function Tabs({
  dialog,
  form,
  change,
  missing,
  overlap,
  imageLost,
}: {
  dialog: PageMarksDialogProps;
  form: FormState;
  change: (patch: Partial<FormState>) => void;
  missing: readonly string[];
  overlap: Overlap | undefined;
  imageLost: boolean;
}): ReactNode {
  const editing = dialog.start.mode === "edit";
  const patch =
    <K extends "watermark" | "headerFooter" | "bates">(key: K) =>
    (next: Partial<FormState[K]>) =>
      change({ [key]: { ...form[key], ...next } } as Partial<FormState>);
  const pick = () => void dialog.pickImage().then((image) => image !== undefined && change({ image }));
  const last = Math.max(dialog.labels.length - 1, 0);
  const range = laterMessage("pagemarks.bates.range", {
    first: batesText(form.bates, form.bates.start),
    last: batesText(form.bates, form.bates.start + last),
  });
  const shown = (id: MarkTab) => (form.tab === id ? missing : []);
  return (
    <SegmentedTabs
      tabs={MARK_TABS.filter((id) => !editing || id === form.tab).map((id) => ({
        id,
        label: laterMessage(TAB_NAMES[id]),
      }))}
      selectedKey={form.tab}
      onSelectionChange={(id) => change({ tab: id })}
      aria-label={laterMessage("pagemarks.tabs")}
    >
      <TabPanel id="watermark">
        <WatermarkTab
          values={form.watermark}
          onChange={patch("watermark")}
          fonts={dialog.fonts}
          missing={shown("watermark")}
          canHide={dialog.canHide}
          image={form.image}
          onPickImage={pick}
          editing={editing}
          imageLost={imageLost}
        />
      </TabPanel>
      <TabPanel id="headerFooter">
        <HeaderFooterTab
          values={form.headerFooter}
          onChange={patch("headerFooter")}
          fonts={dialog.fonts}
          missing={shown("headerFooter")}
          today={dialog.today}
          overlapping={overlap?.slot}
        />
      </TabPanel>
      <TabPanel id="bates">
        <BatesTab
          values={form.bates}
          onChange={patch("bates")}
          fonts={dialog.fonts}
          missing={shown("bates")}
          range={range}
          overlapping={overlap?.slot === form.bates.slot}
        />
      </TabPanel>
    </SegmentedTabs>
  );
}

/** The warning above the preview: text the new mark would run into. */
function OverlapNotice({ overlap, page }: { overlap: Overlap; page: number }): ReactNode {
  const slot = laterMessage(`pagemarks.slot.${overlap.slot}`).toLocaleLowerCase();
  const band = overlap.slot.startsWith("top") ? "header" : "footer";
  return (
    <Notice
      tone="warning"
      title={laterMessage(overlap.pageNumber ? "pagemarks.overlap.number" : "pagemarks.overlap.text", {
        page,
        text: overlap.text,
      })}
    >
      {laterMessage(overlap.pageNumber ? "pagemarks.overlap.number.fix" : "pagemarks.overlap.text.fix", {
        slot,
        band: laterMessage(`pagemarks.band.${band}`),
      })}
    </Notice>
  );
}

/** The footer's words: why the primary waits, else what it does; the progress while it works; the certification's warning. */
function Footer({
  working,
  signed,
  words,
  waiting,
}: {
  working: PageMarksDialogProps["working"];
  signed: PageMarksDialogProps["signed"];
  words: string;
  waiting: boolean;
}): ReactNode {
  return (
    <div className="paper-marks-footer">
      {working === undefined ? (
        <p className="paper-marks-summary" data-waiting={waiting ? true : undefined}>
          {words}
        </p>
      ) : (
        <ProgressBar
          label={laterMessage("pagemarks.working")}
          {...(working.value === undefined ? {} : { value: working.value })}
        />
      )}
      {signed === undefined || signed.length === 0 ? null : (
        <p className="paper-marks-signed" role="status">
          <span>{laterMessage("pagemarks.signed")}</span>
          <SignatureDetails restrictions={signed} />
        </p>
      )}
    </div>
  );
}

/** The column beside the form: the overlap notice, then the preview. */
function Side({
  dialog,
  form,
  derived,
  picture,
  element,
  onStep,
}: {
  dialog: PageMarksDialogProps;
  form: FormState;
  derived: ReturnType<typeof useDerived>;
  picture: PreviewPicture;
  element: CanvasImageSource | undefined;
  onStep: (delta: number) => void;
}): ReactNode {
  const { size, page, lines, overlap } = derived;
  const label = dialog.labels[form.previewPage] ?? "";
  return (
    <div className="paper-marks-side">
      {overlap === undefined ? null : <OverlapNotice overlap={overlap} page={form.previewPage + 1} />}
      <MarkPreview
        size={size}
        picture={picture}
        label={label}
        index={form.previewPage}
        count={dialog.labels.length}
        onStep={onStep}
        name={laterMessage(`pagemarks.preview.name.${form.tab}`, { page: label })}
        draw={(ctx, pixel) => drawMark(ctx, pixel, { form, page, lines, element })}
      />
    </div>
  );
}
