import type { PlatformFile } from "@laminapdf/ui";
import { describe, expect, it } from "vitest";
import { openLaunchedFiles } from "./launch.ts";

const file = (name: string) => ({ name, size: 1, writable: false, read: async () => new Uint8Array() }) as PlatformFile;

describe("openLaunchedFiles", () => {
  it("opens the first file of each launch, and nothing for an empty one", () => {
    let deliver: ((files: PlatformFile[]) => void) | undefined;
    const opened: string[] = [];
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
    );
    deliver?.([file("a.pdf"), file("b.pdf")]);
    deliver?.([]);
    expect(opened).toEqual(["a.pdf"]);
  });
});
