// The `node` condition (Node, Bun): the `.wasm` file of @laminapdf/wasm, read as bytes (Node's
// fetch can't read file: URLs). The one file in src/ allowed a Node API (biome.json override).
import { readFile } from "node:fs/promises";
import type { Locate } from "../init.js";

export const locate: Locate = (module) =>
  readFile(
    new URL(
      module === "base"
        ? import.meta.resolve("@laminapdf/wasm/lamina_base.wasm")
        : module === "text"
          ? import.meta.resolve("@laminapdf/wasm/lamina_text.wasm")
          : import.meta.resolve("@laminapdf/wasm/lamina_extract.wasm"),
    ),
  );
