// How dense the controls are. Desktop gives 24 px minimum targets; touch gives 44 px targets and
// 16 px text in fields (design language §8.7). Without a provider, a coarse pointer picks touch
// through CSS alone; a provider sets the density for everything inside it.

import { createContext, type ReactNode, useContext, useLayoutEffect, useRef } from "react";
import { suppressMotion } from "./motion.js";

/** A control density: desktop (fine pointer) or touch (coarse pointer). */
export type Density = "desktop" | "touch";

const DensityContext = createContext<Density | undefined>(undefined);

/** Props of `DensityProvider`. */
export interface DensityProviderProps {
  /** The density for every control inside. */
  density: Density;
  /** The controls. */
  children: ReactNode;
}

/** Sets the density for the controls inside it, whatever the pointer. */
export function DensityProvider({ density, children }: DensityProviderProps): ReactNode {
  // A change of density is a switch of look: nothing transitions across it (motion.ts).
  const shown = useRef(density);
  useLayoutEffect(() => {
    if (shown.current === density) return;
    shown.current = density;
    suppressMotion();
  }, [density]);
  return (
    <DensityContext value={density}>
      <div className="paper-density" data-density={density}>
        {children}
      </div>
    </DensityContext>
  );
}

/** The density a provider set, or undefined when the pointer decides. */
export function useDensity(): Density | undefined {
  return useContext(DensityContext);
}
