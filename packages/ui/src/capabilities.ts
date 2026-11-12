// The capability page (O05): an entry of its own, `@laminapdf/ui/capabilities`, which an app loads
// with import() when someone opens the page, since nothing at start needs it (#380).

export type {
  CapabilityGroup,
  CapabilityMatrix,
  CapabilityOp,
  CapabilityStatus,
  CapabilitySurface,
} from "./help/capabilities.js";
export { CapabilityPage, type CapabilityPageProps } from "./help/capability-page.js";
