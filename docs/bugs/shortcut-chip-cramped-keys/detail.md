# Diagnosis: shortcut chips run their keys together

[Resolution](resolution.md).

## What the user saw

A shortcut chip such as `⌘V` drew the ⌘ touching the V, and the text didn't look centred in the chip. The owner reported it on 2026-11-08 with a screenshot of the reader's chip.

## How to reproduce it

Render `ShortcutChip` for macOS with the real tokens, fonts and chip CSS, here in Chromium at a pixel ratio of 2 and 8: `⌘V`, `⇧⌘E`, `⌥⌘P`, `⌘↑`, and for comparison `esc` and `Ctrl V`. The ⌘ touches the key after it in every macOS chip, and sits lower than the letters. Windows and Linux chips (`Ctrl V`) look right.

One false start: a page loaded with `setContent` can't read the font files, so the first measurement used a fallback font. The page has to be served as a file for Lamina Sans to load; the browser's font report then confirms it draws both glyphs.

## What was measured

Glyph metrics from the subset `lamina-sans-latin-wght-normal.woff2` (1,000 units to the em):

| Glyph | Ink, bottom to top | Centre | Against the capitals' centre (328) | Side bearings |
| --- | --- | --- | --- | --- |
| `⌘` | −178 to 698 | 260 | −68 | 50 / 50 |
| `↑ ↓ ← →` | −25 to 527 (↑) | 251 | −77 | 26–42 |
| `⌥` | −12 to 662 | 325 | −3 | 38 / 38 |
| `⇧` | −53 to 708 | 328 | 0 | 21 / 21 |
| `V` | 0 to 656 | 328 | 0 | 0 / 0 |

At the chip's 11 px, the ⌘'s right bearing and the V's left bearing add up to 0.55 px, and macOS writes its keys with no space between them (`⌘V`), so nothing else separates them. In the render, the ⌘'s ink reaches 4 device pixels (2 CSS px) below the V's baseline.

The capitals themselves sit in the middle of the chip: 3.5 px of line box above them and 3.3 px below. So the chip's vertical padding was right, and only the glyphs drawn low (⌘ and the arrows) looked off-centre.

## Cause

Two things the font does, neither of them wrong for running text:

- **⌘ is a full em wide with almost no side bearing,** so next to a letter with none (V, A, W) the two glyphs touch. The chip joined the keys into one string, so it couldn't space them.
- **⌘ and the arrows sit below the middle of the capitals:** ⌘ by 0.068 em and the arrows by 0.077 em, since the arrows are centred on the maths axis. Beside a capital, they look low.
