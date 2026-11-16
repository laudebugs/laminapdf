import type { PlatformFile } from "@laminapdf/ui";
import { describe, expect, it } from "vitest";
import { openLaunchedFiles } from "./launch.ts";

const file = (name: string) => ({ name, size: 1, writable: false, read: async () => new Uint8Array() }) as PlatformFile;

describe("openLaunchedFiles", () => {
  it("opens the file of a launch, asks about several, and does nothing for an empty one", () => {
    let deliver: ((files: PlatformFile[]) => void) | undefined;
    const opened: string[] = [];
    const asked: string[][] = [];
    const open = async (f: PlatformFile) => {
      opened.push(f.name);
    };
    openLaunchedFiles(
      {
        onOpenRequest: (listener) => {
          deliver = listener;
          return () => undefined;
        },
      },
      { open },
      (files) => asked.push(files.map((f) => f.name)),
    );
    deliver?.([file("a.pdf")]);
    deliver?.([file("b.pdf"), file("c.pdf")]);
    deliver?.([]);
    expect(opened).toEqual(["a.pdf"]);
    expect(asked).toEqual([["b.pdf", "c.pdf"]]);
  });
});
