// A worker whose one method throws a render `data-pending`, so a test can check that the error
// reaches the page with its ranges.
import { RenderError } from "@laminapdf/render";
import { serve } from "@laminapdf/sdk/worker/server";

serve({
  methods: {
    pending: {
      run: () => {
        throw new RenderError("data-pending", "2 byte ranges of the input are not available yet", [
          [0, 1024],
          [4096, 512],
        ]);
      },
    },
  },
});
