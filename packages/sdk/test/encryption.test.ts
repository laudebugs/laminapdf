// Protecting a file with a password and limits: doc.setEncryption, the entropy a save draws
// itself, doc.encryptionInfo, and what a reader needs to open the result. A test's title starts
// with the capability it proves (its tag: capability G.02, column pdf), which
// `cargo xtask capabilities` counts.
import { Document, init, isLaminaPdfError, type EncryptionOptions } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init());

const SEED = "2a".repeat(32);

function plain(): Document {
  return Document.open(read("fixtures/corpus/hello.pdf"));
}

/** The corpus file protected with `options`, as a fixed seed writes it. */
function protect(options: EncryptionOptions): Uint8Array<ArrayBuffer> {
  using doc = plain();
  doc.setEncryption(options);
  return doc.save({ entropySeed: SEED });
}

describe("setEncryption", () => {
  it("[cap G.02 pdf round-trip] writes a file that opens only with its password", () => {
    const bytes = protect({ user: "open sesame", owner: "owner secret" });
    expect(isLaminaPdfError(thrown(() => Document.open(bytes)), "password-required")).toBe(true);
    expect(isLaminaPdfError(thrown(() => Document.open(bytes, { password: "nope" })), "wrong-password")).toBe(true);
    using user = Document.open(bytes, { password: "open sesame" });
    using owner = Document.open(bytes, { password: "owner secret" });
    expect([user.encryptionInfo()?.openedWith, owner.encryptionInfo()?.openedWith]).toEqual(["user", "owner"]);
    expect(user.pageCount).toBe(plain().pageCount);
  });

  it("[cap G.01 pdf round-trip] opens without a prompt when there is no user password", () => {
    const bytes = protect({ owner: "owner secret" });
    using doc = Document.open(bytes);
    expect(doc.encryptionInfo()?.openedWith).toBe("user");
  });

  it("[cap G.11 pdf read] reports the handler, ciphers and key length, and no key", () => {
    using doc = Document.open(protect({ user: "pw" }), { password: "pw" });
    const info = doc.encryptionInfo();
    expect(info).toMatchObject({
      handler: "Standard",
      version: 5,
      revision: 6,
      keyBits: 256,
      stringCipher: "aes-256",
      streamCipher: "aes-256",
      encryptMetadata: true,
    });
    expect(JSON.stringify(info)).not.toContain("pw");
    using unprotected = plain();
    expect(unprotected.encryptionInfo()).toBeUndefined();
  });

  it("[cap G.04 pdf round-trip] writes the limits it was given, and keeps screen readers allowed", () => {
    const bytes = protect({
      owner: "owner secret",
      permissions: { print: "lowResolution", copy: false, comment: false },
    });
    using doc = Document.open(bytes);
    expect(doc.encryptionInfo()?.permissions).toEqual({
      print: true,
      printHighQuality: false,
      modify: false,
      copy: false,
      annotate: false,
      fillForms: true,
      accessibility: true,
      assemble: true,
    });
  });

  it("[cap G.03 pdf round-trip] makes a permissions password when it is left empty, and reports it once", () => {
    using doc = plain();
    doc.setEncryption({ user: "open sesame" });
    expect(doc.hasPendingEncryption()).toBe(true);
    const { bytes, report } = doc.saveWithReport({ entropySeed: SEED });
    expect(report.encrypted).toBe(true);
    const made = report.generatedOwnerPassword;
    expect(made).toMatch(/^[0-9a-f]{32}$/);
    using owner = Document.open(bytes, { password: made });
    expect(owner.encryptionInfo()?.openedWith).toBe("owner");
    expect(doc.hasPendingEncryption()).toBe(false);
  });

  it("refuses a request with no password at all", () => {
    using doc = plain();
    const error = thrown(() => doc.setEncryption({}));
    expect(isLaminaPdfError(error, "encryption-no-password")).toBe(true);
    expect(doc.hasPendingEncryption()).toBe(false);
  });

  it("draws its own entropy, so two saves differ, and a fixed seed repeats", () => {
    const save = (seed?: string): string => {
      using doc = plain();
      doc.setEncryption({ user: "pw" });
      return Buffer.from(doc.save(seed === undefined ? {} : { entropySeed: seed })).toString("hex");
    };
    expect(save()).not.toBe(save());
    expect(save(SEED)).toBe(save(SEED));
  });

  it("applies inside a transaction", () => {
    using doc = plain();
    doc.transact((tx) => tx.setEncryption({ user: "pw" }));
    expect(doc.hasPendingEncryption()).toBe(true);
  });

  it("keeps a protected file protected when it is edited and saved again", () => {
    using doc = Document.open(protect({ user: "pw", owner: "owner secret" }), { password: "pw" });
    doc.setMetadata({ title: "Edited" });
    const bytes = doc.save();
    using again = Document.open(bytes, { password: "pw" });
    expect(again.encryptionInfo()?.revision).toBe(6);
    expect(again.metadata().title).toBe("Edited");
  });
});
