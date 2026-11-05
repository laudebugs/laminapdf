---
id: image-with-caption
title: An image with a caption
checker: image-with-caption.mjs
inputs: [fixtures/images/photo-rgb.jpg => photo.jpg]
output: photo.pdf
---
Create `photo.pdf`: one A4 page that shows `photo.jpg` with its top-left corner 72 points from the left edge and 72 points from the top edge, 200 points wide, keeping the photo's aspect ratio. Embed the JPEG as it is, without decoding it. Below the photo, add the caption "Figure 1: Test photo".
