// Stamps, files attached to a spot on a page, and sound comments: stampAt, attachFileAt and
// soundAsWav, saved and reopened. A test's title starts with the capability it proves (its tag:
// capabilities D.07 to D.09, column pdf), which `cargo xtask capabilities` counts.
import { Document, init, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(async () => {
  await init();
  await loadExtract();
});

const CSV = new TextEncoder().encode("day,reading\n1,12.5\n");

function savedAndReopened(doc: Document): Document {
  return Document.open(doc.save({ mode: "incremental" }));
}

describe("stamps and attachments", () => {
  it("[cap D.07 pdf round-trip] stamps a page with a standard and a dynamic stamp that reopen with their names", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const approved = doc.stampAt(0, [300, 60, 480, 110], { standard: "approved" });
    const dynamic = doc.stampAt(0, [300, 120, 480, 180], {
      dynamic: { lines: ["RECEIVED", "by Ada Lovelace"], style: "blue" },
    });
    doc.apply([
      { op: "addAnnot", args: approved },
      { op: "addAnnot", args: dynamic },
    ]);
    using again = savedAndReopened(doc);
    const stamps = again.pageAnnotations(0).filter((a) => a.subtype === "Stamp");
    expect(stamps.map((a) => [a.contents, a.hasAppearance, a.details])).toEqual([
      ["Approved", true, { kind: "stamp", name: "Approved" }],
      ["RECEIVED by Ada Lovelace", true, { kind: "stamp", name: "LMNA_Dynamic" }],
    ]);
  });

  it("[cap D.08 pdf round-trip] attaches a file under an icon that reopens byte for byte", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const attach = doc.attachFileAt(0, [40, 40], "readings.csv", CSV, "text/csv");
    doc.apply([{ op: "addAnnot", args: attach }]);
    using again = savedAndReopened(doc);
    const [annot] = again.pageAnnotations(0);
    const details = annot?.details;
    if (details?.kind !== "fileAttachment") throw new Error(`not an attachment: ${JSON.stringify(details)}`);
    expect([annot?.contents, details.name, details.mime, details.icon]).toEqual([
      "readings.csv",
      "readings.csv",
      "text/csv",
      "Paperclip",
    ]);
    expect(again.attachmentBytes(details.file ?? "").bytes).toEqual(CSV);
  });

  it("[cap D.09 pdf read] plays each sound comment's samples as WAV, and refuses compressed ones", () => {
    using doc = Document.open(read("fixtures/annotations/sound.pdf"));
    const sounds = doc.pageAnnotations(0);
    const playable = sounds.map((a) => (a.details?.kind === "sound" ? a.details.playable : undefined));
    expect(playable).toEqual([true, true, true, true, true, true, false]);
    const wav = doc.soundAsWav(sounds[0]?.id ?? "");
    expect(new TextDecoder().decode(wav.subarray(0, 4))).toBe("RIFF");
    expect(() => doc.soundAsWav(sounds[6]?.id ?? "")).toThrow(/sound comment/);
  });
});
