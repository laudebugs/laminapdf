// The Page marks dialog's form as state: each tab's values, the pages, the picture, which page the
// preview shows, and what makes the primary button wait. Apart from the DOM so Node can test it.

import { laterMessage } from "../later-messages.js";
import {
  BATES_DEFAULT,
  type BatesValues,
  HEADER_FOOTER_DEFAULT,
  type HeaderFooterValues,
  hasSlotText,
  type MarkTab,
  type PagesChoice,
  readPages,
  WATERMARK_DEFAULT,
  type WatermarkValues,
} from "./model.js";
import type { MarkImage } from "./preview-draw.js";

/** What the dialog opens on. */
export type MarkStart =
  | { readonly mode: "add"; readonly tab: MarkTab }
  | { readonly mode: "edit"; readonly edit: MarkEdit };

/** A mark being changed: its kind with its values and the pages it is on. */
export type MarkEdit =
  | {
      readonly tab: "watermark";
      readonly values: WatermarkValues;
      readonly pages: readonly number[];
      /** The picture, when this session still has it. */
      readonly image?: MarkImage | undefined;
    }
  | { readonly tab: "headerFooter"; readonly values: HeaderFooterValues; readonly pages: readonly number[] }
  | { readonly tab: "bates"; readonly values: BatesValues; readonly pages: readonly number[] };

/** The values of every tab and the choices around them. */
export interface FormState {
  readonly tab: MarkTab;
  readonly watermark: WatermarkValues;
  readonly headerFooter: HeaderFooterValues;
  readonly bates: BatesValues;
  readonly pages: PagesChoice;
  readonly image: MarkImage | undefined;
  /** The page the preview shows, 0-based. */
  readonly previewPage: number;
}

/** The form a dialog opens with. */
export function initialForm(start: MarkStart, firstPage: number): FormState {
  const base: FormState = {
    tab: start.mode === "add" ? start.tab : start.edit.tab,
    watermark: WATERMARK_DEFAULT,
    headerFooter: HEADER_FOOTER_DEFAULT,
    bates: BATES_DEFAULT,
    pages: { all: true, range: "" },
    image: undefined,
    previewPage: firstPage,
  };
  if (start.mode === "add") return base;
  const { edit } = start;
  const own = { previewPage: edit.pages[0] ?? firstPage };
  if (edit.tab === "watermark") return { ...base, ...own, watermark: edit.values, image: edit.image };
  return { ...base, ...own, [edit.tab]: edit.values };
}

/** The fields' validity, one flag each. */
function numbersOk(form: FormState): boolean {
  if (form.tab === "watermark") {
    const w = form.watermark;
    return Number.isFinite(w.opacity) && Number.isFinite(w.angle) && (w.source === "image" || Number.isFinite(w.size));
  }
  if (form.tab === "headerFooter") {
    const h = form.headerFooter;
    return [h.size, h.top, h.bottom].every(Number.isFinite);
  }
  return [form.bates.start, form.bates.digits, form.bates.size].every(Number.isFinite);
}

/** Why a watermark can't be added yet, or undefined. */
function watermarkWaits(form: FormState, imageLost: boolean): string | undefined {
  const { source, text } = form.watermark;
  if (source === "text" && text.trim() === "") return laterMessage("pagemarks.waiting.text");
  if (source === "image" && form.image === undefined) {
    return laterMessage(imageLost ? "pagemarks.image.lost" : "pagemarks.waiting.image");
  }
  return undefined;
}

/** Why the primary button waits (the footer says it), or undefined when it can run. */
export function waitingFor(
  form: FormState,
  pages: readonly number[],
  pagesError: string | undefined,
  missing: readonly string[],
  imageLost: boolean,
): string | undefined {
  if (pagesError !== undefined) return pagesError;
  if (pages.length === 0) return laterMessage("pagemarks.waiting.pages");
  const own =
    form.tab === "watermark"
      ? watermarkWaits(form, imageLost)
      : form.tab === "headerFooter" && !hasSlotText(form.headerFooter)
        ? laterMessage("pagemarks.waiting.slots")
        : undefined;
  if (own !== undefined) return own;
  if (!numbersOk(form)) return laterMessage("pagemarks.waiting.numbers");
  return missing.length > 0 ? laterMessage("pagemarks.waiting.font") : undefined;
}

/** The pages the form names, in a document labelled `labels`; the pages being edited when editing. */
export function pagesOf(
  form: FormState,
  labels: readonly string[],
  start: MarkStart,
): { pages: readonly number[]; error?: string } {
  return start.mode === "edit" ? { pages: start.edit.pages } : readPages(form.pages, labels);
}
