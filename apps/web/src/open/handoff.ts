// A new document handed to another tab of the app (the comment summary opens in a new tab,
// as C07 asks, where the platform has no windows of its own). The opening tab names the hand-off
// with a token in the new tab's address (`#handoff=<token>`) and keeps the file in memory; the
// new tab says it is ready over a same-origin BroadcastChannel, and gets the file. Nothing is
// stored and nothing leaves the browser. The bytes are never in the address, and the fragment
// isn't sent anywhere (ADR 0011 §3).

/** The channel both tabs use. */
const CHANNEL = "lamina-handoff";

/** How long a tab waits for the other one: the opener for a tab to ask, the receiver for the file. */
const WAIT_MS = 5 * 60_000;

/** How long the opener goes on answering a tab that asks after the summary was cancelled. */
const CANCELLED_WAIT_MS = 30_000;

/** What the tabs say to each other. */
type Message =
  | { readonly type: "ready"; readonly token: string }
  | { readonly type: "file"; readonly token: string; readonly name: string; readonly bytes: Uint8Array }
  | { readonly type: "cancel"; readonly token: string };

/** The part of `BroadcastChannel` the hand-off uses, so a test can stand in. */
export interface HandoffChannel {
  postMessage(message: Message): void;
  addEventListener(type: "message", listener: (event: MessageEvent<Message>) => void): void;
  close(): void;
}

/** The pieces the hand-off needs from the page. */
export interface HandoffHost {
  channel(): HandoffChannel;
  setTimeout: (run: () => void, ms: number) => unknown;
  clearTimeout: (timer: unknown) => void;
}

/** The page's own host. */
const PAGE_HOST: HandoffHost = {
  channel: () => new BroadcastChannel(CHANNEL) as unknown as HandoffChannel,
  setTimeout: (run, ms) => setTimeout(run, ms),
  clearTimeout: (timer) => clearTimeout(timer as ReturnType<typeof setTimeout>),
};

/** The fragment key a new tab is opened with. */
export const HANDOFF_KEY = "handoff";

/** The token in `hash`, when it names a hand-off. */
export function handoffToken(hash: string): string | undefined {
  const found = hash
    .replace(/^#/, "")
    .split("&")
    .find((pair) => pair.startsWith(`${HANDOFF_KEY}=`));
  const token = found?.slice(HANDOFF_KEY.length + 1);
  return token === undefined || token === "" ? undefined : token;
}

/** What the opening tab holds while it waits for the new tab. */
export interface Offer {
  /** The address to open the new tab at. */
  readonly token: string;
  /** Gives the file to the tab, once it asks and once there is one; false if it gave up first. */
  deliver(name: string, bytes: Uint8Array): void;
  /** Tells the tab there will be no file (the summary was cancelled or failed): it closes. */
  cancel(): void;
}

/** Starts an offer: the new tab is opened with `token`, and the file follows when it is made. */
export function offer(host: HandoffHost = PAGE_HOST, token: string = crypto.randomUUID()): Offer {
  const channel = host.channel();
  let file: { readonly name: string; readonly bytes: Uint8Array } | undefined;
  // Whether the new tab has asked: a file posted before it listens would be lost.
  let asked = false;
  let cancelled = false;
  let done = false;
  let timer: unknown;
  const finish = () => {
    if (done) return;
    done = true;
    host.clearTimeout(timer);
    channel.close();
  };
  timer = host.setTimeout(finish, WAIT_MS);
  const send = () => {
    if (file === undefined || !asked || done) return;
    channel.postMessage({ type: "file", token, ...file });
    finish();
  };
  channel.addEventListener("message", (event) => {
    if (event.data.type !== "ready" || event.data.token !== token || done) return;
    asked = true;
    // A tab that asks after a cancel is told so, and closes, instead of waiting for a file.
    if (cancelled) channel.postMessage({ type: "cancel", token });
    else send();
  });
  return {
    token,
    deliver(name, bytes) {
      file = { name, bytes };
      // A tab that asked before the file was made is waiting: it gets it now.
      send();
    },
    cancel() {
      if (done || cancelled) return;
      cancelled = true;
      channel.postMessage({ type: "cancel", token });
      // The tab may not be listening yet: keep answering it for a while.
      host.clearTimeout(timer);
      timer = host.setTimeout(finish, CANCELLED_WAIT_MS);
    },
  };
}

/** What a new tab does with what it is given. */
export interface Receiver {
  /** Opens `bytes` as a new document named `name`. */
  open(name: string, bytes: Uint8Array): void | Promise<void>;
  /** Nothing is coming (cancelled, or nobody answered): closes the tab. */
  giveUp(): void;
}

/**
 * The new tab's side: asks for the file named by `token` until it comes, hands it to `receiver`,
 * and gives up when the opener cancels or doesn't answer. Returns the function that stops it.
 */
export function receive(token: string, receiver: Receiver, host: HandoffHost = PAGE_HOST): () => void {
  const channel = host.channel();
  let done = false;
  const finish = () => {
    if (done) return false;
    done = true;
    host.clearTimeout(timer);
    host.clearTimeout(asking);
    channel.close();
    return true;
  };
  const timer = host.setTimeout(() => finish() && receiver.giveUp(), WAIT_MS);
  // The opener may not be listening yet, or may still be making the file: ask again until answered.
  let asking: unknown;
  const ask = () => {
    if (done) return;
    channel.postMessage({ type: "ready", token });
    asking = host.setTimeout(ask, 500);
  };
  channel.addEventListener("message", (event) => {
    const message = event.data;
    if (message.token !== token) return;
    if (message.type === "file" && finish()) void receiver.open(message.name, message.bytes);
    else if (message.type === "cancel" && finish()) receiver.giveUp();
  });
  ask();
  return () => void finish();
}
