// The Page marks dialog's state over one open document (Paper A03): which dialog is open and on what,
// the marks Manage page marks lists, and each tab's primary button as one op and one undo step. The
// dialog asks; this reads the pages' pictures and text, finds the fonts, makes the ops and says
// what happened. Nothing here reads a clock but the ops' dates.

import type { PageMarkInfo } from "@laminapdf/sdk";
import type { Platform, ToastQueue } from "@laminapdf/ui";
import { formatBytes } from "@laminapdf/ui";
import type { SignatureRestriction } from "@laminapdf/ui/editing";
import {
  laterMessage,
  type ManagedMark,
  type MarkChoice,
  type MarkFont,
  type MarkImage,
  type MarkStart,
  type MarkTab,
  type PageText,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import type { ContentController } from "../content/controller.ts";
import { readImage } from "../content/images.ts";
import { cssFamily } from "../content/text-look.ts";
import { fallbackChain } from "../fonts/fallback.ts";
import { pickedFile } from "../picked-file.ts";
import type { Engine } from "../session.ts";
import { editOf, groupMarks, kindOf, type MarkGroup, managedOf } from "./groups.ts";
import {
  addMarkOp,
  batesSpec,
  deleteMarkOp,
  headerFooterSpec,
  type MarkSpec,
  newMarkId,
  updateMarkOp,
  watermarkSpec,
} from "./ops.ts";

/** The dialog open on a mark: what it shows, and what the file says of saving it. */
export interface OpenMarks {
  readonly start: MarkStart;
  /** The mark being edited. */
  readonly id: string | undefined;
  /** The restrictions of a signature the mark would break; none for a file that saves plainly. */
  readonly signed: readonly SignatureRestriction[];
  /** False on a PDF/A-1 file, which can't hold an optional content group. */
  readonly canHide: boolean;
}

/** What the controller shows. */
export interface PageMarksState {
  readonly dialog: OpenMarks | undefined;
  /** Manage page marks: its marks, or why it has none to show yet; undefined while closed. */
  readonly manage: readonly ManagedMark[] | "reading" | "failed" | undefined;
  /** The mark being added: how far along when that is known. */
  readonly working: { readonly value?: number } | undefined;
}

/** What the controller needs of the app. */
export interface PageMarksHost {
  readonly platform: Platform;
  readonly toasts: ToastQueue;
}

/** The code of `error`, "" when it has none. */
function reasonOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** A mark's pages as a person says them, for the toast. */
function whereOf(count: number, total: number): string {
  if (count === total)
    return laterMessage(total === 1 ? "pagemarks.summary.allOne" : "pagemarks.summary.all", { count });
  return laterMessage(count === 1 ? "pagemarks.summary.someOne" : "pagemarks.summary.some", {
    count,
    pages: "",
  }).trim();
}

/** Page marks over one open document. */
export class PageMarksController {
  readonly #engine: Engine;
  readonly #viewer: Viewer;
  readonly #content: ContentController;
  readonly #host: PageMarksHost;
  readonly #listeners = new Set<() => void>();
  /** The pictures chosen for a watermark in this session, by their object URL, and the blobs that hold their bytes. */
  readonly #pictures = new Map<string, { readonly blob: string; readonly image: MarkImage }>();
  #groups: readonly MarkGroup[] = [];
  #state: PageMarksState = { dialog: undefined, manage: undefined, working: undefined };

  constructor(engine: Engine, viewer: Viewer, content: ContentController, host: PageMarksHost) {
    this.#engine = engine;
    this.#viewer = viewer;
    this.#content = content;
    this.#host = host;
  }

  getState = (): PageMarksState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  #set(change: Partial<PageMarksState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }

  // What the dialog reads.

  /** Each page's label, in page order. */
  labels(): readonly string[] {
    return this.#engine.getState().pages.map((page, i) => page.label ?? String(i + 1));
  }

  /** The open document's name, which `[File name]` stands for. */
  fileName(): string {
    return this.#engine.getState().title ?? "";
  }

  /** Page `page`'s size in points. */
  sizeOf(page: number) {
    return this.#content.sizeOf(page) ?? { width: 612, height: 792 };
  }

  /** The fonts the selects offer, naming what a bundled font's download costs. */
  fonts(): readonly MarkFont[] {
    const registry = this.#content.registry;
    return this.#content.fonts().map((font) => {
      const pending = registry.toFetch([font.id]).length > 0;
      const costly = pending && font.bytes !== undefined && font.bytes > 100_000;
      const note = costly ? ` · ${laterMessage("content.font.download", { size: formatBytes(font.bytes ?? 0) })}` : "";
      return { id: font.id, label: `${font.family}${note}` };
    });
  }

  /** The characters of `text` no font would draw when it is set in `fontId`. */
  lacks(fontId: string, text: string): readonly string[] {
    return fallbackChain(this.#content.fonts(), fontId, text).missing;
  }

  /** The browser's font stack for `fontId`. */
  css = (fontId: string): string => cssFamily(fontId);

  /** Page `page` drawn as it is now, as wide as the preview needs at twice its pixels. */
  async picture(page: number): Promise<CanvasImageSource | undefined> {
    const size = this.sizeOf(page);
    const scale = Math.min(2, 560 / Math.max(size.width, 1));
    const drawn = await this.#engine.thumbnail(page, scale, new AbortController().signal);
    return drawn.bitmap;
  }

  /** The text page `page` shows now, with its boxes. */
  async existing(page: number): Promise<readonly PageText[]> {
    const runs = await this.#engine.textRuns(page);
    return runs.map((run) => ({ text: run.text, bounds: run.bounds as PageText["bounds"] }));
  }

  /** Asks for a JPEG or PNG for a watermark and adds its bytes to the document. */
  async pickImage(): Promise<MarkImage | undefined> {
    const [picked] = await this.#host.platform.openFiles({ multiple: false, accept: "images" });
    if (picked === undefined) return undefined;
    const file = await pickedFile(picked);
    const image = await readImage(file);
    if (image === undefined) {
      this.#host.toasts.add({ message: laterMessage("content.unsupported.body", { file: file.name }) });
      return undefined;
    }
    const url = URL.createObjectURL(new Blob([new Uint8Array(image.bytes)], { type: image.type }));
    const shown = { name: image.name, url, width: image.width, height: image.height };
    this.#pictures.set(url, { blob: await this.#engine.addBlob(image.bytes), image: shown });
    return shown;
  }

  // Opening and closing.

  /** Opens the dialog to add a mark on `tab`. */
  openAdd(tab: MarkTab): void {
    this.#open({ start: { mode: "add", tab }, id: undefined, signed: [], canHide: true });
  }

  #open(dialog: OpenMarks): void {
    this.#set({ dialog, manage: undefined, working: undefined });
    void this.#probe(dialog);
  }

  /** Reads what the file says of saving a mark: a signature it would break, and whether it holds optional content. */
  async #probe(dialog: OpenMarks): Promise<void> {
    const first = this.#engine.getState().pages[0]?.id;
    try {
      const { client } = await this.#engine.view();
      const [violations, metadata] = await Promise.all([
        first === undefined ? [] : client.wouldViolate([probeOp(first)]),
        client.metadata(),
      ]);
      const signed = violations.map((v) => ({
        restriction: v.restriction as SignatureRestriction["restriction"],
        ...(v.permission === undefined ? {} : { permission: v.permission }),
        ...(v.signature.revision === undefined ? {} : { revision: v.signature.revision }),
      }));
      if (this.#state.dialog === dialog) {
        this.#set({ dialog: { ...dialog, signed, canHide: !(metadata.pdfa ?? "").startsWith("1") } });
      }
    } catch {
      // The notes are an extra: without them the dialog still adds the mark.
    }
  }

  /** Closes the dialog and Manage page marks. The pictures chosen stay, so a mark that holds one can be edited again in this session. */
  close(): void {
    this.#set({ dialog: undefined, manage: undefined, working: undefined });
  }

  /** Opens Manage page marks, reading the marks the file holds. */
  async openManage(): Promise<void> {
    this.#set({ dialog: undefined, manage: "reading" });
    try {
      const marks: PageMarkInfo[][] = await this.#engine.pageMarks();
      this.#groups = groupMarks(marks);
      const labels = this.labels();
      if (this.#state.manage === "reading") this.#set({ manage: this.#groups.map((g) => managedOf(g, labels)) });
    } catch (error) {
      console.error("Reading the page marks failed", error);
      if (this.#state.manage === "reading") this.#set({ manage: "failed" });
    }
  }

  /** Opens the dialog on the mark `id` that Manage page marks listed. */
  edit(id: string): void {
    const group = this.#groups.find((g) => g.id === id);
    if (group === undefined) return;
    const fonts = this.#content.documentFonts;
    const edit = editOf(group, {
      fontIdOf: (name) => fonts.idOf(name),
      imageOf: (blob) => this.#imageOf(blob),
    });
    this.#open({ start: { mode: "edit", edit }, id, signed: [], canHide: true });
  }

  /** The picture chosen in this session that `blob` names. */
  #imageOf(blob: string): MarkImage | undefined {
    return [...this.#pictures.values()].find((picture) => picture.blob === blob)?.image;
  }

  // Making the ops.

  /** The font names for `text` set in `fontId`, fetching and adding what the document doesn't have. */
  async #fontsFor(fontId: string, text: string): Promise<readonly string[] | undefined> {
    const plan = await this.#content.documentFonts.forText(fontId, text);
    if (plan.missing.length > 0) {
      this.#host.toasts.add({ message: laterMessage("content.font.missing", { characters: plan.missing.join(" ") }) });
      return undefined;
    }
    return plan.fonts;
  }

  /** The mark `choice` makes, with the fonts it needs. */
  async #specOf(choice: MarkChoice): Promise<MarkSpec | undefined> {
    if (choice.tab === "watermark") {
      const { values } = choice;
      const blob = choice.image === undefined ? undefined : this.#pictures.get(choice.image.url)?.blob;
      if (values.source === "image") return watermarkSpec(values, [], blob);
      const fonts = await this.#fontsFor(values.fontId, values.text);
      return fonts && watermarkSpec(values, fonts, undefined);
    }
    if (choice.tab === "bates") {
      const text = `${choice.values.prefix}0123456789${choice.values.suffix}`;
      const fonts = await this.#fontsFor(choice.values.fontId, text);
      return fonts && batesSpec(choice.values, fonts);
    }
    const text = `${choice.values.slots.join("\n")}\n${choice.date}${this.fileName()}0123456789`;
    const fonts = await this.#fontsFor(choice.values.fontId, text);
    return fonts && headerFooterSpec(choice.values, fonts, choice.date, this.fileName());
  }

  /** Adds the mark `choice` names, or updates the one being edited, as one undo step. */
  async apply(choice: MarkChoice): Promise<void> {
    const open = this.#state.dialog;
    if (open === undefined) return;
    this.#set({ working: {} });
    try {
      const mark = await this.#specOf(choice);
      if (mark !== undefined) await this.#make(choice, mark, open);
    } catch (error) {
      this.#host.toasts.add({ message: laterMessage("pagemarks.failed", { reason: reasonOf(error) }) });
    }
    if (this.#state.dialog === open) this.#set({ working: undefined });
  }

  async #make(choice: MarkChoice, mark: MarkSpec, open: OpenMarks): Promise<void> {
    const pages = choice.pages.map((i) => this.#content.pageIdOf(i));
    const kind = laterMessage(`pagemarks.kind.${mark.kind}`).toLocaleLowerCase();
    const editing = open.id !== undefined;
    const op = editing ? updateMarkOp(pages, open.id ?? "", mark) : addMarkOp(pages, newMarkId(), mark);
    await this.#viewer.edits.apply(
      [op],
      laterMessage(editing ? "pagemarks.step.update" : "pagemarks.step.add", { kind }),
    );
    this.close();
    this.#toast(
      editing
        ? laterMessage("pagemarks.toast.updated")
        : laterMessage(`pagemarks.toast.added.${choice.tab}`, { where: whereOf(pages.length, this.labels().length) }),
    );
  }

  /** Takes the mark `id` off every page it is on, as one undo step. */
  async remove(id: string): Promise<void> {
    const group = this.#groups.find((g) => g.id === id);
    if (group === undefined) return;
    const kind = laterMessage(`pagemarks.kind.${kindOf(group.first)}`).toLocaleLowerCase();
    const op = deleteMarkOp(
      group.pages.map((i) => this.#content.pageIdOf(i)),
      id,
    );
    try {
      await this.#viewer.edits.apply([op], laterMessage("pagemarks.step.remove", { kind }));
    } catch (error) {
      this.#host.toasts.add({ message: laterMessage("pagemarks.failed", { reason: reasonOf(error) }) });
      return;
    }
    this.close();
    this.#toast(laterMessage("pagemarks.toast.removed"));
  }

  #toast(message: string): void {
    this.#host.toasts.add({
      message,
      action: { label: laterMessage("pagemarks.toast.undo"), onPress: () => void this.#viewer.edits.undo() },
    });
  }
}

/** A watermark on one page: the probe `wouldViolate` classifies, to learn what saving a mark would break. */
function probeOp(page: string) {
  return addMarkOp([page], "probe", { kind: "watermark", spec: { mark: { text: { text: "x", size: 12 } } } });
}
