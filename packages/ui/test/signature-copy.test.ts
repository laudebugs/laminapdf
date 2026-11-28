import { describe, expect, it } from "vitest";
import { LATER_ENGLISH } from "../src/later-messages.ts";

/** The signature image's words: every message under `sign.`, and its command's title. */
const SIGNATURE_COPY = Object.entries(LATER_ENGLISH).filter(
  ([id]) => id.startsWith("sign.") || id === "command.sign.add-signature-image",
);

describe("the signature image's copy", () => {
  it("has copy to check", () => {
    expect(SIGNATURE_COPY.length).toBeGreaterThan(30);
  });

  it("never calls a picture of a signature a digital signature, except to say it isn't one", () => {
    const calls = SIGNATURE_COPY.flatMap(([id, text]) =>
      [...text.matchAll(/digital signature/gi)]
        .filter(
          (hit) =>
            !/\bnot an? $/i.test(text.slice(Math.max(0, (hit.index ?? 0) - 6), hit.index)) &&
            !/· not an? $/i.test(text.slice(0, hit.index)),
        )
        .map(() => id),
    );
    expect(calls).toEqual([]);
  });

  it("says at the toolbar that the picture doesn't prove who signed", () => {
    expect(LATER_ENGLISH["sign.toolbar.proof"]).toMatch(/doesn’t prove who signed/);
  });

  it("names the certificate signature as the way to prove who signed", () => {
    expect(LATER_ENGLISH["sign.note"]).toMatch(/Sign with a certificate/);
  });

  it("keeps nothing it says about storage vague: the library stays on this device", () => {
    expect(LATER_ENGLISH["sign.keep.web"]).toMatch(/never uploaded/);
    expect(LATER_ENGLISH["sign.keep.desktop"]).toMatch(/never uploaded/);
  });
});
