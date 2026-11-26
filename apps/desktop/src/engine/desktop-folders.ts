// Folders the person chose to write files into, and the files the main process writes there: the
// parts of Split document and Extract pages. The main process keeps each folder's path and the
// webview names it by an id (src-tauri/src/folders.rs), so no path reaches the page, and the
// engine's parts go to the folder without passing through it.

import type { Call } from "./desktop-sources.ts";

/** A folder the person chose: its id, which the main process knows its path by, and its name. */
export interface DesktopFolder {
  readonly id: number;
  readonly name: string;
}

/** One part to write: its name without `.pdf`, and its pages' IDs. */
export interface PartFile {
  readonly name: string;
  readonly pages: readonly string[];
}

/** The folders of one window. */
export class DesktopFolders {
  readonly #call: Call;

  constructor(call: Call) {
    this.#call = call;
  }

  /** Asks for a folder in the native dialog; undefined when the person cancels. */
  async choose(): Promise<DesktopFolder | undefined> {
    return (await this.#call<DesktopFolder | null>("choose_folder")) ?? undefined;
  }

  /** Which of `names` are files in `folder` already. */
  existing(folder: number, names: readonly string[]): Promise<string[]> {
    return this.#call<string[]>("folder_existing", { folder, names });
  }

  /** Writes `bytes` to the file `name` in `folder`, replacing a file of that name. */
  async write(folder: number, name: string, bytes: Uint8Array): Promise<void> {
    await this.#call("folder_write", { folder, name, bytes: Array.from(bytes) });
  }

  /**
   * Writes each of `parts`, new documents of pages of document `doc` as its edits leave it, into
   * `folder`: a file whose name is taken is replaced when `replace`, else the part is written
   * under a free name. Resolves to the names written, in order.
   */
  writeParts(doc: number, folder: number, parts: readonly PartFile[], replace: boolean): Promise<string[]> {
    return this.#call<string[]>("write_parts", { doc, folder, parts, replace });
  }

  /**
   * Writes a new document of `pages` of document `doc` to a file the person chooses, suggested the
   * name `suggested`. Resolves to the file's name, or undefined when they cancel.
   */
  async savePart(doc: number, suggested: string, pages: readonly string[]): Promise<string | undefined> {
    return (await this.#call<string | null>("save_part", { doc, suggested, pages })) ?? undefined;
  }

  /** Shows file `name` of `folder` in Finder or Explorer. */
  async reveal(folder: number, name: string): Promise<void> {
    await this.#call("reveal_in_folder", { folder, name });
  }
}
