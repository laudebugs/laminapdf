// Comment mode's stamps and attached files (Paper C05). A stamp chosen in the picker is placed
// where the page is pressed and released, its box drawn under the pointer meanwhile, as one
// `addAnnot`: a standard stamp, a dynamic one whose name and time are written when it is placed,
// or a custom image from the library kept on this device (stamp-library.ts), whose bytes each
// document takes once. Attach file asks for a file, then attaches it under an icon centred where
// the page was clicked. A selected attachment's file saves out as it was attached, after O14's
// warning for a program; Replace… puts a new file in the same annotation, which keeps its replies,
// note and author; its description and its icon are each one edit. A sound comment plays its
// recording, which the worker turns into WAV.

import type { Op, PageAnnotation } from "@laminapdf/sdk";
import { formatBytes, message, type Platform, type ToastQueue } from "@laminapdf/ui";
import { isProgramFile, mimeOf } from "@laminapdf/ui/dialogs";
import type { AttachmentIconName, CustomStampItem, StampChoice } from "@laminapdf/ui/editing";
import { laterMessage } from "@laminapdf/ui/editing";
import type { PageFrame } from "@laminapdf/viewer";
import { pageToUser } from "@laminapdf/viewer";
import { pickedFile } from "../picked-file.ts";
import type { Engine } from "../session.ts";
import type { Point } from "./marks.ts";
import { kindName, updateAnnotOp } from "./ops.ts";
import { indexedDbStore, StampLibrary } from "./stamp-library.ts";
import {
  attachFileOp,
  dynamicLines,
  type FileToAttach,
  fileSpec,
  ICON_SIDE,
  type PlaceContext,
  placeStampOp,
  type ReadyStamp,
  stampSize,
} from "./stamp-ops.ts";

/** What stamps and files need of Comment mode. */
export interface FilesHost {
  readonly engine: Engine;
  readonly services: FileServices;
  frame(page: number): Promise<PageFrame | undefined>;
  nameAsked(): Promise<boolean>;
  author(): string | undefined;
  /** Applies `ops` on `page` as one step through the page's gesture; the IDs it made. */
  commit(page: number, ops: Op[], label: string): Promise<readonly string[]>;
  /** Drops the stamp's preview on `page`, when nothing is placed after all. */
  drop(page: number): void;
  /** Selects annotation `id` on `page`. */
  select(page: number, id: string): void;
  /** What the mode shows changed: the picker's stamps, the stamp chosen, Play. */
  changed(): void;
  /** Asks whether to save a program out (O14's warning). */
  confirmProgram(name: string): Promise<boolean>;
  /** The colour a new attachment's icon takes, from the Attach file tool's style. */
  attachColour(): readonly number[];
}

/** Where files come from and go, and where the reader says what happened. */
export interface FileServices {
  readonly platform: Platform;
  readonly toasts: ToastQueue;
}

/** What stamps and files show now. */
export interface FilesState {
  readonly custom: readonly CustomStampItem[];
  /** The stamp the next press places, chosen in the picker. */
  readonly chosen: StampChoice | undefined;
  /** The sound comment playing. */
  readonly playing: string | undefined;
}

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

/** The pixel size of image `bytes`, for a custom stamp's proportions. */
async function imageSize(bytes: Uint8Array, type: string): Promise<{ width: number; height: number }> {
  const bitmap = await createImageBitmap(new Blob([new Uint8Array(bytes)], { type }));
  const size = { width: bitmap.width, height: bitmap.height };
  bitmap.close();
  return size;
}

/** A custom stamp's box while its image is read, points. */
const IMAGE_PLACEHOLDER = 120;

/** A custom stamp's image in a document: its blob ID there, and its pixel size. */
interface PlacedImage {
  readonly blob: string;
  readonly width: number;
  readonly height: number;
}

/** The custom stamps, one library for the page: one IndexedDB connection whatever the mode does. */
let library: StampLibrary | undefined;
const stamps = () => {
  library ??= new StampLibrary(indexedDbStore());
  return library;
};

/** Each engine's custom stamp images, by document and stamp: a document takes an image's bytes once. */
const images = new WeakMap<Engine, Map<string, Promise<PlacedImage>>>();

/** An error's words for a toast. */
function reason(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** The stamps and attached files of one open document's Comment mode. */
export class StampsAndFiles {
  readonly #host: FilesHost;
  #state: FilesState = { custom: [], chosen: undefined, playing: undefined };
  #urls: string[] = [];
  /** The chosen custom stamp's size, once its image is read. */
  #imageSize: { readonly id: string; readonly size: readonly [number, number] } | undefined;
  /** The sound playing, stopped when the mode is left. */
  #audio: HTMLAudioElement | undefined;
  #left = false;

  constructor(host: FilesHost) {
    this.#host = host;
  }

  get state(): FilesState {
    return this.#state;
  }

  /** Whether Save file as… opens a save dialog, or downloads (C05's web difference). */
  get savesWithDialog(): boolean {
    return this.#host.services.platform.capabilities.canSaveAsDialog;
  }

  /** Reads the custom stamps for the picker, each with a thumbnail. */
  async refresh(): Promise<void> {
    let kept: Awaited<ReturnType<StampLibrary["list"]>>;
    try {
      kept = await stamps().list();
    } catch (e) {
      // IndexedDB refused (a private window's policy): the picker shows no custom stamps.
      reportError(e);
      kept = [];
    }
    const custom = kept.map((s) => ({
      id: s.id,
      name: s.name,
      url: URL.createObjectURL(new Blob([new Uint8Array(s.bytes)], { type: s.type })),
    }));
    // Read after the mode was left: nothing shows these thumbnails.
    if (this.#left) {
      for (const c of custom) URL.revokeObjectURL(c.url);
      return;
    }
    for (const url of this.#urls) URL.revokeObjectURL(url);
    this.#urls = custom.map((c) => c.url);
    this.#set({ custom });
  }

  /** Lets the thumbnails go and stops a sound playing (Comment mode left). */
  leave(): void {
    this.#left = true;
    for (const url of this.#urls) URL.revokeObjectURL(url);
    this.#urls = [];
    this.#audio?.pause();
    this.#set({ custom: [], chosen: undefined, playing: undefined });
  }

  /** The stamp the picker chose is placed by the next press; a custom stamp's image is read now, for its size. */
  choose(choice: StampChoice): void {
    this.#set({ chosen: choice });
    if (choice.kind !== "custom") return;
    void this.#image(choice.id).then(
      (image) => {
        const size = stampSize({ kind: "image", ...image });
        this.#imageSize = { id: choice.id, size };
      },
      (e: unknown) => this.#say(laterMessage("stamps.custom.unavailable", { reason: reason(e) })),
    );
  }

  /** The chosen stamp's size on the page, for the preview; undefined before one is chosen. */
  size(): readonly [number, number] | undefined {
    const chosen = this.#state.chosen;
    if (chosen === undefined) return undefined;
    if (chosen.kind !== "custom") return stampSize(chosen.kind === "standard" ? chosen : { ...chosen, lines: [] });
    // Until its image is read, a custom stamp's box is a square; it is placed in its own proportions.
    return this.#imageSize?.id === chosen.id ? this.#imageSize.size : [IMAGE_PLACEHOLDER, IMAGE_PLACEHOLDER];
  }

  /** Places the chosen stamp centred on `at`, on `page`, as one step; it is selected after. */
  async place(page: number, at: Point): Promise<void> {
    const chosen = this.#state.chosen;
    const ready = chosen === undefined || !(await this.#host.nameAsked()) ? undefined : await this.#ready(chosen);
    const context = ready === undefined ? undefined : await this.#context(page);
    if (ready === undefined || context === undefined) {
      this.#host.drop(page);
      return;
    }
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
      return { kind: "image", ...(await this.#image(chosen.id)) };
    } catch (e) {
      this.#say(laterMessage("stamps.custom.unavailable", { reason: reason(e) }));
      return undefined;
    }
  }

  /** Custom stamp `id`'s image in the open document, added to it the first time it is placed or chosen. */
  async #image(id: string): Promise<PlacedImage> {
    const { client } = await this.#host.engine.view();
    const key = `${client.document?.id ?? 0}:${id}`;
    let known = images.get(this.#host.engine);
    if (known === undefined) {
      const fresh = new Map<string, Promise<PlacedImage>>();
      known = fresh;
      images.set(this.#host.engine, fresh);
      // A restarted worker numbers its documents from 1 again, and may have reopened the file
      // without the blobs: every image goes in again.
      client.on("restarted", () => fresh.clear());
    }
    const cached = known.get(key);
    if (cached !== undefined) return cached;
    const made = (async () => {
      const stamp = await stamps().get(id);
      if (stamp === undefined) throw new Error(laterMessage("stamps.custom.gone"));
      const [blob, size] = await Promise.all([
        this.#host.engine.addBlob(stamp.bytes),
        imageSize(stamp.bytes, stamp.type),
      ]);
      return { blob, ...size };
    })();
    known.set(key, made);
    made.catch(() => known.delete(key));
    return made;
  }

  /** Add image…: a PNG or JPEG kept in the library; anything else, or storage refusing it, says why. */
  async addCustom(): Promise<void> {
    const file = await this.#pick("images");
    if (file === undefined) return;
    try {
      const added = await stamps().add(file.name, new Uint8Array(await file.arrayBuffer()));
      if (added === undefined) {
        this.#say(laterMessage("stamps.custom.unreadable", { name: file.name }));
        return;
      }
    } catch (e) {
      this.#say(laterMessage("stamps.custom.unkept", { reason: reason(e) }));
      return;
    }
    await this.refresh();
  }

  async rename(id: string, name: string): Promise<void> {
    await this.#keep(() => stamps().rename(id, name));
  }

  async remove(id: string): Promise<void> {
    if (await this.#keep(() => stamps().remove(id))) {
      if (this.#state.chosen?.kind === "custom" && this.#state.chosen.id === id) this.#set({ chosen: undefined });
    }
  }

  /** Runs a change to the library, then reads it again; a refusal says why. Whether it was made. */
  async #keep(change: () => Promise<void>): Promise<boolean> {
    try {
      await change();
    } catch (e) {
      this.#say(laterMessage("stamps.custom.unkept", { reason: reason(e) }));
      return false;
    }
    await this.refresh();
    return true;
  }

  /** The line a dynamic stamp writes under its title now. */
  dynamicLine(): string {
    return dynamicLines("approved", this.#host.author(), new Date())[1] ?? "";
  }

  /**
   * Attach file: asks for a file (first, while the click still counts as the person's, which a
   * browser asks before it shows its picker), then attaches it under an icon centred on `at`.
   */
  async attachAt(page: number, at: Point): Promise<void> {
    const file = await this.#pick("any");
    if (file === undefined || !this.#fits(file) || !(await this.#host.nameAsked())) return;
    const [context, attached] = await Promise.all([this.#context(page), this.#fileOf(file)]);
    if (context === undefined) return;
    const corner: Point = [at[0] - ICON_SIDE / 2, at[1] - ICON_SIDE / 2];
    const op = attachFileOp(attached, corner, context, { color: this.#host.attachColour() });
    const [made] = await this.#host.commit(page, [op], laterMessage("attachment.label.attach", { name: file.name }));
    if (made !== undefined) this.#host.select(page, made);
  }

  /** One file from the platform's picker (`accept`), read whole; undefined when the person cancels. */
  async #pick(accept: "images" | "any"): Promise<File | undefined> {
    const [picked] = await this.#host.services.platform.openFiles({ multiple: false, accept });
    if (picked === undefined) return undefined;
    return pickedFile(picked);
  }

  /** Whether the document could read `file` back out (O14's memory limit); a toast says when not. */
  #fits(file: File): boolean {
    const limit = this.#host.engine.getState().attachmentLimit;
    if (limit === undefined || file.size <= limit) return true;
    this.#say(laterMessage("attachments.tooLarge.attach", { name: file.name, limit: formatBytes(limit) }));
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
    const name = details.name ?? laterMessage("attachments.untitled");
    if (isProgramFile(name, details.mime) && !(await this.#host.confirmProgram(name))) return;
    const read = await this.#host.engine.attachmentBytes(details.file);
    const { platform, toasts } = this.#host.services;
    const saveExport = platform.saveExport;
    if (saveExport === undefined) return;
    const blob = new Blob([new Uint8Array(read.bytes)], details.mime === undefined ? {} : { type: details.mime });
    const result = await saveExport.call(platform, blob, name);
    if (result.saved) toasts.add({ message: message("download.done", { name: result.name }) });
  }

  /** Replace…: a new file in the annotation, which keeps its place, icon, replies, note and description, as one step. */
  async replace(page: number, a: PageAnnotation): Promise<void> {
    const file = await this.#pick("any");
    if (file === undefined || !this.#fits(file)) return;
    const details = fileOf(a);
    const attached = await this.#fileOf(file, details?.description);
    const op = updateAnnotOp(a.id, { spec: { type: "fileAttachment", file: fileSpec(attached) } });
    const label = laterMessage("attachment.label.replace", { name: details?.name ?? file.name });
    await this.#host.commit(page, [op], label);
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
      this.#audio = audio;
      await new Promise<void>((resolve, reject) => {
        audio.addEventListener("ended", () => resolve(), { once: true });
        // Leaving the mode pauses it: that ends the playing too.
        audio.addEventListener("pause", () => resolve(), { once: true });
        audio.addEventListener("error", () => reject(new Error(audio.error?.message || "the recording didn't load")), {
          once: true,
        });
        audio.play().catch(reject);
      });
    } catch (e) {
      this.#say(laterMessage("sound.unplayable", { reason: reason(e) }));
    } finally {
      if (url !== undefined) URL.revokeObjectURL(url);
      this.#audio = undefined;
      if (!this.#left) this.#set({ playing: undefined });
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

  #say(words: string): void {
    this.#host.services.toasts.add({ message: words });
  }

  #set(change: Partial<FilesState>): void {
    this.#state = { ...this.#state, ...change };
    this.#host.changed();
  }
}
