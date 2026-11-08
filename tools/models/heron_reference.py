"""Records Heron's regions on two example pages with ONNX Runtime's own Python package, PDFium
and Pillow: the reference that laminapdf-layout-onnx's ignored test compares against.

Run by hand after `cargo xtask models fetch docling-layout-heron`, with the packages in
requirements.txt:

    python -I tools/models/heron_reference.py <path to model.onnx>

It writes crates/laminapdf-layout-onnx/tests/reference/heron.json (format it with
`pnpm exec biome format --write` afterwards, as the repository's JSON is). The post-processing is the
crate's: scores of at least 0.3, best first, a box overlapping a kept one by 0.9 IoU or more
dropped, boxes in page points.
"""

import json
import sys
from pathlib import Path

import numpy as np
import onnxruntime
import pypdfium2
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
PAGES = ["invoice.pdf", "pdf_ua_report.pdf"]
DPI = 150
THRESHOLD = 0.3
SAME_BOX = 0.9
NAMES = {
    0: "caption", 1: "footnote", 2: "formula", 3: "list-item", 4: "page-footer", 5: "page-header",
    6: "picture", 7: "section-header", 8: "table", 9: "text", 10: "title",
}


def iou(a, b):
    w = max(0.0, min(a[2], b[2]) - max(a[0], b[0]))
    h = max(0.0, min(a[3], b[3]) - max(a[1], b[1]))
    area = lambda r: max(0.0, r[2] - r[0]) * max(0.0, r[3] - r[1])
    union = area(a) + area(b) - w * h
    return w * h / union if union > 0 else 0.0


def regions(session, path):
    page = pypdfium2.PdfDocument(str(path))[0]
    width_pt, height_pt = page.get_size()
    image = page.render(scale=DPI / 72).to_pil().convert("RGB")
    w, h = image.size
    pixels = np.asarray(image.resize((640, 640), Image.BILINEAR)).transpose(2, 0, 1)[None]
    labels, boxes, scores = session.run(
        None, {"images": pixels, "orig_target_sizes": np.array([[w, h]], dtype=np.int64)}
    )
    found = sorted(
        ((float(s), int(l), [float(v) for v in b]) for l, b, s in zip(labels[0], boxes[0], scores[0]) if s >= THRESHOLD),
        key=lambda t: -t[0],
    )
    kept = []
    for candidate in found:
        if all(iou(k[2], candidate[2]) < SAME_BOX for k in kept):
            kept.append(candidate)
    sx, sy = width_pt / w, height_pt / h
    return [
        {
            "label": NAMES.get(label, "other"),
            "score": round(score, 3),
            "bbox": [round(b[0] * sx, 2), round(b[1] * sy, 2), round(b[2] * sx, 2), round(b[3] * sy, 2)],
        }
        for score, label, b in kept
    ]


def main():
    session = onnxruntime.InferenceSession(sys.argv[1])
    out = {
        "tool": f"onnxruntime {onnxruntime.__version__}, PDFium (pypdfium2) at {DPI} dpi, Pillow bilinear resize",
        "dpi": DPI,
        "threshold": THRESHOLD,
        "pages": {name: regions(session, ROOT / "fixtures/golden/examples" / name) for name in PAGES},
    }
    target = ROOT / "crates/laminapdf-layout-onnx/tests/reference/heron.json"
    target.write_text(json.dumps(out, indent=1) + "\n")
    print(f"wrote {target}")


if __name__ == "__main__":
    main()
