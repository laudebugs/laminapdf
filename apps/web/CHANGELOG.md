# Changelog: the Lamina PDF web reader

The web reader at [app.laminapdf.com](https://app.laminapdf.com). Each release is tagged `web-v<version>`, and production deploys only from those tags.

## 0.1.0

The first release of the web reader. It opens PDFs inside your browser tab: nothing you open is uploaded, and nothing about you is collected. The reader is free.

### What it does

- **Open** a PDF from your computer (choose, drop or paste it), from the recent list, or from a link (`app.laminapdf.com/#url=…`). Password-protected files ask for their password, and damaged files open with a note on what was repaired.
- **Read** with smooth scrolling over thousands of pages, zoom (fit width, fit page or 10–800 %) and rotation. **Move around** with thumbnails, the outline, page labels, and links with back and forward. Reader mode reflows tagged PDFs.
- **Find** text across the whole document, with match case and whole words.
- **Select and copy** text in reading order or exactly as laid out, select a column with Alt or Option, and act on a selection from its toolbar.
- **Print** all or some pages, **download** the original file, and see the **document's properties**.
- **Look** in a light, dark or high-contrast theme with one of four accents, and read pages in their original colours, sepia, dark or high contrast.
- **Work offline** once loaded, and install the reader as an app where the browser offers it.
- **Use it without a mouse**: every command has a keyboard route, `?` lists the shortcuts, and screen readers hear each page by name and position.
- **See what left the device**: the status bar counts the open document's network requests and lists each one.

### Privacy

The reader makes five promises, and a test checks each one before every release: your files stay on your device; a linked file is fetched only from its own site; no cookies, no third parties, no tracking; passwords stay in the tab; and you can clear everything the reader keeps. [laminapdf.com/privacy](https://laminapdf.com/privacy/) explains each promise.

### Browsers

The last two versions of Chrome, Edge, Firefox and Safari on the desktop, and Safari on iPadOS. Phones get a layout of their own.

### Known issues

- The reader's startup code is 266 KB compressed, over our 200 KB target, so the first visit loads a little slower than planned; a later release brings it down ([#380](https://github.com/laudebugs/laminapdf/issues/380)).
- A screen reader sees only the pages near the one on screen, so "read all" stops after them. Use the outline, Go to page or Page Down to move on.
- Safari's Tab key skips buttons and links unless Option is held or "Press Tab to highlight each item" is on.
