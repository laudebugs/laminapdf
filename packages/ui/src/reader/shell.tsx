// The reader's layout (§5.1 of the UI/UX plan): the top bar across the top, an optional secondary
// toolbar under it, then the rail, the canvas and the inspector side by side, and the status bar
// along the bottom. Each is a slot: U1c fills the rail, the modes' toolbars and the inspector.

import type { ReactNode } from "react";

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
}

/** The reader's layout, filling its container. */
export function ReaderShell({
  topBar,
  secondaryToolbar,
  rail,
  inspector,
  status,
  children,
}: ReaderShellProps): ReactNode {
  return (
    <div className="paper-reader-shell">
      {topBar}
      {secondaryToolbar === undefined ? null : <div className="paper-reader-secondary">{secondaryToolbar}</div>}
      <div className="paper-reader-body">
        {rail === undefined ? null : <div className="paper-reader-rail">{rail}</div>}
        <main className="paper-reader-canvas">{children}</main>
        {inspector === undefined ? null : <div className="paper-reader-inspector">{inspector}</div>}
      </div>
      {status === undefined ? null : <footer className="paper-reader-status">{status}</footer>}
    </div>
  );
}
