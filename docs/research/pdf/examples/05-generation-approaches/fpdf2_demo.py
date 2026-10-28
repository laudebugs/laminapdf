import sys
from fpdf import FPDF

pdf = FPDF(unit="pt", format="letter")
pdf.set_title("Hello from lamina")
pdf.add_page()
pdf.set_draw_color(51, 102, 204); pdf.set_line_width(2)
pdf.rect(72, 72, 468, 120)                     # fpdf2: origin top-left, y down
pdf.set_font("Helvetica", "B", 24)
pdf.set_xy(90, 90); pdf.cell(text="Hello from lamina")
pdf.set_font("Helvetica", size=12)
pdf.set_xy(90, 125)
pdf.multi_cell(w=432, h=14, text=(            # automatic line wrapping
    "This one-page document was produced with fpdf2. multi_cell() wraps "
    "the paragraph to the given width, a first step towards layout."))
pdf.output(sys.argv[1] if len(sys.argv) > 1 else "fpdf2.pdf")
