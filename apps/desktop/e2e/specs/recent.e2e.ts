// Recent files on the start window (S01), after the earlier scenarios opened their files and the
// watch scenario deleted its own: the deleted one says "File not found" and can be removed, and an
// entry reopens with no dialog. The cold-open run checks the list after a restart and clears it.
import { $ } from "@wdio/globals";
import { waitForDocument, withText } from "../app.ts";

describe("the recent files list", () => {
  it("should mark a deleted file and remove it", async () => {
    await withText("File not found").waitForExist();
    await $('button[aria-label="Remove watched.pdf from recent files"]').click();
    await withText("File not found").waitForExist({ reverse: true });
  });

  it("should reopen an entry with no dialog", async () => {
    await $("button*=Relevé de compte 2026.pdf").click();
    await waitForDocument(2);
  });
});
