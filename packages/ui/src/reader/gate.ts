// Menu items gated by a capability (ADR 0010 rule 2): there when the platform can do it, disabled
// with how to enable it when the user could change that, and left out when this platform never can.

import type { MenuEntry } from "../overlays/popover.js";
import { capabilityState, usePlatform } from "../platform/context.js";
import type { Capability } from "../platform/types.js";

/** A menu entry, and the capability it needs when it needs one. */
export type GatedEntry<Id extends string> = MenuEntry<Id> & { readonly needs?: Capability };

/** `entries` as this platform allows them: kept, disabled with the reason, or left out. */
export function useGatedEntries<Id extends string>(entries: readonly GatedEntry<Id>[]): MenuEntry<Id>[] {
  const platform = usePlatform();
  return entries.flatMap(({ needs, ...entry }) => {
    if (needs === undefined) return [entry];
    const gate = capabilityState(platform, needs);
    if (gate.state === "available") return [entry];
    return gate.state === "hidden" ? [] : [{ ...entry, isDisabled: true, disabledReason: gate.reason }];
  });
}
