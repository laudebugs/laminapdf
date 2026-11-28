---
"@laminapdf/viewer": minor
---

Push buttons work in Fill & Sign. `client.formActions(id)` and `client.submitRequest(id, index)` read a button's actions and the request a submit would make. The form layer draws a button target over each push button and calls `onPress`.
