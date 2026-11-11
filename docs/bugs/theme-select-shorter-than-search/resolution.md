# Resolution: the theme switch as tall as the search box

The docs site's `paper.css` sets the theme switch in the header (`.docs-header-extras .site-select select`) to 2.5rem, the search box's height. The rule applies only with a fine pointer (a mouse or trackpad). On touch screens the select keeps its 44 px, so it stays easy to tap even where it stands taller than the search box. Elsewhere it keeps §7.19's 32 px.

## How it was checked

The built docs site, in Chromium, at a desktop width: both controls measure 40 px and share a centre line (screenshots in light and dark).

## Left for later

None.
