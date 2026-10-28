import { PDFDocument, StandardFonts, rgb } from 'pdf-lib';
import { writeFileSync } from 'node:fs';

const doc = await PDFDocument.create();
doc.setTitle('Hello from lamina');
const page = doc.addPage([612, 792]);                 // US Letter, points
const regular = await doc.embedFont(StandardFonts.Helvetica);
const bold = await doc.embedFont(StandardFonts.HelveticaBold);

page.drawRectangle({ x: 72, y: 600, width: 468, height: 120,
  borderColor: rgb(0.2, 0.4, 0.8), borderWidth: 2 });
page.drawText('Hello from lamina', { x: 90, y: 690, size: 24, font: bold });
page.drawText(
  'This one-page document was produced with pdf-lib, a drawing-API library.\n' +
  'Line breaks are explicit: pdf-lib does no paragraph layout.',
  { x: 90, y: 660, size: 12, font: regular, lineHeight: 14 });

writeFileSync(process.argv[2] ?? 'pdflib.pdf', await doc.save());
