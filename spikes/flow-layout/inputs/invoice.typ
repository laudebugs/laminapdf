// The invoice as a Typst template over inputs/invoice.json (experiment B).
#let data = json("invoice.json")
#let money(cents) = str(calc.quo(cents, 100)) + "." + (if calc.rem(cents, 100) < 10 { "0" } else { "" }) + str(calc.rem(cents, 100))

#set document(title: "Invoice " + data.number)
#set page(paper: "a4", margin: (x: 56pt, top: 72pt, bottom: 64pt),
  header: align(right, text(9pt)[Invoice #data.number]),
  footer: context align(center, text(9pt)[Page #counter(page).display() of #counter(page).final().first()]))
#set text(font: "Noto Sans", size: 11pt, lang: "en")

= Invoice #data.number

From #data.seller to #data.buyer, issued #data.date.

#table(
  columns: 4,
  table.header([*Description*], [*Qty*], [*Unit*], [*Amount*]),
  ..data.items.map(item => (
    item.description,
    str(item.qty),
    money(item.unitCents),
    money(item.qty * item.unitCents),
  )).flatten()
)

Total due: #money(data.items.map(item => item.qty * item.unitCents).sum())
