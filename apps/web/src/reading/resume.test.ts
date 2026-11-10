// The document open when the reader last closed comes back at start only from a handle whose
// permission the browser kept, never asking (nothing was clicked), and never over a document
// that opened meanwhile.
import type { Platform, PlatformFile, ReaderState, RecentFile } from "@laminapdf/ui";
import { describe, expect, it } from "vitest";
import { SESSION_KEY } from "./positions.ts";
import { resumeLastDocument } from "./resume.ts";

const FILE = { name: "long-read.pdf", size: 10, writable: false, read: async () => new Uint8Array() } as PlatformFile;

/** A platform with one recent entry and a session record naming it, whose reopen gives `file`. */
function platformWith(entry: Partial<RecentFile>, file: PlatformFile | undefined, record: unknown = { recentId: "a" }) {
  const asked: { id: string; ask: boolean | undefined }[] = [];
  const platform = {
    storage: { get: async (key: string) => (key === SESSION_KEY ? record : undefined) },
    recent: {
      list: async () => [{ id: "a", name: FILE.name, size: FILE.size, lastOpened: 0, ...entry }],
      reopen: async (id: string, options?: { ask?: boolean }) => {
        asked.push({ id, ask: options?.ask });
        return file;
      },
    },
  } as unknown as Platform;
  return { platform, asked };
}

/** A reader in `phase` that records what it was asked to open. */
function reader(phase: ReaderState["phase"] = "empty") {
  const opened: PlatformFile[] = [];
  return {
    opened,
    getState: () => ({ phase }) as ReaderState,
    open: async (file: PlatformFile) => void opened.push(file),
  };
}

describe("resuming the last document at start", () => {
  it("reopens it from its kept handle without asking for permission", async () => {
    const { platform, asked } = platformWith({ hasHandle: true }, FILE);
    const r = reader();
    await resumeLastDocument(platform, r, true);
    expect(asked).toEqual([{ id: "a", ask: false }]);
    expect(r.opened).toEqual([FILE]);
  });

  it("leaves it to the home screen when the browser no longer allows reading it", async () => {
    const { platform } = platformWith({ hasHandle: true }, undefined);
    const r = reader();
    await resumeLastDocument(platform, r, true);
    expect(r.opened).toEqual([]);
  });

  it("asks nothing for an entry without a handle, without a record, or with recent files off", async () => {
    for (const [entry, record, remember] of [
      [{ hasHandle: false }, { recentId: "a" }, true],
      [{ hasHandle: true }, null, true],
      [{ hasHandle: true }, { recentId: "gone" }, true],
      [{ hasHandle: true }, { recentId: "a" }, false],
    ] as const) {
      const { platform, asked } = platformWith(entry, FILE, record);
      await resumeLastDocument(platform, reader(), remember);
      expect(asked).toEqual([]);
    }
  });

  it("never replaces a document that opened meanwhile", async () => {
    const { platform } = platformWith({ hasHandle: true }, FILE);
    const r = reader("open");
    await resumeLastDocument(platform, r, true);
    expect(r.opened).toEqual([]);
  });
});
