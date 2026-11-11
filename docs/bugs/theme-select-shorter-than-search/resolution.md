# Resolution: the theme switch as tall as the search box

The docs site's `paper.css` sets the theme switch in the header (`.docs-header-extras .site-select select`) to 2.5rem, the search box's height. The select keeps §7.19's 32 px everywhere else, and 44 px on touch screens, where the phone menu shows it.

## How it was checked

The built docs site, in Chromium, at a desktop width: both controls measure 40 px and share a centre line (screenshots in light and dark).

## Left for later

None.
