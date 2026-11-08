import { defineConfig } from "vitest/config";

// The resize, the label map and the box filter in Node, with a fake ONNX Runtime; the real model
// runs in tools/heron-web. A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"] },
});
