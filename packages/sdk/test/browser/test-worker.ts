// The worker the browser tests start: the package's protocol, plus two hooks the tests use. None
// of them ship: the package's worker (src/worker/entry.ts) is `serve()` alone. A worker named
// `budget-<n>` saves in steps of <n> work units, so a small save takes many steps.
import { serve, workerStats } from "@laminapdf/sdk/worker/server";

const budget = /^budget-(\d+)$/.exec(self.name)?.[1];

serve({
  methods: {
    __stats: { run: () => workerStats() },
    // An uncaught error: the page sees the worker's `error` event, a crash.
    __crash: {
      run: () => {
        setTimeout(() => {
          throw new Error("crash requested by a test");
        });
        return new Promise(() => {});
      },
    },
  },
  ...(budget === undefined ? {} : { saveBudget: Number(budget) }),
});
