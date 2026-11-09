import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { userEvent } from "vitest/browser";
import { FindResults } from "../../src/index.ts";
import { FakeFind } from "../fakes.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(cleanup);

const snippet = (match: string) => ({ before: "…the ", match, after: " of the year…" });
const LIST = [
  { ordinal: 0, page: 2, snippet: snippet("backlog") },
  { ordinal: 1, page: 2, snippet: snippet("Backlog") },
  { ordinal: 2, page: 7, snippet: snippet("backlog") },
];
const heading = (page: number) => `Page ${page + 1}${page === 2 ? " · Repairs backlog" : ""}`;

function results(state: ConstructorParameters<typeof FakeFind>[0] = {}) {
  const find = new FakeFind({ query: "backlog", total: 3, current: 0, ...state });
  find.list = LIST;
  const went: string[] = [];
  const host = renderIn(<FindResults find={find} heading={heading} onNavigate={() => went.push("navigated")} />);
  return { find, host, went };
}

describe("FindResults", () => {
  it("groups the matches by page under their headings, as links in page order", () => {
    const { host } = results();
    const groups = [...host.querySelectorAll("section")];
    expect(groups.map((g) => g.querySelector("h3")?.textContent)).toEqual(["Page 3 · Repairs backlog", "Page 8"]);
    expect(groups.map((g) => g.getAttribute("aria-label"))).toEqual(["Page 3 · Repairs backlog", "Page 8"]);
    const links = [...host.querySelectorAll("a")];
    expect(links.map((a) => a.getAttribute("href"))).toEqual(["#page=3", "#page=3", "#page=8"]);
    expect(links[1]?.textContent).toBe("…the Backlog of the year…");
    expect(links[1]?.querySelector("strong")?.textContent).toBe("Backlog");
  });

  it("marks the current match, and follows it as find moves", async () => {
    const { find, host } = results();
    const current = () =>
      [...host.querySelectorAll('[aria-current="true"]')].map((a) => a.getAttribute("data-ordinal"));
    expect(current()).toEqual(["0"]);
    find.set({ current: 2 });
    await expect.poll(current).toEqual(["2"]);
  });

  it("goes to a match on a click or Enter, then tells the reader", async () => {
    const { find, host, went } = results();
    const links = host.querySelectorAll("a");
    await userEvent.click(links[2] as HTMLElement);
    (links[1] as HTMLElement).focus();
    await userEvent.keyboard("{Enter}");
    expect(find.went).toEqual([2, 1]);
    expect(went).toEqual(["navigated", "navigated"]);
    // The links' own fragments never reach the address bar.
    expect(location.hash).not.toContain("page=");
  });

  it("says when nothing matched, and offers to turn off Whole words before Match case", async () => {
    const find = new FakeFind({
      query: "backlogs",
      total: 0,
      done: true,
      options: { wholeWords: true, matchCase: true },
    });
    const host = renderIn(<FindResults find={find} heading={heading} />);
    expect(host.querySelector("h3")?.textContent).toBe("No matches for “backlogs”.");
    await userEvent.click(host.querySelector("button") as HTMLElement);
    expect(find.calls).toEqual([{ query: "backlogs", options: { wholeWords: false, matchCase: true } }]);
  });

  it("offers no toggle when no option is on, and shows nothing while find is idle", () => {
    const none = renderIn(<FindResults find={new FakeFind({ query: "x", done: true })} heading={heading} />);
    expect(none.querySelector("button")).toBeNull();
    const idle = renderIn(<FindResults find={new FakeFind()} heading={heading} />);
    expect(idle.textContent).toBe("");
  });

  it("lists the matches found so far while pages are still searched", () => {
    const { host } = results({ done: false, total: 3 });
    expect(host.querySelectorAll("a")).toHaveLength(3);
  });

  for (const theme of ["light", "dark", "high-contrast"] as const) {
    it(`passes axe in the ${theme} theme`, async () => {
      const find = new FakeFind({ query: "backlog", total: 3, current: 1 });
      find.list = LIST;
      const host = renderIn(<FindResults find={find} heading={heading} />, { theme });
      const { violations } = await axe.run(host, { resultTypes: ["violations"] });
      expect(violations.map((v) => v.id)).toEqual([]);
    });
  }
});
