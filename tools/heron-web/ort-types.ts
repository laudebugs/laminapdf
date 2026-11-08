// Compiles only if onnxruntime-web's own module fits @laminapdf/layout-heron's `OnnxRuntime`, the
// way the package's README passes it. The package's tests use a fake runtime, so this is what
// catches a mismatch with the real types (README.md has the command).
import * as ort from "onnxruntime-web";
import { HeronLayout } from "../../packages/layout-heron/src/index.ts";

export async function load(weights: Uint8Array): Promise<HeronLayout> {
  return HeronLayout.load(weights, ort);
}
