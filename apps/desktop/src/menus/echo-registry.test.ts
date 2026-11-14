import { type AnyCommand, type CommandContext, createWebPlatform } from "@laminapdf/ui";
import { describe, expect, it, vi } from "vitest";
import { createDesktopPlatform } from "../platform/desktop.ts";
import { EchoRegistry } from "./echo-registry.ts";

const ctx: CommandContext = { platform: createDesktopPlatform(createWebPlatform()) };

function registry(run: () => void): EchoRegistry {
  const commands = new EchoRegistry();
  const command: AnyCommand = { id: "test.do-thing", title: "Do the thing", group: "app", home: ["native-menu"], run };
  commands.register(command);
  return commands;
}

describe("EchoRegistry", () => {
  it("runs a command once when one key press reaches it from the keys and the native menu", async () => {
    vi.useFakeTimers();
    try {
      const run = vi.fn();
      const commands = registry(run);
      await commands.run("test.do-thing", ctx, { source: "keys" });
      await commands.run("test.do-thing", ctx, { source: "menu" });
      expect(run).toHaveBeenCalledTimes(1);
      // The same source twice is two presses; the other source later is another choice.
      await commands.run("test.do-thing", ctx, { source: "menu" });
      vi.advanceTimersByTime(400);
      await commands.run("test.do-thing", ctx, { source: "keys" });
      expect(run).toHaveBeenCalledTimes(3);
      // Two quick presses, each echoed by the menu: two runs.
      vi.advanceTimersByTime(400);
      await commands.run("test.do-thing", ctx, { source: "keys" });
      await commands.run("test.do-thing", ctx, { source: "menu" });
      vi.advanceTimersByTime(150);
      await commands.run("test.do-thing", ctx, { source: "keys" });
      await commands.run("test.do-thing", ctx, { source: "menu" });
      expect(run).toHaveBeenCalledTimes(5);
    } finally {
      vi.useRealTimers();
    }
  });

  it("runs a run without a source as the registry does", async () => {
    const run = vi.fn();
    const commands = registry(run);
    await commands.run("test.do-thing", ctx);
    await commands.run("test.do-thing", ctx);
    expect(run).toHaveBeenCalledTimes(2);
  });
});
