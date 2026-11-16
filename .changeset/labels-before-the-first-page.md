---
"@laminapdf/sdk": patch
---

Pages inserted before the first page of a labelled document no longer repeat its label. They join the start of the first label range: before a range starting at "i" or "1", the first new page takes that label and the range's own pages are numbered up by one, while later ranges, such as an appendix's "A-1", keep theirs. Before a range that starts higher, say at 5, new pages take the numbers below it and nothing else moves. This applies to inserted blank pages, pages inserted from another file and merges.
