// The gallery page's script: the tokens' stylesheet, then the gallery itself.
import "@laminapdf/tokens/tokens.css";
import { GALLERY_CSS, galleryHtml } from "./gallery.ts";

const style = document.createElement("style");
style.textContent = GALLERY_CSS;
document.head.append(style);
document.body.innerHTML = galleryHtml();
