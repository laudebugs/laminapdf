// The cursor check over a built page (motion plan §3, 1 and §6): every element a person can click
// shows the hand, a disabled one the arrow and a busy one the busy arrow, on the element and on
// whatever the pointer meets at its centre. Run in the page by Playwright (`page.evaluate`), so the
// function carries everything it needs: nothing outside it is in scope there. The sites' e2e suites
// call it on each page they open, in Chromium, Firefox and WebKit.

/** An element whose cursor breaks the rule: what it is, the cursor it shows and the one it should. */
export interface CursorProblem {
  readonly element: string;
  readonly cursor: string;
  readonly expected: string;
}

/**
 * Every clickable element on the page that a person can see or reach and whose cursor isn't the
 * rule's: `pointer` (the tokens' --paper-cursor-action and --paper-cursor-link), `default` when
 * disabled (`:disabled`, `aria-disabled="true"`), `progress` when busy (`aria-busy="true"`).
 * Elements with no box, or hidden by `visibility`, are skipped; a transparent one isn't, since a
 * pointer still finds it.
 */
export function cursorProblems(): CursorProblem[] {
  // No named function inside: a test runner's transform may wrap one in a helper the page lacks.
  const clickable = [
    "a[href]",
    "button",
    "summary",
    "select",
    'input:is([type="button"], [type="submit"], [type="reset"], [type="checkbox"], [type="radio"])',
    ...["button", "link", "tab", "option", "menuitem", "checkbox", "radio", "switch", "combobox"].map(
      (role) => `[role="${role}"]:not(input, textarea)`,
    ),
  ].join(", ");
  const shown = [...document.querySelectorAll(clickable)].filter((element) => {
    const box = element.getBoundingClientRect();
    return box.width > 0 && box.height > 0 && element.checkVisibility({ visibilityProperty: true });
  });
  return shown.flatMap((element): CursorProblem[] => {
    const busy = element.matches('[aria-busy="true"]');
    const disabled = element.matches(':disabled, [aria-disabled="true"]');
    const expected = busy ? "progress" : disabled ? "default" : "pointer";
    // The element, then what the pointer meets at its centre when that is in the window and inside
    // the element: an icon or a label in a button must not show another cursor.
    const box = element.getBoundingClientRect();
    const [x, y] = [box.left + box.width / 2, box.top + box.height / 2];
    const inWindow = x >= 0 && y >= 0 && x < innerWidth && y < innerHeight;
    const hit = inWindow ? document.elementFromPoint(x, y) : null;
    const targets = hit !== null && hit !== element && element.contains(hit) ? [element, hit] : [element];
    const wrong = targets.find((target) => getComputedStyle(target).cursor !== expected);
    if (wrong === undefined) return [];
    const [what, at] = [element, wrong].map((target) => {
      const classes = [...target.classList].map((name) => `.${name}`).join("");
      const label = target.getAttribute("aria-label") ?? (target.textContent ?? "").trim().replace(/\s+/g, " ");
      return `<${target.localName}${classes}>${label === "" ? "" : ` "${label.slice(0, 40)}"`}`;
    });
    const where = wrong === element ? `${what}` : `${what} at ${at}`;
    return [{ element: where, cursor: getComputedStyle(wrong).cursor, expected }];
  });
}
