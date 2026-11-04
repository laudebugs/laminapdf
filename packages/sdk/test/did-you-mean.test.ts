// The did-you-mean rule, TS half: every case of fixtures/did-you-mean.json suggests exactly its
// expected names (laminapdf-wasm's src/error.rs runs the same file, so both layers agree).
import { describe, expect, it } from "vitest";
import { didYouMean } from "../src/errors.js";
import { readText } from "./helpers.js";

interface Case {
  readonly about: string;
  readonly given: string;
  readonly known: readonly string[];
  readonly expected: readonly string[];
}
const { cases } = JSON.parse(readText("fixtures/did-you-mean.json")) as { cases: Case[] };

describe("didYouMean", () => {
  it.each(cases)("$about", ({ given, known, expected }) => {
    expect(didYouMean(given, known)).toEqual(expected);
  });
});
