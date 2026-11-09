import { describe, expect, it } from "vitest";
import { externalLink } from "./link-url.js";

describe("externalLink", () => {
  it("opens https with the host apart", () => {
    expect(externalLink("https://example.com/a?b=c#d")).toEqual({
      kind: "open",
      href: "https://example.com/a?b=c#d",
      parts: { before: "https://", host: "example.com", after: "/a?b=c#d" },
    });
  });

  it("opens http and keeps a port with the host", () => {
    expect(externalLink("http://example.com:8080/x")).toMatchObject({
      parts: { before: "http://", host: "example.com:8080", after: "/x" },
    });
  });

  it("shows an international host in punycode", () => {
    expect(externalLink("https://bücher.example/")).toMatchObject({
      href: "https://xn--bcher-kva.example/",
      parts: { host: "xn--bcher-kva.example" },
    });
  });

  it("emphasises the host after credentials, not the name before the @", () => {
    expect(externalLink("https://example.com@evil.test/")).toMatchObject({
      parts: { before: "https://example.com@", host: "evil.test", after: "/" },
    });
  });

  it("finds the host after credentials that spell a host", () => {
    expect(externalLink("https://evil.com.x@evil.com/")).toMatchObject({
      parts: { before: "https://evil.com.x@", host: "evil.com", after: "/" },
    });
  });

  it("opens mailto with the domain apart", () => {
    expect(externalLink("mailto:someone@example.com?subject=hi")).toEqual({
      kind: "open",
      href: "mailto:someone@example.com?subject=hi",
      parts: { before: "mailto:someone@", host: "example.com", after: "?subject=hi" },
    });
  });

  it.each([
    "javascript:alert(1)",
    "JAVASCRIPT:alert(1)",
    " javascript:alert(1)",
    "file:///etc/passwd",
    "data:text/html,hi",
    "ftp://example.com/",
    "budget.pdf",
    "",
  ])("refuses %j", (uri) => {
    expect(externalLink(uri)).toEqual({ kind: "refused", uri });
  });
});
