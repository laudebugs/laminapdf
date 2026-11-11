# Triaging a bug

A bug report describes a symptom; the fix belongs at its cause. This guide is the order we work in, from the report to a fix at the source, and it ends with a written report. Each bug gets a folder under [`docs/bugs/`](../bugs/README.md) that records what happened and how it was resolved.

## 1. Reproduce it the way the user meets it

Start from the user's own steps, on the same kind of input and screen. A harness that calls an API once can miss a bug that a person's clicks, keys or gestures set off.

- Use a real input. A real paper from arXiv showed the blurry-zoom bug; the viewer's synthetic perf fixtures didn't.
- Match the environment: the device pixel ratio (a laptop screen is 2×, browser zoom makes it fractional), the viewport, the browser.
- Drive the real app with Playwright, the way a person does: the toolbar buttons, Ctrl+wheel, the keyboard. In the blurry-zoom bug, one `setZoom` call in a harness looked fine, while the app's Zoom in button reproduced the bug every time.
- Keep a screenshot or a log of the failure, so the fix can be checked against the same evidence.

## 2. Tell slow from stuck

Sample the state over time: a screenshot after 300 ms, 1 s and 6 s, or the page's render state on every frame. A symptom that clears on its own is a performance problem; one that never clears means some work stopped. The two have different causes, and the timeline tells them apart in a minute.

## 3. Measure every stage before guessing

List the stages between the input and the symptom, then time each one on its own. For rendering, these are the engine's parse, the draw of a tile, the worker's round trip and the main thread's paint. Write each candidate cause down, then rule it in or out with a number.

In the blurry-zoom bug, the first suspects were PDFium re-parsing the page for every tile, a tiny low-resolution pass, and tiles requested one at a time. A Node benchmark showed that a parse takes 1 ms and a whole sharp page 60 ms. None of the three could explain a blur that lasted seconds, so the search moved on.

## 4. Log every failure, including the expected ones

Wrap the call between two layers (here, the viewer's requests to the document worker) and log every request with its outcome. Keep the failures a layer treats as routine too. Drop only those you have checked, such as aborts. A layer that catches an error and moves on hides a bug; the log shows it at once. In the blurry-zoom bug, one line explained the symptom: the renderer refused a tile because it "extends past the page".

## 5. Look for two definitions of one thing

When one layer asks for something another refuses, check whether both compute the same quantity separately. In the blurry-zoom bug, the viewer and the renderer each computed the page's size in pixels: one in 64-bit floats from CSS width × pixel ratio, the other in 32-bit floats from points × scale. They differed by one pixel at most zoom levels. A table of both values across the inputs (zoom levels, pixel ratios) shows the pattern, and why some inputs work, here 100 % and 200 %.

## 6. Fix the cause, not the symptom

Change the design so the bug can't recur. Don't make the failing case pass with a guard, a retry, a wider tolerance or a special case. When two layers disagree, give the quantity one home ([maintainability.md](maintainability.md): one home per concept) and make both layers use it. The blurry-zoom fix exported the renderer's own rule, `pagePixels`, and the viewer sizes its canvases and tiles with it, so the viewer can no longer ask for a tile the renderer refuses.

The same holds for a failing test, in CI or on your machine. Don't change product code only to make a test pass. First find out why it fails. If the product is wrong, fix the cause as above. If the test checks something that doesn't make sense (a timing the user never notices, a shortcut of the wrong platform, an implementation detail), change it to check what the user needs, or remove it and say why in the commit message. A change you can't explain by what it does for the user is a patch, and it leaves the real fault in place.

Note what you leave alone. A retry for a failed tile, or a canvas size that would stay sharp at very high zoom, could be worth doing, but they are separate changes with their own PRs.

## 7. Prove the fix

- **A regression test that fails without the fix.** Sweep the inputs that matter (here zoom levels, pixel ratios, page sizes and rotations) rather than adding one case. Then run the old rule through the same sweep and count the failures it would have had. Under the old rule, the blurry-zoom sweep failed in 4,365 of 14,464 cases.
- **The reproduction again.** Rerun step 1's steps and compare with the evidence you kept: the same zoom levels, the same screenshots, now with no failed request.
- **For a visual bug, a test of what the user does and sees.** Drive the interaction the user performs (a real mouse drag, real keys) and check what they would see, not only what an API returns. The text layer's tests selected through the Selection API and passed while no mouse drag could select anything; once a drag could, its tint hid the text it selected, and only a check of the pixels would have caught it.
- **For a visual bug, a look from the user's side.** Ask whether the user can now do what they came to do, not only whether the symptom is gone: try the fix in the built app in a real browser, as they would, and look at the screenshots.
- **The package's own suites.** Run them in every runtime they cover (Node and the browsers for the viewer).

## 8. Record it and report

Create `docs/bugs/<bug-name>/` with two files. Name the folder after the symptom (`viewer-blurry-at-zoom`), and add a row for it to [`docs/bugs/README.md`](../bugs/README.md).

- **`detail.md`**: the report and the diagnosis. It covers what the user saw, how to reproduce it, the measurements that ruled causes in or out, and the cause.
- **`resolution.md`**: the fix and why it removes the cause, the tests that prove it, and what was left for later.

The fix's commit message and PR description repeat the diagnosis in short and link the bug's folder. A bug fix needs no todo: the bug's folder is its record, as the todo is for planned work (an exempt PR, [git workflow §2](git-workflow.md#pick-work-from-the-todo-board)).

End by reporting back to whoever filed the bug. The report answers five questions in this order:

1. **What was wrong**, in one or two sentences a user can follow.
2. **Why**, the cause, with the numbers that show it.
3. **What changed** to fix it.
4. **How it was checked**: the regression test, the reproduction rerun and the suites.
5. **What is left**: anything noticed and not fixed, so it can become its own work.
