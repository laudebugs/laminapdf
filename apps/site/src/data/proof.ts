// The snapshot behind the How we test page, proof.json in this folder, which `cargo xtask proof
// snapshot` writes from the nightly's proof records (tools/xtask/src/proof/snapshot.rs documents
// each field). The page reads it at build time through readProof, which fails the build on a file
// this type doesn't describe. A section is `null` when that night's job didn't produce it.

import { count, date, entries, flag, list, nullable, oneOf, read, record, text } from "./check.js";

/** How Lamina PDF read one source's files. */
export interface CorpusSource {
  readonly files: number;
  /** Opened with no repair. */
  readonly opened: number;
  /** Opened after a repair. */
  readonly repaired: number;
  /** Refused with an error. */
  readonly refused: number;
  /** Refused while at least one other reader opened it. */
  readonly refusedAlone: number;
  /** Panics, crashes and timeouts. */
  readonly errors: number;
}

/** How often Lamina PDF and another reader agreed on a fact, over the files both read. */
export interface Tally {
  readonly agree: number;
  readonly disagree: number;
  /** Disagreements an exceptions file explains. */
  readonly excused: number;
}

export interface Night {
  readonly commit: string;
  /** The day it ran, `YYYY-MM-DD`. */
  readonly ran: string;
  readonly lamina: string;
  /** Whether every job of the night succeeded. */
  readonly passed: boolean;
  readonly jobs: Readonly<Record<string, string>>;
  readonly corpus: { readonly complete: boolean; readonly sources: Readonly<Record<string, CorpusSource>> } | null;
  readonly differential: {
    readonly inputs: number;
    readonly failures: number;
    readonly tools: Readonly<
      Record<string, { readonly version: string; readonly facts: Readonly<Record<string, Tally>> }>
    >;
  } | null;
  readonly text: {
    readonly tools: Readonly<Record<string, string>>;
    readonly sources: Readonly<
      Record<string, { readonly pages: number; readonly compared: number; readonly agree: number }>
    >;
  } | null;
  readonly fuzz: {
    readonly targets: number;
    readonly seconds: number;
    readonly executions: number | null;
    readonly crashed: number;
  } | null;
  readonly verapdf: {
    readonly version: string;
    readonly validations: readonly {
      readonly file: string;
      readonly profile: "2b" | "2u" | "3b" | "ua1";
      readonly compliant: boolean;
    }[];
  } | null;
  readonly roundtrip: {
    readonly cases: number;
    readonly saved: number;
    readonly refused: number;
    readonly tools: Readonly<Record<string, boolean>>;
  } | null;
  readonly runtimes: {
    readonly passed: readonly string[];
    readonly failed: readonly string[];
    readonly missing: readonly string[];
  } | null;
  readonly render: {
    readonly dpi: number;
    readonly pagesPerFile: number | null;
    readonly sources: Readonly<Record<string, { readonly pages: number; readonly passMinorBp: number }>>;
  } | null;
}

/** proof.json. */
export interface Proof {
  readonly version: 1;
  /** The day the snapshot was written, `YYYY-MM-DD`. */
  readonly date: string;
  /** The newest full night. */
  readonly night: Night;
  /** Of the last 30 full nights: how many were recorded, how many passed, and the day the first ran. */
  readonly nights: { readonly recorded: number; readonly passed: number; readonly since: string };
  /** Fuzzing time summed over every full night, in whole hours. */
  readonly fuzzHours: number;
}

const TALLY = record({ agree: count, disagree: count, excused: count });

const NIGHT = record({
  commit: text,
  ran: date,
  lamina: text,
  passed: flag,
  jobs: entries(text),
  corpus: nullable(
    record({
      complete: flag,
      sources: entries(
        record({ files: count, opened: count, repaired: count, refused: count, refusedAlone: count, errors: count }),
      ),
    }),
  ),
  differential: nullable(
    record({
      inputs: count,
      failures: count,
      tools: entries(record({ version: text, facts: entries(TALLY) })),
    }),
  ),
  text: nullable(
    record({
      tools: entries(text),
      sources: entries(record({ pages: count, compared: count, agree: count })),
    }),
  ),
  fuzz: nullable(record({ targets: count, seconds: count, executions: nullable(count), crashed: count })),
  verapdf: nullable(
    record({
      version: text,
      validations: list(record({ file: text, profile: oneOf("2b", "2u", "3b", "ua1"), compliant: flag })),
    }),
  ),
  roundtrip: nullable(record({ cases: count, saved: count, refused: count, tools: entries(flag) })),
  runtimes: nullable(record({ passed: list(text), failed: list(text), missing: list(text) })),
  render: nullable(
    record({
      dpi: count,
      pagesPerFile: nullable(count),
      sources: entries(record({ pages: count, passMinorBp: count })),
    }),
  ),
});

const PROOF = record({
  version: oneOf(1),
  date,
  night: NIGHT,
  nights: record({ recorded: count, passed: count, since: date }),
  fuzzHours: count,
});

/** `value` (proof.json, parsed) as a {@link Proof}; throws, naming the field, when it isn't one. */
export function readProof(value: unknown): Proof {
  return read<Proof>("proof.json", PROOF, value);
}
