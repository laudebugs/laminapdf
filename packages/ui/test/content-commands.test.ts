import { describe, expect, it } from "vitest";
import { delegatedCommand } from "../src/commands/delegated.ts";
import { laterMessage } from "../src/later-messages.ts";
import { readerCommandsFor } from "./commands.tsx";
import { FakeEdits } from "./fakes.ts";

const WORDS = { noSession: "content.unavailable", notInMode: "content.inEdit" } as const;

describe("a delegated command", () => {
  const command = delegatedCommand("tool.add-text", { ...WORDS, keywords: ["text"], home: ["content-toolbar"] });
  const readOnly = delegatedCommand("tool.select", {
    ...WORDS,
    keywords: [],
    home: ["content-toolbar"],
    readOnly: true,
  });
  const enabled = (c: typeof command, ctx: Parameters<NonNullable<typeof command.enabled>>[0]) => c.enabled?.(ctx);

  it("says there is no document before anything else", () => {
    expect(enabled(command, { platform: readerCommandsFor().context.platform })).toBe(
      laterMessage("content.unavailable"),
    );
  });

  it("says its mode isn't on until the mode gives it a handler", () => {
    const { context } = readerCommandsFor({ edits: new FakeEdits() });
    expect(enabled(command, context)).toBe(laterMessage("content.inEdit"));
    expect(enabled(command, { ...context, actions: { "tool.add-text": () => undefined } })).toBe(true);
  });

  it("is refused by a blocked session unless it only reads", () => {
    const edits = new FakeEdits([], 0, { blocked: "encrypted" });
    const { context } = readerCommandsFor({ edits });
    const actions = { "tool.add-text": () => undefined, "tool.select": () => undefined };
    expect(enabled(command, { ...context, actions })).not.toBe(true);
    expect(enabled(readOnly, { ...context, actions })).toBe(true);
  });

  it("runs the handler its mode registered, with the arguments", () => {
    const seen: unknown[] = [];
    const { context } = readerCommandsFor({ edits: new FakeEdits() });
    command.run({ ...context, actions: { "tool.add-text": (args) => void seen.push(args) } }, undefined as never);
    expect(seen).toEqual([undefined]);
  });
});

describe("Edit mode's commands", () => {
  it("are left out until Edit mode gives them a handler", () => {
    const base = readerCommandsFor({ edits: new FakeEdits(), content: true });
    // A delegated command with no handler is left out of the views, whose own `enabled` gives the reason.
    expect(base.registry.availability("tool.image", base.context)).toEqual({ state: "hidden" });
    const on = readerCommandsFor({
      edits: new FakeEdits(),
      content: true,
      actions: { "tool.image": () => undefined },
    });
    expect(on.registry.availability("tool.image", on.context)).toEqual({ state: "available" });
  });
});
