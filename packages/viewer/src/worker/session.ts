// The edit session's history (E1 plan §5, ADR 0006 §4 and §7): what the interface lists of the op
// log the model keeps. The model holds the undo records; this holds a step per transaction, with
// its label, pages and time, and where the document stands among them. edit.ts moves both
// together, so the steps done are always the model's undo stack and the steps undone its redo
// stack, up to the same depth.
//
// Positions are counted from the document as opened: position p (absolute) is the state after the
// p-th step ever made. `saved` is the absolute position the last save wrote, so the semantic dirty
// state is the distance from it, even once the depth limit has dropped the steps back to it (undo
// can't reach it then, but every step since still counts). It is lost when the saved document is
// on no path from here any more: a new edit cleared the steps that led to it, or one was skipped.

import { type EditHistory, type EditStep, HISTORY_DEPTH, type RevertedStep } from "../protocol.js";

/** One document's history. */
export interface Session {
  /** The steps kept, oldest first. */
  readonly steps: EditStep[];
  /** How many of `steps` are done. */
  position: number;
  /** How many of the oldest steps the depth limit dropped. */
  dropped: number;
  /** The absolute position of the last save (0: the file as opened); undefined once it is on no path from here. */
  saved: number | undefined;
  /** The next step's ID. */
  nextId: number;
  /** The most steps kept. */
  readonly depth: number;
}

/** An empty history for a document just opened, keeping `depth` steps. */
export function newSession(depth = HISTORY_DEPTH): Session {
  return { steps: [], position: 0, dropped: 0, saved: 0, nextId: 1, depth };
}

/** The absolute position the document is at. */
const at = (session: Session) => session.dropped + session.position;

/** What the interface reads of `session`. */
export function historyOf(session: Session, diverged: boolean): EditHistory {
  return {
    steps: [...session.steps],
    position: session.position,
    dropped: session.dropped,
    unsaved: unsaved(session),
    diverged,
  };
}

/**
 * Transactions between the document now and the saved one: the distance between their positions,
 * or, with the saved document on no path from here, every step done (at least one: it differs).
 */
function unsaved(session: Session): number {
  if (session.saved === undefined) return Math.max(1, session.position);
  return Math.abs(at(session) - session.saved);
}

/**
 * Forgets the steps undone: a new transaction cleared the model's redo stack. A save among them
 * can no longer be reached.
 */
export function clearRedo(session: Session): void {
  session.steps.length = session.position;
  if (session.saved !== undefined && session.saved > at(session)) session.saved = undefined;
}

/** Records a transaction the model committed: the steps undone go, and the oldest past the depth. */
export function record(session: Session, label: string, pages: readonly number[], time: number): EditStep {
  clearRedo(session);
  const step: EditStep = { id: session.nextId++, label, pages: [...pages], at: time };
  session.steps.push(step);
  session.position += 1;
  while (session.steps.length > session.depth) {
    session.steps.shift();
    session.position -= 1;
    session.dropped += 1;
  }
  return step;
}

/** How the model's undo or redo of one step went. */
export type Outcome = RevertedStep["outcome"];

/**
 * Moves back over the newest done step, which the model just undid with `outcome`. A skipped
 * undo restored nothing and left no redo entry: the step goes, and the document stays where it
 * was, after it.
 */
export function stepBack(session: Session, outcome: Outcome): RevertedStep {
  const index = session.position - 1;
  const step = session.steps[index];
  if (step === undefined) throw new Error("there is no step to undo");
  if (outcome === "skipped") {
    // The state before the step can't be reached any more; the one after it is where we are.
    removeStep(session, index, "undo");
    return { step, way: "undo", outcome };
  }
  session.position -= 1;
  return { step: mark(session, index, outcome), way: "undo", outcome };
}

/**
 * Moves forward over the oldest undone step, which the model just redid with `outcome`. A skipped
 * redo left no undo entry: the step goes, and the document stays where it was, before it.
 */
export function stepForward(session: Session, outcome: Outcome): RevertedStep {
  const index = session.position;
  const step = session.steps[index];
  if (step === undefined) throw new Error("there is no step to redo");
  if (outcome === "skipped") {
    removeStep(session, index, "redo");
    return { step, way: "redo", outcome };
  }
  session.position += 1;
  return { step: mark(session, index, outcome), way: "redo", outcome };
}

/** Marks step `index` partly restored, or not, after an undo or redo that went `outcome`. */
function mark(session: Session, index: number, outcome: Outcome): EditStep {
  const { partial: _, ...step } = session.steps[index] as EditStep;
  const next: EditStep = outcome === "partial" ? { ...step, partial: true } : step;
  session.steps[index] = next;
  return next;
}

/**
 * Takes out step `index`, which the model no longer has. Its two states become one: after it for
 * an undo that restored nothing, before it for such a redo; the other can't be reached.
 */
export function removeStep(session: Session, index: number, way: "undo" | "redo"): void {
  const before = session.dropped + index;
  session.steps.splice(index, 1);
  if (index < session.position) session.position -= 1;
  const { saved } = session;
  if (saved === undefined) return;
  const lost = way === "undo" ? before : before + 1;
  if (saved === lost) session.saved = undefined;
  else if (saved > before) session.saved = saved - 1;
}

/** Records the document now as the one saved (E1e's saves). */
export function markSaved(session: Session): void {
  session.saved = at(session);
}

/** A copy of `session` to go back to when a move fails half way. */
export function snapshot(session: Session): Session {
  return { ...session, steps: [...session.steps] };
}

/** Puts `session` back as `kept` was. */
export function restore(session: Session, kept: Session): void {
  session.steps.length = 0;
  session.steps.push(...kept.steps);
  session.position = kept.position;
  session.dropped = kept.dropped;
  session.saved = kept.saved;
  session.nextId = kept.nextId;
}
