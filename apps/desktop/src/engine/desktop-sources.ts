// The other files an edit takes pages from (Insert pages from a file, Replace pages): each is a
// document of its own in this window's engine, picked in the main process's dialog (the path never
// reaches the webview), so its pages draw as any document's do. Giving one to a document's edit
// session (`add_source`) returns the ID its ops name it by. That session is the engine's, and it
// is lost when the engine restarts, so the sources a step needs are kept open as long as the
// document is, and are given to the new engine again before the step is made again (desktop-edits).

import type { DesktopDocument } from "./desktop-engine-client.ts";

/** Calls a main process command, rejecting with the client's `DesktopEngineError`. */
export type Call = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

/** What adding a source returns: its ID in the edit session, and its pages' IDs in page order. */
export interface AddedSource {
  readonly id: string;
  readonly pages: readonly string[];
}

/** A file picked for a merge, as the main process opened it: `PickedSource` in Rust. */
export interface PickedSource {
  /** The file's name, without its folder. */
  readonly name: string;
  /** The document it opened as, or none when it didn't open. */
  readonly opened: DesktopDocument | null;
  /** Why it didn't open. */
  readonly error: { readonly kind: string; readonly message: string } | null;
}

/** A source a document's edit session holds. */
interface Held {
  /** The source's document in the main process. */
  readonly source: number;
  readonly id: string;
}

/** The sources of this window's documents. */
export class DesktopSources {
  readonly #call: Call;
  /** The sources each document's edit session holds, by the document. */
  readonly #held = new Map<number, Held[]>();
  /** The sources being given to a document: a dialog that closes meanwhile must not close them. */
  readonly #adding = new Set<number>();

  constructor(call: Call) {
    this.#call = call;
  }

  /** Asks for a PDF in the native dialog and opens it; undefined when the person cancels. */
  async pick(): Promise<DesktopDocument | undefined> {
    return (await this.#call<DesktopDocument | null>("pick_source")) ?? undefined;
  }

  /** Asks for PDFs in the native dialog and opens each; none when the person cancels. */
  async pickMany(): Promise<PickedSource[]> {
    return await this.#call<PickedSource[]>("pick_sources");
  }

  /** The title in `doc`'s properties, if it has one: what names its bookmark in a merge. */
  async title(doc: number): Promise<string | undefined> {
    const read = await this.#call<{ properties: { title?: string }; metadata: { title?: string } }>(
      "document_properties",
      { doc },
    ).catch(() => undefined);
    const title = read?.metadata.title ?? read?.properties.title;
    return title === undefined || title.trim() === "" ? undefined : title;
  }

  /** Unlocks an encrypted source with the password the person typed. */
  async unlock(source: number, password: string): Promise<DesktopDocument> {
    return await this.#call<DesktopDocument>("unlock_document", { doc: source, password });
  }

  /** Gives `doc`'s edit session `source`, and keeps `source` open as long as `doc` is. */
  async add(doc: number, source: number): Promise<AddedSource> {
    this.#adding.add(source);
    try {
      const added = await this.#call<AddedSource>("add_source", { doc, source });
      const held = this.#held.get(doc) ?? [];
      if (!held.some((h) => h.source === source && h.id === added.id)) held.push({ source, id: added.id });
      this.#held.set(doc, held);
      return added;
    } finally {
      this.#adding.delete(source);
    }
  }

  /** The source documents behind the IDs `ops` name, which `doc`'s edit session holds. */
  needed(doc: number, ops: readonly object[]): number[] {
    const ids = new Set(sourceIds(ops));
    return (this.#held.get(doc) ?? []).filter((h) => ids.has(h.id)).map((h) => h.source);
  }

  /** Gives `doc`'s edit session each of `sources` again, as a restarted engine has none. */
  async restore(doc: number, sources: readonly number[]): Promise<void> {
    for (const source of sources) await this.#call("add_source", { doc, source });
  }

  /** Closes source `source`, unless an edit session holds it, or is being given it. */
  async release(source: number): Promise<void> {
    if (this.#adding.has(source)) return;
    for (const held of this.#held.values()) if (held.some((h) => h.source === source)) return;
    await this.#call("close_document", { doc: source }).catch(() => undefined);
  }

  /** Closes the sources `doc`'s edit session held: it closed. */
  async closeAll(doc: number): Promise<void> {
    const held = this.#held.get(doc) ?? [];
    this.#held.delete(doc);
    await Promise.all(held.map((h) => this.release(h.source)));
  }
}

/** The source IDs ops name: `args.source`, and `args.sources[].source` of a merge. */
export function sourceIds(ops: readonly object[]): string[] {
  return ops.flatMap((op) => {
    const args = (op as { args?: { source?: unknown; sources?: unknown } }).args;
    const merged = Array.isArray(args?.sources) ? (args.sources as { source?: unknown }[]) : [];
    return [args?.source, ...merged.map((m) => m.source)].filter((id): id is string => typeof id === "string");
  });
}
