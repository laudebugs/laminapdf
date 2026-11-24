import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { ISSUES_URL, SUPPORT, SUPPORT_EMAIL } from "../src/data/support.ts";

// The support page speaks for the apps, so its claims are held to the apps' own text.
const root = new URL("../../../", import.meta.url);
const read = (path: string) => readFileSync(new URL(path, root), "utf8");

describe("the support page", () => {
  it("gives the address the owner chose, as text and as a link", () => {
    expect(SUPPORT_EMAIL).toBe("support@laminapdf.com");
    expect(SUPPORT.description).toContain(SUPPORT_EMAIL);
    const page = read("apps/site/src/pages/support.astro");
    expect(page).toContain("mailto:${SUPPORT_EMAIL}");
    expect(page).toContain("{SUPPORT_EMAIL}</a>");
  });

  it("says about crash reports only what the apps' own text says", () => {
    const messages = read("packages/ui/src/messages.ts");
    expect(messages).toContain(
      "send an anonymous report of what went wrong. It never holds your files, their names or what's in them. Off until you turn it on",
    );
    expect(messages).toContain("This copy came from the Mac App Store, so it sends no crash reports of its own");
    const text = SUPPORT.leaveOut.body.join(" ");
    expect(text).toContain("anonymous");
    expect(text).toContain("never holds your files, their names or what's in them");
  });

  it("names a tracker that is this repository's issues page, and asks for no PDFs", () => {
    expect(ISSUES_URL).toBe("https://github.com/laudebugs/laminapdf/issues");
    expect(SUPPORT.leaveOut.body[0]).toContain("Don't attach your PDFs");
    expect(SUPPORT.leaveOut.body[0]).toContain("private information");
  });

  it("promises no response time", () => {
    const all = JSON.stringify(SUPPORT).toLowerCase();
    for (const word of ["within", "hours", "business day", "24", "48", "guarantee"]) expect(all).not.toContain(word);
  });
});
