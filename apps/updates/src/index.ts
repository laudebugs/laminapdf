// The Worker's entry: binds the handler to the R2 buckets, the Cache API and Workers Logs. The only
// file that uses the Workers runtime's own types (src/generated/worker-configuration.d.ts, from
// `pnpm run types`); the rest runs in Node's tests as well.
import { CHANNEL_BUCKETS, type Channel } from "./channels.ts";
import { type HandlerDeps, handle, logEvent } from "./handler.ts";

/** Each channel's bucket binding, from the one map in channels.ts. */
function storesOf(env: Env): HandlerDeps["stores"] {
  const of = (channel: Channel) => env[CHANNEL_BUCKETS[channel].binding];
  // An object literal over Record<Channel, …>: the compiler fails if a channel is missing.
  return { stable: of("stable"), beta: of("beta"), nightly: of("nightly"), staging: of("staging") };
}

/** The Worker: every request goes to {@link handle} with the runtime's buckets, cache and logger. */
export default {
  fetch(request, env, ctx): Promise<Response> {
    const deps: HandlerDeps = {
      stores: storesOf(env),
      cache: caches.default,
      now: Date.now,
      waitUntil: (work) => ctx.waitUntil(work),
      // One JSON line per event for Workers Logs (ADR 0014 §9); events carry no request header.
      log: logEvent,
    };
    return handle(request, deps);
  },
} satisfies ExportedHandler<Env>;
