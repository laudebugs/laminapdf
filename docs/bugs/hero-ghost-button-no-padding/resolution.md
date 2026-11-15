# Resolution: the minimal hero button padded as a ghost button

The docs site's `paper.css` gives `.sl-link-button.minimal` the inline padding of Starlight's other link buttons: 1.125rem, and 1.25rem from 50 rem wide. Its hover and pressed fills now leave room round the label, as a ghost button's do (design language §7.0). At rest the label sits a little further from "Your first PDF".

## How it was checked

The built docs site in Chromium, hovering the button at 1280 px in the light and dark themes and at 390 px: the inline padding is 20 px (18 px at 390 px), and the fill clears the label on both sides.

## Left for later

None.
