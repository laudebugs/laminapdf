// The capability page with its data: the matrix `cargo xtask capabilities` writes, bundled when
// the reader is built (O05: the page makes no request), and the reader's version.

import { message, usePlatform } from "@laminapdf/ui";
import { type CapabilityMatrix, CapabilityPage } from "@laminapdf/ui/capabilities";
import type { ReactNode } from "react";
import matrix from "../../../docs/capabilities/matrix.json" with { type: "json" };

/** How the statuses are proven: the matrix's own page, with its rules for each status. */
const HOW_TESTED = "https://github.com/laudebugs/laminapdf/blob/main/docs/capabilities/README.md";

/** The page, with Back naming the open file when there is one. */
export function CapabilityRoutePage({
  fileName,
  onBack,
}: {
  fileName: string | undefined;
  onBack: () => void;
}): ReactNode {
  const platform = usePlatform();
  return (
    <CapabilityPage
      matrix={matrix as CapabilityMatrix}
      version={__APP_VERSION__}
      backLabel={
        fileName === undefined ? message("capabilities.back.reader") : message("capabilities.back", { name: fileName })
      }
      onBack={onBack}
      onHowTested={() => void platform.openExternal(HOW_TESTED).catch(() => undefined)}
    />
  );
}
