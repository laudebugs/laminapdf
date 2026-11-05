// A menu item gated by a capability (ADR 0010 rule 2): there when the platform can do it, disabled
// with how to enable it when the user could change that, and left out when this platform never can.

import type { MenuEntry } from "../overlays/popover.js";
import { useCapability } from "../platform/context.js";
import type { Capability } from "../platform/types.js";

/** `entry` as `flag` allows it: itself, disabled with the reason, or undefined to leave it out. */
export function useGatedEntry<Id extends string>(flag: Capability, entry: MenuEntry<Id>): MenuEntry<Id> | undefined {
  const gate = useCapability(flag);
  if (gate.state === "available") return entry;
  if (gate.state === "hidden") return undefined;
  return { ...entry, isDisabled: true, disabledReason: gate.reason };
}
