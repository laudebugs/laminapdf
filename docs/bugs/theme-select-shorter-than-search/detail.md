# The theme switch is shorter than the search box beside it

## What the user saw

In the docs site's header, the theme switch (32 px high) stood beside the search box (40 px), so the two controls didn't line up. The owner asked for them to match.

## How to reproduce

Open any page of the docs site at a desktop width (50 rem or wider) and compare the theme switch with the search box in the header.

## Measurements

The search box is 40 px high: Starlight sets `height: 2.5rem` on its search button. The theme switch is `@laminapdf/site-kit`'s select, which follows design language §7.19's 32 px field height.

## Cause

Each control followed its own size: Starlight's search button its 2.5rem, the Paper select §7.19's 32 px. Nothing set the header's controls to one height.
