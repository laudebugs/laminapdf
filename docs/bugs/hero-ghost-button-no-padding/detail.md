# The docs home page's "See the recipes" button has no room round its label on hover

## What the user saw

On the docs site's home page, hovering "See the recipes" filled a pill whose ends touched the first and last letters of the label. The owner sent a screenshot and said it looked weird.

## How to reproduce

Open the docs site's home page with a mouse or trackpad and hover "See the recipes", the second button under the tagline.

## Measurements

Built docs, Chromium, 1280 px wide: the button's inline padding was 0, against 20 px on "Your first PDF" beside it (16 px against 0 below 50 rem).

## Cause

The button is Starlight's `minimal` link button, which Starlight 0.42.8 (`LinkButton.astro`) draws as a text link with `padding-inline: 0`. `paper.css` gives every Starlight link button the Paper hover and pressed fill of a ghost button (design language §7.0), so the fill was drawn on a button with no inline padding.
