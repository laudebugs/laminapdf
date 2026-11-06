// The worker the browser tests start: the package's protocol, plus two hooks and a stalling fetch
// the tests use. None of them ship: the package's worker (src/worker/entry.ts) is `serve()` alone.
// A worker named `budget-<n>` saves in steps of <n> work units, so a small save takes many steps.
import { emit, serve, workerStats } from "@laminapdf/sdk/worker/server";
import { crashMethod } from "@laminapdf/test-utils/worker-hooks";

const budget = /^budget-(\d+)$/.exec(self.name)?.[1];

// A fetch of a URL ending in `#hold` stalls like a server that never answers: it emits
// `fetch-held`, then rejects with its signal's reason once that aborts, as fetch does. A test
// that aborts on the event knows its abort lands while the worker is inside the fetch.
const fetchNow = globalThis.fetch;
globalThis.fetch = (input, init) => {
  const signal = init?.signal;
  if (!(typeof input === "string" && input.endsWith("#hold")) || !signal) return fetchNow(input, init);
  emit("fetch-held");
  return new Promise<Response>((_, reject) => {
    const abort = () => reject(signal.reason);
    if (signal.aborted) abort();
    else signal.addEventListener("abort", abort, { once: true });
  });
};

serve({
  methods: {
    __stats: { run: () => workerStats() },
    __crash: crashMethod,
  },
  ...(budget === undefined ? {} : { saveBudget: Number(budget) }),
});
