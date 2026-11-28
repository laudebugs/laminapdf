import { WATERMARK_DEFAULT } from "@laminapdf/ui/editing";
import { describe, expect, it, vi } from "vitest";
import { PageMarksController } from "./controller.ts";

/** A promise settled from outside. */
function gate<T = void>() {
  let open: (value: T) => void = () => undefined;
  const promise = new Promise<T>((resolve) => {
    open = resolve;
  });
  return { promise, open };
}

describe("Page marks controller", () => {
  it("closes the dialog when Apply was pressed before the file's answer arrived", async () => {
    const probe = gate<unknown>();
    const edit = gate();
    const engine = {
      getState: () => ({ pages: [{ id: "p1", label: "1" }], title: "report" }),
      view: () => probe.promise,
    };
    const apply = vi.fn(() => edit.promise);
    const viewer = {
      edits: { getState: () => ({ history: { position: 0 } }), subscribe: () => () => undefined, apply },
    };
    const content = {
      documentFonts: { forText: async () => ({ fonts: ["helvetica"], missing: [] }) },
      pageIdOf: () => "p1",
    };
    const toasts = { add: vi.fn() };
    const marks = new PageMarksController(engine as never, viewer as never, content as never, { toasts } as never);

    marks.openAdd("watermark");
    const applying = marks.apply({
      tab: "watermark",
      values: { ...WATERMARK_DEFAULT, text: "DRAFT" },
      image: undefined,
      pages: [0],
    });
    await vi.waitFor(() => expect(apply).toHaveBeenCalled());
    // The probe answers while the edit is being made, and replaces the dialog's object.
    probe.open({ client: { wouldViolate: async () => [], metadata: async () => ({}) } });
    await vi.waitFor(() => expect(marks.getState().dialog?.canHide).toBe(true));
    edit.open();
    await applying;

    expect(marks.getState().dialog).toBeUndefined();
    expect(marks.getState().working).toBeUndefined();
    expect(toasts.add).toHaveBeenCalledOnce();
    marks.dispose();
  });
});
