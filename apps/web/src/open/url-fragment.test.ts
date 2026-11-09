import { describe, expect, it } from "vitest";
import { fragmentAction } from "./navigation.ts";
import { buildFragment, fragmentFor, fragmentPage, hasKey, parseFragment, remoteUrl } from "./url-fragment.ts";

const enc = encodeURIComponent;

describe("parseFragment", () => {
  it("decodes an encoded url whose query holds our own key names", () => {
    const fragment = parseFragment(`#url=${enc("https://h/doc.pdf?x=1&page=2")}&page=5`);
    expect(remoteUrl(fragment)).toEqual({ kind: "url", url: "https://h/doc.pdf?x=1&page=2" });
    expect(fragmentPage(fragment)).toBe(5);
  });

  it("keeps + as +, not a space", () => {
    expect(parseFragment("q=a+b").entries[0]?.value).toBe("a+b");
  });

  it("reads keys without a value, and skips empty pairs", () => {
    const fragment = parseFragment("#debug&&page=2");
    expect(hasKey(fragment, "debug")).toBe(true);
    expect(fragment.entries.map((e) => e.key)).toEqual(["debug", "page"]);
  });

  it("splits a pair at its first =", () => {
    expect(parseFragment("k=a=b").entries[0]).toMatchObject({ key: "k", raw: "a=b", value: "a=b" });
  });

  it("gives no page for values that aren't whole numbers from 1", () => {
    for (const page of ["0", "-1", "1.5", "x", "", "01"])
      expect(fragmentPage(parseFragment(`page=${page}`))).toBeUndefined();
  });
});

describe("remoteUrl", () => {
  it("refuses an address pasted without encoding instead of guessing", () => {
    expect(remoteUrl(parseFragment("#url=https://h/doc.pdf?x=1&page=2"))).toEqual({
      kind: "invalid",
      problem: "not-encoded",
    });
  });

  it("refuses invalid percent-encoding", () => {
    expect(remoteUrl(parseFragment("url=%E0%A4%A"))).toEqual({ kind: "invalid", problem: "not-encoded" });
  });

  it("refuses a value that isn't an absolute address", () => {
    expect(remoteUrl(parseFragment(`url=${enc("doc.pdf")}`))).toEqual({ kind: "invalid", problem: "not-absolute" });
  });

  it.each(["javascript:alert(1)", "data:application/pdf;base64,JVBERi0=", "file:///etc/passwd", "ftp://h/a.pdf"])(
    "refuses %s",
    (url) => {
      expect(remoteUrl(parseFragment(`url=${enc(url)}`))).toEqual({ kind: "invalid", problem: "scheme" });
    },
  );

  it("refuses plain http, and local http unless allowed", () => {
    expect(remoteUrl(parseFragment(`url=${enc("http://h/a.pdf")}`), true)).toEqual({
      kind: "invalid",
      problem: "insecure",
    });
    const local = parseFragment(`url=${enc("http://localhost:8000/a.pdf")}`);
    expect(remoteUrl(local)).toEqual({ kind: "invalid", problem: "insecure" });
    expect(remoteUrl(local, true)).toEqual({ kind: "url", url: "http://localhost:8000/a.pdf" });
    expect(remoteUrl(parseFragment(`url=${enc("http://127.0.0.1:8000/a.pdf")}`), true).kind).toBe("url");
  });

  it("is none without a url key", () => {
    expect(remoteUrl(parseFragment("page=3"))).toEqual({ kind: "none" });
  });
});

describe("buildFragment", () => {
  const awkward = "https://h/~user/a (1)!'.pdf?q=a+b";

  it("builds the share button's fragment exactly", () => {
    expect(fragmentFor(awkward, 7)).toBe(`url=${enc(awkward)}&page=7`);
    expect(fragmentFor(awkward)).toBe(`url=${enc(awkward)}`);
  });

  it("round-trips the encoded url", () => {
    expect(remoteUrl(parseFragment(fragmentFor(awkward)))).toEqual({ kind: "url", url: awkward });
  });

  it("keeps a shared link's url byte for byte when the page changes", () => {
    const shared = fragmentFor(awkward, 1);
    const raw = shared.split("&")[0];
    const next = buildFragment(parseFragment(shared), { page: "9" });
    expect(next).toBe(`${raw}&page=9`);
  });

  it("never re-encodes a pasted link whose encoding differs from ours", () => {
    // %7E for ~, %20 for the space, %2B for +: what another tool may have written.
    const pasted = "url=https%3A%2F%2Fh%2F%7Euser%2Fa%20(1)!'.pdf%3Fq%3Da%2Bb&page=1";
    expect(buildFragment(parseFragment(pasted), { page: "2" })).toBe(
      "url=https%3A%2F%2Fh%2F%7Euser%2Fa%20(1)!'.pdf%3Fq%3Da%2Bb&page=2",
    );
  });

  it("keeps the order and unknown keys, appends new ones and removes undefined ones", () => {
    const fragment = parseFragment("debug&zoom=2&page=1");
    expect(buildFragment(fragment, { page: undefined, nameddest: "intro" })).toBe("debug&zoom=2&nameddest=intro");
  });

  it("keeps one entry for a key named twice", () => {
    expect(buildFragment(parseFragment("page=1&page=2"), { page: "3" })).toBe("page=3");
  });
});

describe("fragmentAction", () => {
  const url = "https://h/doc.pdf?x=1&page=2";
  const withUrl = (page?: number) => parseFragment(fragmentFor(url, page));

  it("opens a new address at its page", () => {
    expect(fragmentAction(withUrl(5), undefined)).toEqual({ type: "open", url, place: { page: 5 } });
    expect(fragmentAction(withUrl(), "https://other/a.pdf")).toEqual({ type: "open", url, place: undefined });
  });

  it("only navigates when the page alone changed", () => {
    expect(fragmentAction(withUrl(3), url)).toEqual({ type: "navigate", place: { page: 3 } });
    expect(fragmentAction(withUrl(), url)).toEqual({ type: "none" });
  });

  it("closes the remote document when url is removed", () => {
    expect(fragmentAction(parseFragment("page=2"), url)).toEqual({ type: "close" });
  });

  it("navigates within a local file", () => {
    expect(fragmentAction(parseFragment("page=2"), undefined)).toEqual({ type: "navigate", place: { page: 2 } });
    expect(fragmentAction(parseFragment(""), undefined)).toEqual({ type: "none" });
  });

  it("goes to a named destination, before a page", () => {
    expect(fragmentAction(parseFragment("nameddest=chapter-2&page=4"), undefined)).toEqual({
      type: "navigate",
      place: { dest: "chapter-2" },
    });
    expect(fragmentAction(parseFragment(`${fragmentFor(url)}&nameddest=a%20b`), url)).toEqual({
      type: "navigate",
      place: { dest: "a b" },
    });
  });

  it("ignores an empty named destination", () => {
    expect(fragmentAction(parseFragment("nameddest=&page=2"), undefined)).toEqual({
      type: "navigate",
      place: { page: 2 },
    });
  });

  it("reports an invalid address", () => {
    expect(fragmentAction(parseFragment("url=https://h/a.pdf"), undefined)).toEqual({
      type: "invalid",
      problem: "not-encoded",
    });
  });
});
