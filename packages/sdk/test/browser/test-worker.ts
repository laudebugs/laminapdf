// The worker the browser tests start: the package's protocol, plus two hooks the tests use. None
// of them ship: the package's worker (src/worker/entry.ts) is `serve()` alone. A worker named
// `budget-<n>` saves in steps of <n> work units, so a small save takes many steps.
import { serve, workerStats } from "@laminapdf/sdk/worker/server";
import { crashMethod } from "@laminapdf/test-utils/worker-hooks";

const budget = /^budget-(\d+)$/.exec(self.name)?.[1];

serve({
  methods: {
    __stats: { run: () => workerStats() },
    __crash: crashMethod,
  },
  ...(budget === undefined ? {} : { saveBudget: Number(budget) }),
});
