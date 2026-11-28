import type { ContentTextRun, ObjectRef, Op, TextEditReport } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { TextEditor } from "./text-edit.ts";

/** An error as the engine throws one: its code, and a message that says more. */
function errorOf(code: string, message = code): Error {
  return Object.assign(new Error(message), { code });
}

/** The arguments of the first op of `step`. */
function argsOf(step: { ops: Op[] } | undefined): Record<string, unknown> {
  return ((step?.ops[0] ?? { args: {} }) as unknown as { args: Record<string, unknown> }).args;
}

interface Line {
  text: string;
  box: [number, number, number, number];
  readOnly?: string;
}

/** A page of lines of text whose names change with every edit that touches it, as the engine's do. */
function world(lines: Line[] = []) {
  const page: Line[] =
    lines.length > 0
      ? lines
      : [
          { text: "Hello, world!", box: [72, 100, 200, 112] },
          { text: "A second line", box: [72, 130, 190, 142] },
        ];
  let version = 0;
  const steps: { label: string; ops: Op[] }[] = [];
  const runs = (): ContentTextRun[] =>
    page.map(
      (line, ordinal) =>
        ({
          run: { object: { ordinal, hash: `v${version}` } as unknown as ObjectRef, start: 0, end: line.text.length },
          text: line.text,
          bounds: line.box,
          fontName: "Helvetica",
          fontSize: 12,
          line: ordinal,
          readOnly: line.readOnly,
        }) as unknown as ContentTextRun,
    );
  let editor: TextEditor;
  const report = (substitutions: object[]) =>
    ({
      fidelity: substitutions.length > 0 ? "substituted" : "exact",
      substitutions,
      grewBy: 0,
    }) as unknown as TextEditReport;
  const apply = async (ops: readonly Op[], label: string, pages: readonly number[]) => {
    for (const op of ops) {
      const args = (op as unknown as { args: { run: { object: ObjectRef }; text?: string } }).args;
      if (op.op === "editText") {
        if (args.run.object.hash !== `v${version}`) throw errorOf("content-object-stale");
        const line = page[args.run.object.ordinal];
        if (line !== undefined) line.text = args.text ?? "";
      }
    }
    steps.push({ label, ops: [...ops] });
    version += 1;
    await editor.read(pages);
    return true;
  };
  let chain: Promise<unknown> = Promise.resolve();
  const said: string[] = [];
  const focus = { count: 0 };
  editor = new TextEditor({
    textRuns: async () => runs(),
    plan: async (_page, _run, text, fonts) => {
      if (text.includes("你")) throw errorOf("missing-glyph", "the font has no glyph for U+4F60");
      if (text.includes("č")) {
        if (fonts === undefined) throw errorOf("missing-glyph", "the font has no glyph for U+010D");
        return report([{ text: "č", font: "Times-Roman", kind: "substitute" }]);
      }
      return report([]);
    },
    chain: async (primary, text) => ({ fonts: [primary, "times"], missing: [...text].filter((c) => c === "你") }),
    pageId: (p) => `p${p}`,
    apply,
    applyNow: apply,
    enqueue: <T>(work: () => Promise<T>) => {
      const run = chain.then(work);
      chain = run.catch(() => undefined);
      return run;
    },
    changed: () => undefined,
    clearOthers: () => undefined,
    focusPage: () => {
      focus.count += 1;
    },
    say: (message) => void said.push(message),
  });
  return { editor, page, steps, said, focus };
}

/** The first run on page 0, read. */
async function first(editor: TextEditor): Promise<ContentTextRun> {
  await editor.read([0]);
  return editor.on(0)[0] as ContentTextRun;
}

describe("typing into text the page shows", () => {
  it("makes one editText op when the line is finished, and the pick follows the renamed run", async () => {
    const { editor, steps, focus } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    expect(editor.typing?.original).toBe("Hello, world!");
    editor.typed("Hello, lamina!");
    expect(await editor.commit("enter")).toBe(true);
    expect(steps).toHaveLength(1);
    expect(steps[0]?.label).toBe("Edit text");
    const args = argsOf(steps[0]);
    expect(args).toMatchObject({ page: "p0", text: "Hello, lamina!" });
    expect(args).not.toHaveProperty("fonts");
    expect(editor.typing).toBeUndefined();
    expect(editor.pick?.runs.map((r) => r.text)).toEqual(["Hello, lamina!"]);
    expect(focus.count).toBeGreaterThan(0);
  });

  it("makes no op, and no step, when nothing changed or when Escape cancels", async () => {
    const { editor, steps } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    expect(await editor.commit("enter")).toBe(true);
    await editor.press(0, { x: 100, y: 106 }, false);
    editor.typed("Something else");
    editor.cancel();
    expect(steps).toHaveLength(0);
    expect(editor.typing).toBeUndefined();
  });

  it("puts the original back with Revert", async () => {
    const { editor } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    const before = editor.typing?.version ?? 0;
    editor.typed("Changed");
    editor.revert();
    expect(editor.typing?.text).toBe("Hello, world!");
    expect(editor.typing?.version).toBeGreaterThan(before);
  });

  it("deleting all the text of a line is an edit with empty text, named Delete text", async () => {
    const { editor, steps } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    editor.typed("");
    expect(await editor.commit("enter")).toBe(true);
    expect(steps[0]?.label).toBe("Delete text");
  });
});

describe("a substitution is shown before it is saved", () => {
  it("stays open on the first Enter with the notice, and saves the same text on the next with the fonts", async () => {
    const { editor, steps, said } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    editor.typed("Čech č");
    expect(await editor.commit("enter")).toBe(false);
    expect(editor.typing?.plan?.kind).toBe("substituted");
    expect(editor.typing?.shown).toBe("Čech č");
    expect(steps).toHaveLength(0);
    expect(await editor.commit("enter")).toBe(true);
    expect(steps).toHaveLength(1);
    expect(argsOf(steps[0]).fonts).toEqual(["helvetica", "times"]);
    expect(said[0]).toContain("č");
    expect(said[0]).toContain("Times-Roman");
  });

  it("is shown while typing, once the plan has run, so Enter saves at once", async () => {
    const { editor, steps } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    editor.typed("č");
    await editor.planNow();
    expect(editor.typing?.shown).toBe("č");
    expect(await editor.commit("enter")).toBe(true);
    expect(steps).toHaveLength(1);
  });

  it("is saved, with a word about it, when the person leaves the box", async () => {
    const { editor, steps, said } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    editor.typed("č");
    expect(await editor.commit("away")).toBe(true);
    expect(steps).toHaveLength(1);
    expect(said).toHaveLength(1);
  });

  it("is not saved at all for a character no font has, and says which", async () => {
    const { editor, steps } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    editor.typed("你好");
    expect(await editor.commit("away")).toBe(false);
    expect(editor.typing?.plan).toEqual({ kind: "missing", characters: ["你"] });
    expect(steps).toHaveLength(0);
  });
});

describe("text that can't be edited in place", () => {
  it("is picked and says why, but takes no typing", async () => {
    const { editor, steps } = world([{ text: "Chart label", box: [10, 10, 80, 22], readOnly: "type3-font" }]);
    await editor.press(0, { x: 20, y: 16 }, false);
    expect(editor.pick?.runs[0]?.readOnly).toBe("type3-font");
    expect(editor.typing).toBeUndefined();
    editor.editPicked();
    expect(editor.typing).toBeUndefined();
    expect(await editor.deleteText()).toBe(false);
    expect(steps).toHaveLength(0);
  });
});

describe("Delete on picked text", () => {
  it("removes the text of every picked line as one step named Delete text, last line first", async () => {
    const { editor, steps, page } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    await editor.press(0, { x: 100, y: 136 }, true);
    expect(editor.pick?.runs).toHaveLength(2);
    expect(await editor.deleteText()).toBe(true);
    expect(steps).toHaveLength(1);
    expect(steps[0]?.label).toBe("Delete text");
    const texts = steps[0]?.ops.map(
      (op) => (op as unknown as { args: { run: { object: ObjectRef } } }).args.run.object.ordinal,
    );
    expect(texts).toEqual([1, 0]);
    expect(page.map((l) => l.text)).toEqual(["", ""]);
    expect(editor.pick).toBeUndefined();
  });
});

describe("styling picked text", () => {
  it("is one setTextStyle op for all the picked lines, named for the change", async () => {
    const { editor, steps } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    // Editing began with the press: leaving it unchanged ends it with no op, then the style applies.
    expect(await editor.style({ bold: true })).toBe(true);
    expect(steps).toHaveLength(1);
    expect(steps[0]?.label).toBe("Bold text");
    expect(argsOf(steps[0])).toMatchObject({
      page: "p0",
      bold: true,
    });
    expect(editor.pick?.runs).toHaveLength(1);
  });

  it("saves the text being typed first, so the style is made on the edited run", async () => {
    const { editor, steps } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    editor.typed("Hello again");
    expect(await editor.style({ underline: true })).toBe(true);
    expect(steps.map((s) => s.label)).toEqual(["Edit text", "Underline text"]);
    expect(editor.pick?.runs[0]?.text).toBe("Hello again");
  });

  it("is not made while typed text can't be saved", async () => {
    const { editor, steps } = world();
    await editor.press(0, { x: 100, y: 106 }, false);
    editor.typed("你");
    expect(await editor.style({ bold: true })).toBe(false);
    expect(steps).toHaveLength(0);
  });
});

describe("stepping and announcing", () => {
  it("steps through editable lines and says which is picked", async () => {
    const { editor } = world();
    await first(editor);
    editor.cycle(0, 1);
    expect(editor.announcement()).toBe("Text “Hello, world!” selected, page 1, line 1");
    editor.cycle(0, 1);
    expect(editor.pick?.runs[0]?.text).toBe("A second line");
  });

  it("types into the first editable line inside a block's box, and Tab goes on to the next", async () => {
    const { editor, steps } = world();
    expect(await editor.editWithin(0, [0, 90, 300, 150])).toBe(true);
    expect(editor.typing?.original).toBe("Hello, world!");
    editor.typed("Hi");
    await editor.step(0, 1);
    expect(steps).toHaveLength(1);
    expect(editor.typing?.original).toBe("A second line");
    expect(await editor.editWithin(0, [400, 400, 500, 500])).toBe(false);
  });
});
