---
title: "Comment on a PDF"
description: "Highlight, underline and strike through text, add notes and carets, draw ink and shapes, and change them in the inspector, in the Lamina PDF reader."
version: "0.6.0"
url: "https://docs.laminapdf.com/help/annotating/"
---

# Comment on a PDF

Highlight, underline and strike through text, add notes and carets, draw ink and shapes, and change them in the inspector, in the Lamina PDF reader.

The reader's Comment mode marks up a PDF the way you would a printout: highlight a sentence, leave a note in the margin, circle a figure. Each comment is stored in the PDF as an annotation, so other PDF readers show it too. This page covers the web reader at [app.laminapdf.com](https://app.laminapdf.com).

Nothing changes in your file until you save. Every comment you add, move or change is one step that Undo takes back.

## Start commenting

Open a PDF, then choose **Comment** in the mode switcher at the top of the window, beside Read and Organise. The Comment toolbar appears under the top bar. You can also run **Comment mode** from the command palette.

The first time you add a comment, the reader asks for your name. Each comment carries it, so others can tell who made it. The name stays on your device; you can change it later from the inspector.

## Choose a tool

The toolbar's tools, from left to right:

| Tool | Key | What it does |
| --- | --- | --- |
| Select | | Selects comments to move, resize or change |
| Highlight | H | Marks text with a see-through colour |
| Underline | U | Draws a line under text |
| Strike | | Draws a line through text |
| Caret | | Marks a place where text should be inserted |
| Note | | Places a note you can open to read |
| Ink | P | Draws freehand |
| Shapes | | Draws a rectangle, ellipse, line, arrow, polygon (a closed shape of straight sides) or polyline (an open one) |

A tool's key works while the page has the focus. To turn these keys off, with the reader's other single-key shortcuts, switch off **Single-key shortcuts** in Settings › Appearance and accessibility.

While a tool is active, the status bar says how to use it:

- **Highlight, Underline and Strike:** drag across text, or select text and press Enter.
- **Caret:** click between two letters.
- **Note:** click where the note goes, or press Enter to place one in the middle of the page.
- **Shapes:** drag to draw, or press Enter to place one in the middle of the page. Hold Shift for squares, circles and 45° lines.
- **Polygon and polyline:** click each corner, then click the last one again or press Enter.
- **Ink:** drag to draw. Strokes made within a second of each other join one drawing.

Escape goes back to Select.

**Colour** and **Width** on the toolbar set how your next comment looks. Highlights, notes and fills come in yellow, green, blue or pink. Lines come in black, red, blue or green, from 0.5 to 8 points wide (a point is 1/72 inch).

## Change a comment in the inspector

With Select, click a comment to select it. Shift-click, or Ctrl-click (⌘-click on a Mac), adds a comment to the selection or takes it out. The inspector at the side of the window shows what you can change:

- **Colour** (**Line colour** for shapes and lines), **Fill** and **Opacity**
- **Width** and **Border**: solid, dashed or cloudy
- **Line start** and **Line end** for lines and arrows
- **Smoothing** for ink
- the **Note** text and the **Author**, where **Change your name…** changes the name your comments carry
- **Delete**, which removes the selected comments; Delete or Backspace on the keyboard does the same

If a comment was already in the file, a setting the reader can't read yet shows "As in the file". It stays that way until you change it. A comment the file locks can't be moved or changed, and the inspector says so.

## Move and resize

Drag a selected comment to move it. Carets, ink, lines, shapes, polygons and polylines also resize: drag one of the eight handles round a single selected one. Hold Shift to keep its proportions.

From the keyboard:

- the arrow keys move the selection 1 point, and Shift with the arrows 10 points;
- Alt with the arrows resizes a shape or drawing from its bottom-right corner.

## A shape over text doesn't hide it

When a shape covers text, the reader warns: *This shape covers text, but the text stays in the file: anyone can still select and copy it.* A shape is drawn on top of the page, so the words under it are still there. To remove text for good, use redaction instead.

## Files that limit comments

Some files say who may change them, and a comment is a change:

- **A file certified to allow no changes, or only form filling:** its author signed it and limited what may change later. A banner under the toolbar warns: *Saving will invalidate the signature.* **Save a copy…** (**Download a copy** in browsers that can't save to a folder) keeps the original signed. If you save the file itself, the reader asks before it removes the signature.
- **A file whose usage rights don't allow comments:** the banner offers the same: save or download a copy to keep the rights.
- **An encrypted file:** the reader can't edit it yet, so the tools are turned off.

Once your comments are in, save the file, or save a copy to keep the original as it was.
