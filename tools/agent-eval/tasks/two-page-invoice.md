---
id: two-page-invoice
title: A two-page invoice with a table
checker: two-page-invoice.mjs
inputs: []
output: invoice.pdf
---
Create `invoice.pdf`, a two-page invoice on US Letter pages.

Page 1 has the heading "Invoice INV-2026-042" near the top, then a table with the columns Item, Qty and Price, one row per item, each row's cells on one line:

| Item | Qty | Price |
| --- | --- | --- |
| Paper stock | 10 | 45.00 |
| Toner cartridge | 2 | 160.00 |
| Binding service | 1 | 80.50 |
| Delivery | 1 | 25.00 |

Page 2 has the line "Total: 310.50 EUR" and, below it, "Thank you for your business".
