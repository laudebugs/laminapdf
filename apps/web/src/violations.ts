// What a save would break, as the dialogs and banners say it: the signatures a change's
// violations break, and the restrictions behind them, each once (Organise's refusals, Comment
// mode's banner).

import type { Violation } from "@laminapdf/sdk";
import type { RestrictionKind, SaveSignature, SignatureRestriction } from "@laminapdf/ui/editing";
import { signature } from "./save-flow.ts";

/** `violations` mapped by `map`, each key `keyOf` gives once, in order. */
function onceEach<T>(violations: readonly Violation[], keyOf: (v: Violation) => string, map: (v: Violation) => T): T[] {
  const seen = new Set<string>();
  return violations.flatMap((v) => {
    const key = keyOf(v);
    if (seen.has(key)) return [];
    seen.add(key);
    return [map(v)];
  });
}

/** A signature's identity: its kind and field. */
const signatureKey = ({ kind, field }: Violation["signature"]) => `${kind}:${field ?? ""}`;

/** Each signature `violations` break, once. */
export function brokenSignatures(violations: readonly Violation[]): SaveSignature[] {
  return onceEach(
    violations,
    (v) => signatureKey(v.signature),
    (v) => signature(v.signature),
  );
}

/** Each restriction `violations` break, once, for Details. */
export function restrictionsOf(violations: readonly Violation[]): SignatureRestriction[] {
  return onceEach(
    violations,
    (v) => `${v.restriction}:${signatureKey(v.signature)}`,
    ({ restriction, permission, signature: { revision } }) => ({
      // The WASM type says string; the planner names only these five.
      restriction: restriction as RestrictionKind,
      ...(permission === undefined ? {} : { permission }),
      ...(revision === undefined ? {} : { revision }),
    }),
  );
}
