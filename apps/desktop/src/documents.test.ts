// Insert pages from a file and Replace pages in the desktop window, over a stand-in for the
// engine client and the Organise controller.

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
function setup(picked: DesktopDocument | undefined | Error) {
  const log: string[] = [];
  const edits: { ops: readonly object[]; indices: readonly number[]; label: string }[] = [];
  const said: string[] = [];
  const client = {
    current: { doc: 4 },
    sources: {
      pick: () => (picked instanceof Error ? Promise.reject(picked) : Promise.resolve(picked)),
      unlock: (doc: number, password: string) => {
        log.push(`unlock ${doc}`);
        return password === "right"
          ? Promise.resolve({ ...OTHER, needsPassword: false })
          : Promise.reject(Object.assign(new Error("no"), { code: "wrong-password" }));
      },
      add: (doc: number, source: number) => {
        log.push(`add ${doc} ${source}`);
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
  return { documents: new DesktopDocumentsController(client, organise), log, edits, said };
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
    expect(documents.getState()).toEqual({ dialog: "from-file", source: undefined });
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
    expect([log, documents.getState()]).toEqual([["release 9"], { dialog: undefined, source: undefined }]);
  });
});
