// The reader's layout (§5.1 of the UI/UX plan, ADR 0024 §1): the top bar across the top, an optional
// secondary toolbar under it, then the rail, the canvas and the inspector side by side, and the
// status bar along the bottom. Each is a slot: the app fills the rail with `LeftRail` and the
// inspector with `Inspector`, which draw their own landmarks and nothing while closed, so the canvas
// takes their width back. Given the shell's `modes`, the secondary toolbar is the current mode's,
// unless a tool's toolbar or the find bar replaces it (ADR 0024 §2); given its `layout`, focus mode
// hides the mode's toolbar and keeps the one that replaced it until it is closed (§1).

import type { ReactNode } from "react";
import type { ShellLayout } from "../shell/layout.js";
import type { ModeRegistry, ModesState } from "../shell/modes.js";
import { usePhone } from "../shell/phone.js";
import { type ExternalStore, useStoreState } from "../state/hooks.js";

/** Props of `ReaderShell`. */
export interface ReaderShellProps {
  /** The top bar: a `ReaderTopBar`. */
  topBar: ReactNode;
  /** A tool's toolbar or the find bar, under the top bar in place of the mode's own. */
  secondaryToolbar?: ReactNode;
  /** The left rail: pages, outline and the other tabs. */
  rail?: ReactNode;
  /** The right docked panel, such as repair details; it labels itself. */
  inspector?: ReactNode;
  /** The status bar's items. */
  status?: ReactNode;
  /** The phone's bottom bar (`PhoneBottomBar`), in place of the status bar below the phone breakpoint. */
  bottomBar?: ReactNode;
  /** The canvas: the viewer, or a page the canvas holds instead (the home, an error). */
  children: ReactNode;
  /** The shell's layout: focus mode hides the mode's toolbar. */
  layout?: ShellLayout;
  /** The modes: the current one's toolbar sits under the top bar. */
  modes?: ModeRegistry;
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

const NO_MODES: ExternalStore<ModesState> = {
  getState: () => ({ modes: [], current: undefined }),
  subscribe: () => () => undefined,
};

/** The current mode's toolbar, or undefined without modes or for a mode without one. */
function useModeToolbar(modes: ModeRegistry | undefined): (() => ReactNode) | undefined {
  return useStoreState(modes ?? NO_MODES, (s) => s.modes.find((m) => m.id === s.current)?.toolbar);
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
  modes,
  bottomBar,
}: ReaderShellProps): ReactNode {
  const focus = useFocus(layout);
  const phone = usePhone();
  const phoneBar = phone && bottomBar !== undefined;
  const foot = phoneBar ? bottomBar : status;
  const modeToolbar = useModeToolbar(modes);
  const toolbar = secondaryToolbar ?? (focus || modeToolbar === undefined ? undefined : modeToolbar());
  return (
    <div className="paper-reader-shell" data-focus={focus || undefined}>
      {topBar}
      {toolbar === undefined ? null : <div className="paper-reader-secondary">{toolbar}</div>}
      <div className="paper-reader-body">
        {rail}
        {/* The page's single-key shortcuts (Page Down, +, ?) act while focus is in the canvas. */}
        <main className="paper-reader-canvas" data-shortcut-scope="page">
          {children}
        </main>
        {inspector}
      </div>
      {foot === undefined ? null : (
        <footer className="paper-reader-status" data-phone={phoneBar || undefined}>
          {foot}
        </footer>
      )}
    </div>
  );
}
