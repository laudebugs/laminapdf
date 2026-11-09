// The document worker's network log (ADR 0011, R13). Imported first by entry.ts, so the wrappers
// are in place before any other module of the worker can capture an unwrapped `fetch`. Every
// request and every blocked request is kept (the page asks for them all with `networkLog`, so
// nothing sent before the page listened is lost) and posted as it happens (`network` and
// `csp-violation` events). Each event carries its index in the log, so the page can merge the two
// without counting a request twice.

import { emit } from "@laminapdf/sdk/worker/server";
import { watchNetwork } from "../network/watch.js";
import type { NetworkRecord, PolicyViolation, WorkerNetworkLog } from "../protocol.js";

const worker = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
const requests: NetworkRecord[] = [];
const violations: PolicyViolation[] = [];

watchNetwork(
  globalThis,
  {
    request(record) {
      emit("network", { worker, index: requests.push(record) - 1, record });
    },
    violation(violation) {
      emit("csp-violation", { worker, index: violations.push(violation) - 1, violation });
    },
  },
  "worker",
);

/** Everything the worker has requested, and had blocked, since it started. */
export function networkLog(): WorkerNetworkLog {
  return { worker, requests: [...requests], violations: [...violations] };
}
