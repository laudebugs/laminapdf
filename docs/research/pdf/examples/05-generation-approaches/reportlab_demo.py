import sys
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import getSampleStyleSheet
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Flowable

class Box(Flowable):                     # custom flowable: a stroked rectangle
    def wrap(self, aw, ah): return (468, 40)
    def draw(self):
        self.canv.setStrokeColorRGB(0.2, 0.4, 0.8); self.canv.setLineWidth(2)
        self.canv.rect(0, 0, 468, 40)

ss = getSampleStyleSheet()
story = [Paragraph("Hello from lamina", ss["Title"]),
         Paragraph("This document was produced with ReportLab's PLATYPUS "
                   "layout engine: flowables are measured, wrapped and "
                   "paginated into frames automatically.", ss["BodyText"]),
         Spacer(1, 12), Box()]
SimpleDocTemplate(sys.argv[1] if len(sys.argv) > 1 else "reportlab.pdf",
                  pagesize=letter, title="Hello from lamina").build(story)
