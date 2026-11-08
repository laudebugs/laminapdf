// Compiles only if the real types still fit, which the package's tests (a fake runtime, its own
// declared types) can't show (README.md has the command):
// - onnxruntime-web's own module fits @laminapdf/layout-heron's `OnnxRuntime`, the way the
//   package's README passes it;
// - the region and model types the package declares fit @laminapdf/sdk's.
import * as ort from "onnxruntime-web";
import { HERON, HeronLayout } from "../../packages/layout-heron/src/index.ts";
import type { LayoutRegion, ModelInfo } from "../../packages/sdk/src/index.ts";

export async function load(weights: Uint8Array): Promise<HeronLayout> {
  return HeronLayout.load(weights, ort);
}

export const info: ModelInfo = HERON;

export async function regions(heron: HeronLayout, page: Parameters<HeronLayout["detect"]>[0]): Promise<LayoutRegion[]> {
  return heron.detect(page);
}
