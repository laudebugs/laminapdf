// Comment mode's stamps and attached files (Paper C05). A stamp chosen in the picker rides the
// pointer until a click places it, as one `addAnnot`: a standard stamp, a dynamic one whose name
// and time are written when it is placed, or a custom image from the library kept on this device
// (stamp-library.ts), whose bytes the document takes once. Attach file asks for a file where the
// page was clicked and attaches it under an icon. A selected attachment's file saves out as it
// was attached, after O14's warning for a program; Replace…, its description and its icon are each
// one edit. A sound comment plays its recording, which the worker turns into WAV.

import type { Op, PageAnnotation } from "@laminapdf/sdk";
import { formatBytes, message, type Platform, type ToastQueue } from "@laminapdf/ui";
import { isProgram, mimeOf } from "@laminapdf/ui/dialogs";
import type { AttachmentIconName, CustomStampItem, StampChoice } from "@laminapdf/ui/editing";
import { laterMessage } from "@laminapdf/ui/editing";
import type { PageFrame } from "@laminapdf/viewer";
import { pageToUser } from "@laminapdf/viewer";
import type { Engine } from "../session.ts";
import type { Point } from "./marks.ts";
import { kindName, updateAnnotOp } from "./ops.ts";
import { indexedDbStore, StampLibrary } from "./stamp-library.ts";
import {
  attachFileOp,
  dynamicLines,
  type FileToAttach,
  type PlaceContext,
  placeStampOp,
  type ReadyStamp,
  replaceFileOps,
  stampSize,
  stampTime,
} from "./stamp-ops.ts";

/** What stamps and files need of Comment mode. */
export interface FilesHost {
  readonly engine: Engine;
  readonly platform: Platform;
  readonly toasts: ToastQueue;
  frame(page: number): Promise<PageFrame | undefined>;
  nameAsked(): Promise<boolean>;
  author(): string | undefined;
  /** Applies `ops` on `page` as one step through the page's gesture; the IDs it made. */
  commit(page: number, ops: Op[], label: string): Promise<readonly string[]>;
  /** Selects annotation `id` on `page`. */
  select(page: number, id: string): void;
  /** What the mode shows changed: the picker's stamps, the stamp chosen, a notice, Play. */
  changed(): void;
  /** Asks whether to save a program out (O14's warning). */
  confirmProgram(name: string): Promise<boolean>;
  /** The colour a new attachment's icon takes, from the Attach file tool's style. */
  attachColour(): readonly number[];
}

/** What stamps and files show now. */
export interface FilesState {
  readonly custom: readonly CustomStampItem[];
  /** The stamp riding the pointer, chosen in the picker. */
  readonly chosen: StampChoice | undefined;
  /** The sound comment playing. */
  readonly playing: string | undefined;
}

/** The common entries an attached file's annotation keeps when its file is replaced. */
type Look = Parameters<typeof replaceFileOps>[4];

/** The icons by `/Name`, as the inspector names them. */
const ICONS: Readonly<Record<string, AttachmentIconName>> = {
  Paperclip: "paperclip",
  PushPin: "pushPin",
  Graph: "graph",
  Tag: "tag",
};

/** An annotation's attached-file details. */
function fileOf(a: PageAnnotation) {
  return a.details?.kind === "fileAttachment" ? a.details : undefined;
}

/** An attachment's icon as the inspector names it; a name another app wrote shows as Push pin, as readers draw it. */
export function iconOf(a: PageAnnotation): AttachmentIconName {
  return ICONS[fileOf(a)?.icon ?? ""] ?? "pushPin";
}

/** Asks for one file, through a file input (`accept` its types); undefined when the person cancels. */
function chooseFile(accept: string): Promise<File | undefined> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.addEventListener("change", () => resolve(input.files?.[0]));
    input.addEventListener("cancel", () => resolve(undefined));
    input.click();
  });
}

/** The pixel size of image `bytes`, for a custom stamp's proportions. */
async function imageSize(bytes: Uint8Array, type: string): Promise<{ width: number; height: number }> {
  const bitmap = await createImageBitmap(new Blob([new Uint8Array(bytes)], { type }));
  const size = { width: bitmap.width, height: bitmap.height };
  bitmap.close();
  return size;
}

/** The stamps and attached files of one open document's Comment mode. */
export class StampsAndFiles {
  readonly #host: FilesHost;
  readonly #library = new StampLibrary(indexedDbStore());
  /** Each custom stamp's image in this document, by stamp ID: the document takes the bytes once. */
  readonly #blobs = new Map<string, Promise<{ blob: string; width: number; height: number }>>();
  #state: FilesState = { custom: [], chosen: undefined, playing: undefined };
  #urls: string[] = [];

  constructor(host: FilesHost) {
    this.#host = host;
  }

  get state(): FilesState {
    return this.#state;
  }

  /** Whether Save file as… opens a save dialog, or downloads (C05's web difference). */
  get savesWithDialog(): boolean {
    return this.#host.platform.capabilities.canSaveAsDialog;
  }

  /** Reads the custom stamps for the picker, each with a thumbnail. */
  async refresh(): Promise<void> {
    let stamps: Awaited<ReturnType<StampLibrary["list"]>>;
    try {
      stamps = await this.#library.list();
    } catch (e) {
      // IndexedDB refused (a private window's policy): the picker shows no custom stamps.
      reportError(e);
      stamps = [];
    }
    for (const url of this.#urls) URL.revokeObjectURL(url);
    const custom = stamps.map((s) => ({
      id: s.id,
      name: s.name,
      url: URL.createObjectURL(new Blob([new Uint8Array(s.bytes)], { type: s.type })),
    }));
    this.#urls = custom.map((c) => c.url);
    this.#set({ custom });
  }

  /** Lets the thumbnails go (Comment mode left). */
  leave(): void {
    for (const url of this.#urls) URL.revokeObjectURL(url);
    this.#urls = [];
    this.#set({ custom: [], chosen: undefined });
  }

  /** The stamp the picker chose rides the pointer from now on. */
  choose(choice: StampChoice): void {
    this.#set({ chosen: choice });
  }

  /** The chosen stamp's size on the page, for the preview; undefined before one is chosen. */
  size(): readonly [number, number] | undefined {
    const chosen = this.#state.chosen;
    if (chosen === undefined) return undefined;
    // An image's proportions are known once it is read: a square stands in until then.
    if (chosen.kind === "custom") return [120, 120];
    return stampSize(chosen.kind === "standard" ? chosen : { ...chosen, lines: [] });
  }

  /** Places the chosen stamp centred on `at`, on `page`, as one step; it is selected after. */
  async place(page: number, at: Point): Promise<void> {
    const chosen = this.#state.chosen;
    if (chosen === undefined || !(await this.#host.nameAsked())) return;
    const ready = await this.#ready(chosen);
    const context = await this.#context(page);
    if (ready === undefined || context === undefined) return;
    const op = placeStampOp(ready, at, context);
    const label = laterMessage("comment.step.add", { kind: kindName("Stamp").toLocaleLowerCase() });
    const [made] = await this.#host.commit(page, [op], label);
    if (made !== undefined) this.#host.select(page, made);
  }

  /** The chosen stamp with what placing it needs: a dynamic stamp's lines now, a custom stamp's image in the document. */
  async #ready(chosen: StampChoice): Promise<ReadyStamp | undefined> {
    if (chosen.kind === "standard") return chosen;
    if (chosen.kind === "dynamic") {
      return { ...chosen, lines: dynamicLines(chosen.name, this.#host.author(), new Date()) };
    }
    try {
      const image = await this.#image(chosen.id);
      return image === undefined ? undefined : { kind: "image", ...image };
    } catch (e) {
      reportError(e);
      return undefined;
    }
  }

  /** Custom stamp `id`'s image, added to the document the first time it is placed. */
  #image(id: string): Promise<{ blob: string; width: number; height: number } | undefined> {
    const known = this.#blobs.get(id);
    if (known !== undefined) return known;
    const made = (async () => {
      const stamp = await this.#library.get(id);
      if (stamp === undefined) throw new Error("the custom stamp is gone");
      const [blob, size] = await Promise.all([
        this.#host.engine.addBlob(stamp.bytes),
        imageSize(stamp.bytes, stamp.type),
      ]);
      return { blob, ...size };
    })();
    this.#blobs.set(id, made);
    made.catch(() => this.#blobs.delete(id));
    return made;
  }

  /** Add image…: a PNG or JPEG kept in the library; anything else says why it can't be a stamp. */
  async addCustom(): Promise<void> {
    const file = await chooseFile("image/png,image/jpeg");
    if (file === undefined) return;
    const added = await this.#library.add(file.name, new Uint8Array(await file.arrayBuffer()));
    if (added === undefined) {
      this.#host.toasts.add({ message: laterMessage("stamps.custom.unreadable", { name: file.name }) });
      return;
    }
    await this.refresh();
  }

  async rename(id: string, name: string): Promise<void> {
    await this.#library.rename(id, name);
    await this.refresh();
  }

  async remove(id: string): Promise<void> {
    await this.#library.remove(id);
    if (this.#state.chosen?.kind === "custom" && this.#state.chosen.id === id) this.#set({ chosen: undefined });
    await this.refresh();
  }

  /** The line a dynamic stamp writes under its title now. */
  dynamicLine(): string {
    const time = stampTime(new Date());
    const author = this.#host.author();
    return author === undefined ? time : laterMessage("stamps.dynamic.by", { name: author, time });
  }

  /** Attach file: asks for a file, then attaches it under an icon whose corner is `at`, on `page`. */
  async attachAt(page: number, at: Point): Promise<void> {
    if (!(await this.#host.nameAsked())) return;
    const file = await chooseFile("");
    if (file === undefined || !this.#fits(file)) return;
    const [context, attached] = await Promise.all([this.#context(page), this.#fileOf(file)]);
    if (context === undefined) return;
    const label = laterMessage("attachment.label.attach", { name: file.name });
    const op = attachFileOp(attached, at, context, { color: this.#host.attachColour() });
    const [made] = await this.#host.commit(page, [op], label);
    if (made !== undefined) this.#host.select(page, made);
  }

  /** Whether the document could read `file` back out (O14's memory limit); a toast says when not. */
  #fits(file: File): boolean {
    const limit = this.#host.engine.getState().attachmentLimit;
    if (limit === undefined || file.size <= limit) return true;
    this.#host.toasts.add({
      message: laterMessage("attachments.tooLarge.attach", { name: file.name, limit: formatBytes(limit) }),
    });
    return false;
  }

  /** `file`'s bytes in the document, with its name, type and date. */
  async #fileOf(file: File, description?: string): Promise<FileToAttach> {
    const blob = await this.#host.engine.addBlob(new Uint8Array(await file.arrayBuffer()));
    return {
      blob,
      name: file.name,
      mime: mimeOf(file.name, file.type),
      modified: new Date(file.lastModified),
      ...(description === undefined ? {} : { description }),
    };
  }

  /** Save file as… (a download where the browser has no save dialog), after O14's warning for a program. */
  async save(a: PageAnnotation): Promise<void> {
    const details = fileOf(a);
    if (details?.file === undefined) return;
    const name = details.name ?? "attachment";
    // The document's attachment list holds the file's record, which says whether it is a program.
    const record = this.#host.engine.getState().attachments.find((f) => f.id === details.file);
    if (record !== undefined && isProgram(record) && !(await this.#host.confirmProgram(name))) return;
    const read = await this.#host.engine.attachmentBytes(details.file);
    const saveExport = this.#host.platform.saveExport;
    if (saveExport === undefined) return;
    const blob = new Blob([new Uint8Array(read.bytes)], details.mime === undefined ? {} : { type: details.mime });
    const result = await saveExport.call(this.#host.platform, blob, name);
    if (result.saved) this.#host.toasts.add({ message: message("download.done", { name: result.name }) });
  }

  /** Replace…: another file in the same place, with the same icon, colour and description, as one step. */
  async replace(page: number, a: PageAnnotation): Promise<void> {
    const file = await chooseFile("");
    if (file === undefined || !this.#fits(file) || a.rect === undefined) return;
    const details = fileOf(a);
    const [context, attached] = await Promise.all([this.#context(page), this.#fileOf(file, details?.description)]);
    if (context === undefined) return;
    const look: Look = { icon: iconOf(a), ...(a.color.length === 0 ? {} : { color: a.color }) };
    const at: Point = [a.rect[0], a.rect[1]];
    const label = laterMessage("attachment.label.replace", { name: details?.name ?? file.name });
    const ops = replaceFileOps(a.id, attached, at, context, look);
    const made = (await this.#host.commit(page, ops, label)).at(-1);
    if (made !== undefined) this.#host.select(page, made);
  }

  /** The attached file's description, as one step; empty removes it. */
  async describe(page: number, a: PageAnnotation, text: string): Promise<void> {
    const details = fileOf(a);
    if (details?.file === undefined) return;
    const op: Op = {
      op: "updateAttachment",
      args: { attachment: details.file, description: text === "" ? null : text },
    };
    await this.#host.commit(page, [op], laterMessage("attachment.label.describe", { name: details.name ?? "" }));
  }

  /** The attachment's icon, as one step. */
  async setIcon(page: number, a: PageAnnotation, icon: AttachmentIconName): Promise<void> {
    const op = updateAnnotOp(a.id, { spec: { type: "fileAttachment", icon } });
    const label = laterMessage("comment.step.change", { kind: kindName(a.subtype).toLocaleLowerCase() });
    await this.#host.commit(page, [op], label);
  }

  /** Plays sound comment `a`, its samples turned into WAV by the worker. */
  async play(a: PageAnnotation): Promise<void> {
    if (this.#state.playing !== undefined) return;
    this.#set({ playing: a.id });
    let url: string | undefined;
    try {
      const { client } = await this.#host.engine.view();
      const wav = await client.soundAsWav(a.id);
      url = URL.createObjectURL(new Blob([new Uint8Array(wav)], { type: "audio/wav" }));
      const audio = new Audio(url);
      await new Promise<void>((resolve, reject) => {
        audio.addEventListener("ended", () => resolve(), { once: true });
        audio.addEventListener("error", () => reject(audio.error), { once: true });
        audio.play().catch(reject);
      });
    } catch (e) {
      this.#host.toasts.add({
        message: laterMessage("sound.unplayable", { reason: String((e as Error)?.message ?? e) }),
      });
    } finally {
      if (url !== undefined) URL.revokeObjectURL(url);
      this.#set({ playing: undefined });
    }
  }

  /** Where an edit on `page` goes: its ID, its frame's mapping and turn, the author and now. */
  async #context(page: number): Promise<PlaceContext | undefined> {
    const frame = await this.#host.frame(page);
    const pageId = this.#host.engine.getState().pages[page]?.id;
    if (frame === undefined || pageId === undefined) return undefined;
    return {
      pageId,
      toUser: (point) => pageToUser(frame, point),
      rotation: frame.rotate,
      author: this.#host.author(),
      created: new Date(),
    };
  }

  #set(change: Partial<FilesState>): void {
    this.#state = { ...this.#state, ...change };
    this.#host.changed();
  }
}
