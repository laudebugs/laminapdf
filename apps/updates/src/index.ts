// The Worker's entry: binds the handler to the R2 buckets, the Cache API and Workers Logs. The only
// file that uses the Workers runtime's own types (src/generated/worker-configuration.d.ts, from
// `pnpm run types`); the rest runs in Node's tests as well.
import { type HandlerDeps, handle } from "./handler.ts";

export default {
  fetch(request, env, ctx): Promise<Response> {
    const deps: HandlerDeps = {
      stores: { stable: env.RELEASES, beta: env.RELEASES, nightly: env.NIGHTLY, staging: env.STAGING },
      cache: caches.default,
      now: Date.now,
      waitUntil: (work) => ctx.waitUntil(work),
      // One JSON line per event for Workers Logs (ADR 0014 §9); events carry no request header.
      log: (entry) => console.error(JSON.stringify(entry)),
    };
    return handle(request, deps);
  },
} satisfies ExportedHandler<Env>;
