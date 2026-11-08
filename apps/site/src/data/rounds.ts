// The plain names the dashboard gives the todo board's sections (website plan §4), by ID letter.
// The board's own headings ("Library — Round B") stay in progress.json for reference.

const NAMES: Readonly<Record<string, string>> = {
  A: "Foundations",
  B: "Core library",
  L: "Shareable libraries",
  C: "Web reader",
  W: "Website",
  D: "Desktop reader",
  U: "Design and interface",
  H: "Hosted service",
  E: "Editor",
  P: "Publishing",
  Q: "Docs and code quality",
  T: "Trust and proof",
  X: "Cross-cutting",
};

/** The plain name of the board section `letter`; fails the build on a section without one. */
export function roundName(letter: string): string {
  const name = NAMES[letter];
  if (name === undefined) throw new Error(`no plain name for the board's section ${letter}: add one to rounds.ts`);
  return name;
}
