import { describe, expect, it, vi } from "vitest";
import { LayerStack } from "../src/overlays/layer-stack.ts";

const layer = (dismissable = true) => ({ dismiss: vi.fn(), dismissable });

describe("LayerStack", () => {
  it("closes only the innermost layer on Escape", () => {
    const stack = new LayerStack();
    const outer = layer();
    const inner = layer();
    stack.open(outer);
    stack.open(inner);
    expect(stack.isTopmost(inner)).toBe(true);
    expect(stack.escape()).toBe(true);
    expect(inner.dismiss).toHaveBeenCalledOnce();
    expect(outer.dismiss).not.toHaveBeenCalled();
  });

  it("stops Escape at a required question, so nothing beneath it closes", () => {
    const stack = new LayerStack();
    const below = layer();
    const required = layer(false);
    stack.open(below);
    stack.open(required);
    expect(stack.escape()).toBe(true);
    expect(required.dismiss).not.toHaveBeenCalled();
    expect(below.dismiss).not.toHaveBeenCalled();
  });

  it("removes a layer wherever it sits, and reports an empty stack", () => {
    const stack = new LayerStack();
    const first = layer();
    const second = layer();
    const closeFirst = stack.open(first);
    const closeSecond = stack.open(second);
    closeFirst();
    expect(stack.size).toBe(1);
    expect(stack.isTopmost(second)).toBe(true);
    closeSecond();
    closeSecond();
    expect(stack.size).toBe(0);
    expect(stack.escape()).toBe(false);
  });
});
