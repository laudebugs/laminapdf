// The cursor check over a built page (motion plan §3, 1 and §6): every element a person can click
// shows the hand, a disabled one the arrow and a busy one the busy arrow, read where a pointer over
// its middle lands. Run in the page by Playwright (`page.evaluate(cursorCheck)`), so the function
// carries everything it needs: nothing outside it is in scope there. The reader's, the docs' and
// the website's e2e suites call it in each state they open, in Chromium, Firefox and WebKit. Being
// one function, it is longer and branchier than Biome allows elsewhere (biome.json).

/** An element whose cursor breaks the rule: what it is, the cursor it shows and the one it should. */
export interface CursorProblem {
  readonly element: string;
  readonly cursor: string;
  readonly expected: string;
}

/** What `cursorCheck` found: how many controls it checked, and those whose cursor is wrong. */
export interface CursorCheck {
  readonly checked: number;
  readonly problems: CursorProblem[];
}

/**
 * Every control on the page a person can point at, and whose cursor isn't the rule's: `pointer`
 * (the tokens' --paper-cursor-action and --paper-cursor-link); `progress` when busy (inside
 * `aria-busy="true"` or React Aria's `data-pending`); `default` when disabled (inside `:disabled`,
 * `aria-disabled="true"` or `data-disabled`). Busy wins, since a running control is also blocked.
 * An enabled control matching `dragHandles` (a selector the caller passes, such as a sheet's
 * grabber; an empty string for none) is dragged rather than pressed, and may show `grab` in place of the hand; any other
 * control showing `grab` is a problem.
 *
 * A control in the window is read where a pointer at its middle lands, on what the pointer meets
 * there (the control, or an icon or label inside it): a control another element covers there is
 * left out, as is one
 * under 4 px a side, too small to be a target itself (React Aria's hidden inputs and dismiss
 * buttons, whose label or overlay is the target). A control below or beside the window, that a
 * person scrolls to, is read on itself; one placed where no scroll reaches (a skip link waiting
 * for focus), with no box, or hidden by `visibility`, is left out. Since covered controls are left
 * out, callers check `checked` too: a page where every control is covered checks nothing.
 */
export function cursorCheck(dragHandles: string): CursorCheck {
  // No named function inside: a test runner's transform may wrap one in a helper the page lacks.
  const clickable = [
    "a[href]",
    "button",
    "summary",
    "select",
    'input:is([type="button"], [type="submit"], [type="reset"], [type="checkbox"], [type="radio"])',
    'label:has(input[type="radio"], input[type="checkbox"])',
    '[role="treegrid"] [role="row"]',
    ...[
      "button",
      "link",
      "tab",
      "option",
      "menuitem",
      "menuitemradio",
      "menuitemcheckbox",
      "checkbox",
      "radio",
      "switch",
      "combobox",
    ].map((role) => `[role="${role}"]:not(input, textarea)`),
  ].join(", ");
  let checked = 0;
  const problems: CursorProblem[] = [];
  for (const element of document.querySelectorAll(clickable)) {
    const box = element.getBoundingClientRect();
    if (box.width < 4 || box.height < 4 || !element.checkVisibility({ visibilityProperty: true })) continue;
    const [x, y] = [box.left + box.width / 2, box.top + box.height / 2];
    // Where no scroll reaches: above or left of the page's start.
    if (y + scrollY < 0 || x + scrollX < 0) continue;
    const inWindow = x >= 0 && y >= 0 && x < innerWidth && y < innerHeight;
    const hit = inWindow ? document.elementFromPoint(x, y) : null;
    if (inWindow && (hit === null || !(hit === element || element.contains(hit)))) continue;
    checked += 1;
    const busy = element.closest('[aria-busy="true"], [data-pending]') !== null;
    const disabled = element.closest(':disabled, [aria-disabled="true"], [data-disabled]') !== null;
    const expected = busy ? "progress" : disabled ? "default" : "pointer";
    // What the pointer meets: an icon or a label inside a control must show the control's cursor.
    const wrong = hit ?? element;
    const dragged = dragHandles !== "" && (element.matches(dragHandles) || wrong.closest(dragHandles) !== null);
    const allowed = expected === "pointer" && dragged ? ["pointer", "grab"] : [expected];
    if (allowed.includes(getComputedStyle(wrong).cursor)) continue;
    const [what, at] = [element, wrong].map((target) => {
      const classes = [...target.classList].map((name) => `.${name}`).join("");
      const role = target.getAttribute("role");
      const label = target.getAttribute("aria-label") ?? (target.textContent ?? "").trim().replace(/\s+/g, " ");
      return `<${target.localName}${role === null ? "" : `[role=${role}]`}${classes}>${label === "" ? "" : ` "${label.slice(0, 40)}"`}`;
    });
    problems.push({
      element: wrong === element ? `${what}` : `${what} at ${at}`,
      cursor: getComputedStyle(wrong).cursor,
      expected,
    });
  }
  return { checked, problems };
}
