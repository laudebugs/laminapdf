// The one Platform a shell creates at startup, handed to every component below it (ADR 0010 §1).

import { createContext, type ReactNode, useContext } from "react";
import type { Capability, Platform } from "./types.js";

const PlatformContext = createContext<Platform | undefined>(undefined);

/** Makes `platform` the one every component below uses. */
export function PlatformProvider({ platform, children }: { platform: Platform; children: ReactNode }): ReactNode {
  return <PlatformContext.Provider value={platform}>{children}</PlatformContext.Provider>;
}

/** The platform the nearest `PlatformProvider` holds; throws outside one, a wiring mistake. */
export function usePlatform(): Platform {
  const platform = useContext(PlatformContext);
  if (platform === undefined) throw new Error("usePlatform() needs a PlatformProvider above it");
  return platform;
}

/**
 * How a control gated by `flag` should appear (ADR 0010 rule 2): available; disabled, with why and
 * how to enable it, when the user could change that; or hidden when this platform never can.
 */
export type CapabilityState =
  | { readonly state: "available" }
  | { readonly state: "disabled"; readonly reason: string }
  | { readonly state: "hidden" };

/** The state of a control gated by `flag`. */
export function useCapability(flag: Capability): CapabilityState {
  return capabilityState(usePlatform(), flag);
}

/** The state of a control gated by `flag` on `platform`. */
export function capabilityState(platform: Platform, flag: Capability): CapabilityState {
  if (platform.capabilities[flag]) return { state: "available" };
  const reason = platform.howToEnable(flag);
  return reason === undefined ? { state: "hidden" } : { state: "disabled", reason };
}
