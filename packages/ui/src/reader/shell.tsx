// The reader's layout (§5.1 of the UI/UX plan, ADR 0024 §1): the top bar across the top, an optional
// secondary toolbar under it, then the rail, the canvas and the inspector side by side, and the
// status bar along the bottom. Each is a slot: the app fills the rail with `LeftRail` and the
// inspector with `Inspector`, which draw their own landmarks and nothing while closed, so the canvas
// takes their width back. Given the shell's `layout`, focus mode hides the secondary toolbar.

import type { ReactNode } from "react";
import type { ShellLayout } from "../shell/layout.js";
import { type ExternalStore, useStoreState } from "../state/hooks.js";

/** Props of `ReaderShell`. */
export interface ReaderShellProps {
  /** The top bar: a `ReaderTopBar`. */
  topBar: ReactNode;
  /** The current mode's toolbar, under the top bar. */
  secondaryToolbar?: ReactNode;
  /** The left rail: pages, outline and the other tabs. */
  rail?: ReactNode;
  /** The right docked panel, such as repair details; it labels itself. */
  inspector?: ReactNode;
  /** The status bar's items. */
  status?: ReactNode;
  /** The canvas: the viewer, or a page the canvas holds instead (the home, an error). */
  children: ReactNode;
  /** The shell's layout: focus mode hides the secondary toolbar. */
  layout?: ShellLayout;
}

/** The shell's layout as a store that never changes, for a shell without one: never in focus mode. */
const NO_LAYOUT: ExternalStore<{ readonly focus: boolean }> = {
  getState: () => ({ focus: false }),
  subscribe: () => () => undefined,
};

/** Whether `layout` is in focus mode; false without one. */
function useFocus(layout: ShellLayout | undefined): boolean {
  return useStoreState<{ readonly focus: boolean }, boolean>(layout ?? NO_LAYOUT, (s) => s.focus);
}

/** The reader's layout, filling its container. */
export function ReaderShell({
  topBar,
  secondaryToolbar,
  rail,
  inspector,
  status,
  children,
  layout,
}: ReaderShellProps): ReactNode {
  const focus = useFocus(layout);
  return (
    <div className="paper-reader-shell" data-focus={focus || undefined}>
      {topBar}
      {secondaryToolbar === undefined || focus ? null : (
        <div className="paper-reader-secondary">{secondaryToolbar}</div>
      )}
      <div className="paper-reader-body">
        {rail}
        {/* The page's single-key shortcuts (Page Down, +, ?) act while focus is in the canvas. */}
        <main className="paper-reader-canvas" data-shortcut-scope="page">
          {children}
        </main>
        {inspector}
      </div>
      {status === undefined ? null : <footer className="paper-reader-status">{status}</footer>}
    </div>
  );
}
