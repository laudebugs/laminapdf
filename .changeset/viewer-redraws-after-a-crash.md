---
"@laminapdf/viewer": patch
---

After the document worker crashes, or `stop()` ends it, `render()` sends the renders that were waiting again, once, in the restarted worker, as ADR 0009 §3.6 describes. The pages and thumbnails they were for are drawn again. Before, they stayed blank, and the thumbnail strip reported "the worker stopped" to the page as an unhandled error. The render that was drawing when the worker was lost still fails, so a page that crashes the engine can't loop. When the restart can't reopen the document (an encrypted one waits for its password), a waiting render rejects with an `AbortError`.
