import { describe, expect, it } from "vitest";
import { desktopCommands } from "../src/commands/desktop-commands.ts";
import { editCommands } from "../src/commands/edit-commands.ts";
import { readerCommands } from "../src/commands/reader-commands.ts";
import { CommandRegistry } from "../src/commands/registry.ts";
import { shortcutProblems } from "../src/commands/shortcuts.ts";
import type { CommandContext } from "../src/commands/types.ts";
import { commentCommands, registerCommentCommands } from "../src/comment/commands.ts";
import { FakeEdits, fakePlatform } from "./fakes.ts";

const context = (actions: NonNullable<CommandContext["actions"]>, edits?: FakeEdits): CommandContext => ({
  platform: fakePlatform(),
  ...(edits === undefined ? {} : { edits }),
  actions,
});

describe("Comment mode's commands", () => {
  it("each have a visible home besides the palette", () => {
    expect(commentCommands().filter((c) => c.home.length === 0)).toEqual([]);
  });

  it("give Highlight, Underline, Text box and Ink single keys on the page, and bold and italic ⌘B and ⌘I, that no other command takes", () => {
    const keyed = commentCommands().flatMap((c) => (c.shortcuts ?? []).map((b) => `${c.id} ${b.mac} ${b.scope}`));
    expect(keyed).toEqual([
      "comment.highlight H page",
      "comment.underline U page",
      "comment.ink P page",
      "comment.text-box T page",
      "annot.toggle-bold Mod+B global",
      "annot.toggle-italic Mod+I global",
    ]);
    const all = [...readerCommands(), ...editCommands(), ...desktopCommands(), ...commentCommands()];
    expect([...shortcutProblems(all, "web"), ...shortcutProblems(all, "desktop")]).toEqual([]);
  });

  it("say why a tool can't run: no document, a blocked session, or Comment not open", () => {
    const registry = new CommandRegistry();
    registerCommentCommands(registry);
    const run = () => undefined;
    expect([
      registry.availability("comment.highlight", context({ "comment.highlight": run })),
      registry.availability("comment.highlight", context({}, new FakeEdits())),
      registry.availability("comment.highlight", context({ "comment.highlight": run }, new FakeEdits())),
    ]).toEqual([
      { state: "disabled", reason: "Open a document to comment on it" },
      { state: "hidden" },
      { state: "available" },
    ]);
  });

  it("run their tool with the shape chosen", () => {
    const registry = new CommandRegistry();
    registerCommentCommands(registry);
    const chosen: unknown[] = [];
    const ctx = context({ "comment.shape": (args) => void chosen.push(args) }, new FakeEdits());
    void registry.run("comment.shape", ctx, { args: "arrow" });
    expect(chosen).toEqual(["arrow"]);
  });
});
