import { ReaderController } from "@laminapdf/ui";
import { fakePlatform } from "@laminapdf/ui/testing";
import { describe, expect, it } from "vitest";
import { type Engine, remoteFile } from "../session.ts";
import { watchFragment } from "./watch.ts";

const URL_A = "https://example.com/a.pdf";
const LINKED = `#url=${encodeURIComponent(URL_A)}`;

/** A window whose address changes by `go` (a hashchange) and records every replaceState. */
function fakeWindow(hash: string) {
  let current = hash;
  const replaced: string[] = [];
  const listeners = new Map<string, () => void>();
  const win = {
    location: {
      pathname: "/",
      search: "",
      get hash() {
        return current;
      },
    },
    history: {
      state: null,
      replaceState(_state: unknown, _title: string, url: string) {
        current = url.replace(/^[^#]*/, "");
        replaced.push(url);
      },
    },
    addEventListener: (type: string, listener: () => void) => listeners.set(type, listener),
    removeEventListener: (type: string) => listeners.delete(type),
  };
  const go = (next: string) => {
    current = next;
    listeners.get("hashchange")?.();
  };
  return { win: win as unknown as Window, replaced, go, hash: () => current };
}

/** An engine over `reader`, with what the watcher asks of it: no viewer, no link problem. */
function fakeEngine(reader: ReaderController, problems: string[] = []): Engine {
  const engine = {
    reader,
    getState: () => ({ viewer: undefined, linkProblem: undefined }),
    subscribe: () => () => undefined,
    openRemote: () => Promise.resolve(),
    goTo: () => undefined,
    showLinkProblem: async (problem: string) => {
      if (!(await reader.requestClose())) return false;
      problems.push(problem);
      return true;
    },
  };
  return engine as unknown as Engine;
}

/** A reader holding the linked file, whose close guard answers `leave`. */
async function holding(leave: boolean): Promise<ReaderController> {
  const reader = new ReaderController(fakePlatform(), async () => ({ pageCount: 1 }));
  await reader.open(remoteFile(URL_A));
  reader.setCloseGuard(async () => leave);
  return reader;
}

const settled = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("watchFragment and the close guard", () => {
  it("keeps the document and puts the link back when the guard says no to the address losing #url=", async () => {
    const reader = await holding(false);
    const { win, go, hash } = fakeWindow(LINKED);
    const stop = watchFragment(win, fakeEngine(reader));
    go("");
    await settled();
    expect(reader.getState().phase).toBe("open");
    expect(hash()).toBe(LINKED);
    stop();
  });

  it("closes when the guard lets the document go, and leaves the address without the link", async () => {
    const reader = await holding(true);
    const { win, go, hash } = fakeWindow(LINKED);
    const stop = watchFragment(win, fakeEngine(reader));
    go("");
    await settled();
    expect(reader.getState().phase).toBe("empty");
    expect(hash()).toBe("");
    stop();
  });

  it("shows no link problem and names the open file again when the guard keeps it from an unusable link", async () => {
    const reader = await holding(false);
    const problems: string[] = [];
    const { win, go, hash } = fakeWindow(LINKED);
    const stop = watchFragment(win, fakeEngine(reader, problems));
    go("#url=ftp%3A%2F%2Fexample.com%2Fb.pdf");
    await settled();
    expect(problems).toEqual([]);
    expect(reader.getState().phase).toBe("open");
    expect(hash()).toBe(LINKED);
    stop();
  });

  it("shows the unusable link's problem once the guard lets the document go", async () => {
    const reader = await holding(true);
    const problems: string[] = [];
    const { win, go } = fakeWindow(LINKED);
    const stop = watchFragment(win, fakeEngine(reader, problems));
    go("#url=ftp%3A%2F%2Fexample.com%2Fb.pdf");
    await settled();
    expect(problems).toHaveLength(1);
    expect(reader.getState().phase).toBe("empty");
    stop();
  });
});
