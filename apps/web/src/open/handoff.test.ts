import { describe, expect, it } from "vitest";
import { type HandoffChannel, type HandoffHost, handoffToken, offer, receive } from "./handoff.ts";

/** Channels on one bus: what one posts, the others hear, as BroadcastChannels of one name do. */
function bus(): HandoffHost & { readonly pending: (() => void)[] } {
  const channels = new Set<{ listener?: (event: MessageEvent) => void }>();
  const pending: (() => void)[] = [];
  return {
    pending,
    channel(): HandoffChannel {
      const self: { listener?: (event: MessageEvent) => void } = {};
      channels.add(self);
      return {
        postMessage: (message) => {
          for (const other of channels) if (other !== self) other.listener?.({ data: message } as MessageEvent);
        },
        addEventListener: (_type, listener) => {
          self.listener = listener;
        },
        close: () => void channels.delete(self),
      };
    },
    // Timers are run by hand, so a test says when the receiver asks again.
    setTimeout: (run) => pending.push(run),
    clearTimeout: () => undefined,
  };
}

describe("the hand-off to a new tab", () => {
  it("reads its token from the address fragment", () => {
    expect(handoffToken("#handoff=abc-123")).toBe("abc-123");
    expect(handoffToken("#page=2&handoff=abc")).toBe("abc");
    expect(handoffToken("#url=x")).toBeUndefined();
    expect(handoffToken("#handoff=")).toBeUndefined();
  });

  it("gives the file to a tab that asked after it was made", () => {
    const host = bus();
    const sent = offer(host, "t1");
    sent.deliver("Report – comments.pdf", new Uint8Array([1, 2]));
    const got: string[] = [];
    receive(
      "t1",
      { open: (name, bytes) => void got.push(`${name} ${bytes.length}`), giveUp: () => got.push("gave up") },
      host,
    );
    expect(got).toEqual(["Report – comments.pdf 2"]);
  });

  it("gives the file to a tab that asked before it was made, when it asks again", () => {
    const host = bus();
    const sent = offer(host, "t2");
    const got: string[] = [];
    receive("t2", { open: (name) => void got.push(name), giveUp: () => got.push("gave up") }, host);
    sent.deliver("a.pdf", new Uint8Array());
    expect(got).toEqual(["a.pdf"]);
  });

  it("closes the tab when the summary is cancelled", () => {
    const host = bus();
    const sent = offer(host, "t3");
    const got: string[] = [];
    receive("t3", { open: () => void got.push("opened"), giveUp: () => got.push("gave up") }, host);
    sent.cancel();
    expect(got).toEqual(["gave up"]);
  });

  it("tells a tab that asks after the summary was cancelled to close, instead of leaving it waiting", () => {
    const host = bus();
    const sent = offer(host, "t4");
    sent.cancel();
    const got: string[] = [];
    receive("t4", { open: () => void got.push("opened"), giveUp: () => got.push("gave up") }, host);
    expect(got).toEqual(["gave up"]);
  });

  it("ignores another hand-off's file", () => {
    const host = bus();
    const sent = offer(host, "other");
    sent.deliver("x.pdf", new Uint8Array());
    const got: string[] = [];
    receive("mine", { open: () => void got.push("opened"), giveUp: () => got.push("gave up") }, host);
    expect(got).toEqual([]);
  });
});
