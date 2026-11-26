// Insert pages from a file and Replace pages in the desktop window, over a stand-in for the
// engine client and the Organise controller.

import type { ToastQueue } from "@laminapdf/ui";
import type { OrganiseController } from "@laminapdf/ui/editing";
import { describe, expect, it } from "vitest";
import { DesktopDocumentsController } from "./documents.tsx";
import type { DesktopDocument, DesktopEngineClient } from "./engine/desktop-engine-client.ts";

const PAGE = { id: "p1.0", width: 612, height: 792, rotate: 0 };
const OTHER: DesktopDocument = {
  doc: 9,
  name: "other.pdf",
  size: 10,
  pageCount: 2,
  encrypted: false,
  needsPassword: false,
  repaired: false,
  pages: [PAGE, { ...PAGE, id: "p2.0" }],
};
const LOCKED: DesktopDocument = { ...OTHER, needsPassword: true, encrypted: true, pageCount: 0, pages: [] };

/** A controller over a client that picks `picked`, and what it asked of them. */
function setup(
  picked: DesktopDocument | undefined | Error,
  addFails = false,
  many: readonly { name: string; opened: DesktopDocument | null }[] = [],
  saved: string | null = "merged.pdf",
  folder: { id: number; name: string } | null = { id: 7, name: "Parts" },
  taken: readonly string[] = [],
) {
  const log: string[] = [];
  const edits: { ops: readonly object[]; indices: readonly number[]; label: string }[] = [];
  const said: string[] = [];
  const toasted: { message: string; action?: { label: string; onPress: () => void } }[] = [];
  const toasts = { add: (options: (typeof toasted)[number]) => toasted.push(options) } as unknown as ToastQueue;
  const client = {
    current: { doc: 4, name: "base.pdf" },
    planSave: () => Promise.resolve({ signatures: [] }),
    folders: {
      choose: () => Promise.resolve(folder ?? undefined),
      existing: (_id: number, names: readonly string[]) => Promise.resolve(names.filter((n) => taken.includes(n))),
      writeParts: (doc: number, id: number, parts: readonly { name: string }[], replace: boolean) => {
        log.push(`parts ${doc} ${id} ${parts.map((p) => p.name).join(",")} replace=${replace}`);
        return Promise.resolve(parts.map((p) => `${p.name}.pdf`));
      },
      savePart: (doc: number, suggested: string, pages: readonly string[]) => {
        log.push(`part ${doc} ${suggested} ${pages.join(",")}`);
        return Promise.resolve(saved ?? undefined);
      },
      reveal: (id: number, name: string) => {
        log.push(`reveal ${id} ${name}`);
        return Promise.resolve();
      },
    },
    sources: {
      pickMany: () => Promise.resolve(many.map((m) => ({ ...m, error: null }))),
      mergeIntoNewFile: (
        doc: number,
        title: string,
        suggested: string,
        files: readonly { source: number; title: string }[],
      ) => {
        log.push(`merge ${doc} ${title} (${suggested}) ${files.map((f) => `${f.source}:${f.title}`).join(",")}`);
        return Promise.resolve(saved ?? undefined);
      },
      title: (doc: number) => Promise.resolve(doc === 9 ? "Other title" : undefined),
      pick: () => (picked instanceof Error ? Promise.reject(picked) : Promise.resolve(picked)),
      unlock: (doc: number, password: string) => {
        log.push(`unlock ${doc}`);
        return password === "right"
          ? Promise.resolve({ ...OTHER, needsPassword: false })
          : Promise.reject(Object.assign(new Error("no"), { code: "wrong-password" }));
      },
      reserve: () => () => undefined,
      add: (doc: number, source: number) => {
        log.push(`add ${doc} ${source}`);
        if (addFails) return Promise.reject(new Error("refused"));
        return Promise.resolve({ id: "abc", pages: ["s1.0", "s2.0"] });
      },
      release: (doc: number) => {
        log.push(`release ${doc}`);
        return Promise.resolve();
      },
    },
  } as unknown as DesktopEngineClient;
  const organise = {
    pages: () => [{ id: "p1.0" }, { id: "p2.0" }, { id: "p3.0" }],
    selectedIndices: () => [1],
    edit: (ops: readonly object[], indices: readonly number[], label: string) => edits.push({ ops, indices, label }),
    say: (message: string) => said.push(message),
  } as unknown as OrganiseController;
  return { documents: new DesktopDocumentsController(client, organise, toasts), log, edits, said, toasted };
}

const choice = {
  pages: [1],
  where: "after",
  page: 2,
  annotations: true,
  linkFields: false,
} as const;

describe("Insert pages from a file", () => {
  it("shows the picked file's pages, ready", async () => {
    const { documents } = setup(OTHER);
    await documents.openFromFile("from-file");
    expect(documents.getState().source?.shown).toMatchObject({ name: "other.pdf", status: "ready" });
  });

  it("stays as it was when the person cancels the file dialog", async () => {
    const { documents } = setup(undefined);
    await documents.openFromFile("from-file");
    expect(documents.getState()).toMatchObject({ dialog: "from-file", source: undefined, merge: [] });
  });

  it("says so when the file isn't a PDF", async () => {
    const { documents, said } = setup(new Error("invalid"));
    await documents.openFromFile("from-file");
    expect(said).toHaveLength(1);
  });

  it("asks for the password of an encrypted file, and again after a wrong one", async () => {
    const { documents } = setup(LOCKED);
    await documents.openFromFile("from-file");
    const first = documents.getState().source?.shown;
    await documents.sourcePassword("wrong");
    const wrong = documents.getState().source?.shown;
    await documents.sourcePassword("right");
    expect([first, wrong, documents.getState().source?.shown.status]).toMatchObject([
      { status: "password", wrong: false },
      { status: "password", wrong: true },
      "ready",
    ]);
  });

  it("inserts the chosen pages as one step over the file's source, and keeps the file open", async () => {
    const { documents, edits, log } = setup(OTHER);
    await documents.openFromFile("from-file");
    await documents.applyFromFile(choice);
    expect([log, edits[0]?.ops]).toEqual([
      ["add 4 9"],
      [
        {
          op: "insertPagesFrom",
          args: {
            source: "abc",
            pages: ["s2.0"],
            at: { after: "p3.0" },
            carry: { annotations: true, fields: "rename" },
          },
        },
      ],
    ]);
  });

  it("replaces the selected pages", async () => {
    const { documents, edits } = setup(OTHER);
    await documents.openFromFile("replace");
    await documents.applyFromFile({ ...choice, where: "replace" });
    expect(edits[0]?.ops).toMatchObject([
      { op: "replacePages", args: { targets: ["p2.0"], source: "abc", pages: ["s2.0"] } },
    ]);
  });

  it("lets go of a file nothing took pages from when the dialog closes", async () => {
    const { documents, log } = setup(OTHER);
    await documents.openFromFile("from-file");
    await documents.close();
    expect([log, documents.getState()]).toMatchObject([
      ["release 9"],
      { dialog: undefined, source: undefined, merge: [] },
    ]);
  });

  it("lets go of the file, and says so, when it can't be given to the document", async () => {
    const { documents, log, said } = setup(OTHER, true);
    await documents.openFromFile("from-file");
    await documents.applyFromFile(choice);
    expect([log, said.length]).toEqual([["add 4 9", "release 9"], 1]);
  });
});

describe("Merge documents", () => {
  const SECOND: DesktopDocument = { ...OTHER, doc: 10, name: "second.pdf" };
  const files = [
    { name: "other.pdf", opened: OTHER },
    { name: "second.pdf", opened: SECOND },
    { name: "broken.pdf", opened: null },
  ];

  it("lists the files added, a broken one as unreadable", async () => {
    const { documents } = setup(undefined, false, files);
    documents.openMerge();
    await documents.addMergeFiles();
    expect(documents.getState().merge.map((f) => [f.name, f.shown.status])).toEqual([
      ["other.pdf", "ready"],
      ["second.pdf", "ready"],
      ["broken.pdf", "unreadable"],
    ]);
  });

  it("moves a file down and takes one out, closing it", async () => {
    const { documents, log } = setup(undefined, false, files);
    documents.openMerge();
    await documents.addMergeFiles();
    const [first, second] = documents.getState().merge;
    documents.moveMergeFile(first?.key ?? "", 1);
    await documents.removeMergeFile(second?.key ?? "");
    expect([documents.getState().merge.map((f) => f.name), log]).toEqual([["other.pdf", "broken.pdf"], ["release 10"]]);
  });

  it("adds the readable files to the document as one step, named by their titles", async () => {
    const { documents, edits, log } = setup(undefined, false, files);
    documents.openMerge();
    await documents.addMergeFiles();
    await documents.merge({ result: "add", bookmarks: "perFile", fields: "rename" });
    expect([log.filter((l) => l.startsWith("add")), edits[0]?.ops]).toEqual([
      ["add 4 9", "add 4 10"],
      [
        {
          op: "mergeDocuments",
          args: {
            sources: [
              { source: "abc", title: "Other title" },
              { source: "abc", title: "second" },
            ],
            at: "end",
            options: { outline: "perFile", fields: "rename" },
          },
        },
      ],
    ]);
  });

  it("closes the files when the dialog closes, unless an edit took pages from them", async () => {
    const { documents, log } = setup(undefined, false, files);
    documents.openMerge();
    await documents.addMergeFiles();
    await documents.close();
    expect(log.sort()).toEqual(["release 10", "release 9"]);
  });

  it("merges into a new file named by the person, and says where it went", async () => {
    const { documents, log, said, edits } = setup(undefined, false, files);
    documents.openMerge();
    await documents.addMergeFiles();
    await documents.merge({ result: "new", bookmarks: "keep", fields: "link" });
    expect([log.filter((l) => l.startsWith("merge")), said.length, edits.length]).toEqual([
      ["merge 4 base (base merged.pdf) 9:Other title,10:second"],
      1,
      0,
    ]);
  });

  it("says nothing and keeps nothing when the person cancels the save dialog", async () => {
    const { documents, said, log } = setup(undefined, false, files, null);
    documents.openMerge();
    await documents.addMergeFiles();
    await documents.merge({ result: "new", bookmarks: "perFile", fields: "rename" });
    expect([said, log.filter((l) => l.startsWith("release")).sort()]).toEqual([[], ["release 10", "release 9"]]);
  });
});

describe("Split document and Extract pages", () => {
  const parts = [
    { name: "one", pages: [0, 1] },
    { name: "two", pages: [2] },
  ];

  it("writes several parts into a folder the person chose, naming it in the toast and offering to show the first", async () => {
    const { documents, log, toasted } = setup(undefined);
    documents.openSplit("split");
    await documents.split(parts);
    toasted[0]?.action?.onPress();
    expect([log, toasted[0]?.message, documents.getState().dialog]).toEqual([
      ["parts 4 7 one,two replace=true", "reveal 7 one.pdf"],
      "Saved 2 files in Parts.",
      undefined,
    ]);
  });

  it("keeps both when the folder has files of those names and the person says so", async () => {
    const { documents, log } = setup(undefined, false, [], "x.pdf", { id: 7, name: "Parts" }, ["one"]);
    documents.openSplit("split");
    const done = documents.split(parts);
    await Promise.resolve();
    await new Promise((resolve) => setTimeout(resolve, 0));
    documents.getState().exists?.answer(false);
    await done;
    expect(log).toContain("parts 4 7 one,two replace=false");
  });

  it("writes nothing when the person cancels the question about files that exist", async () => {
    const { documents, log } = setup(undefined, false, [], "x.pdf", { id: 7, name: "Parts" }, ["one"]);
    documents.openSplit("split");
    const done = documents.split(parts);
    await new Promise((resolve) => setTimeout(resolve, 0));
    documents.getState().exists?.answer(undefined);
    await done;
    expect([log.filter((l) => l.startsWith("parts")), documents.getState().dialog]).toEqual([[], "split"]);
  });

  it("writes nothing when the person cancels the folder dialog", async () => {
    const { documents, log } = setup(undefined, false, [], "x.pdf", null);
    documents.openSplit("split");
    await documents.split(parts);
    expect([log, documents.getState().dialog]).toEqual([[], "split"]);
  });

  it("writes one part to a file the person chooses", async () => {
    const { documents, log, toasted } = setup(undefined, false, [], "one.pdf");
    documents.openSplit("extract");
    await documents.split([{ name: "one", pages: [1, 2] }]);
    expect([log, toasted[0]?.message]).toEqual([["part 4 one.pdf p2.0,p3.0"], "Saved one.pdf."]);
  });
});
