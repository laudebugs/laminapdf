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

describe("LayerStack order", () => {
  it("knows whether a layer opened from another is open, through any depth", () => {
    const stack = new LayerStack();
    const sheet = { dismiss: vi.fn(), dismissable: true };
    const menu = { dismiss: vi.fn(), dismissable: true, parent: sheet };
    const submenu = { dismiss: vi.fn(), dismissable: true, parent: menu };
    const toolbar = { dismiss: vi.fn(), dismissable: true };
    stack.open(sheet);
    stack.open(toolbar);
    expect(stack.hasOpenedFrom(sheet)).toBe(false);
    const closeMenu = stack.open(menu);
    const closeSubmenu = stack.open(submenu);
    closeMenu();
    expect(stack.hasOpenedFrom(sheet)).toBe(true);
    closeSubmenu();
    expect(stack.hasOpenedFrom(sheet)).toBe(false);
  });

  it("keeps a layer beneath the layers opened from it, whichever registers first", () => {
    const stack = new LayerStack();
    const dialog = { dismiss: vi.fn(), dismissable: true };
    const menu = { dismiss: vi.fn(), dismissable: true, parent: dialog };
    stack.open(menu);
    stack.open(dialog);
    expect(stack.isTopmost(menu)).toBe(true);
    stack.escape();
    expect(menu.dismiss).toHaveBeenCalledOnce();
    expect(dialog.dismiss).not.toHaveBeenCalled();
  });

  it("reads whether a layer may be dismissed when Escape comes, not when it opened", () => {
    const stack = new LayerStack();
    let dismissable = true;
    const dialog = {
      dismiss: vi.fn(),
      get dismissable() {
        return dismissable;
      },
    };
    stack.open(dialog);
    dismissable = false;
    stack.escape();
    expect(dialog.dismiss).not.toHaveBeenCalled();
  });
});
