// The capability page (O05): an entry of its own, `@laminapdf/ui/capabilities`, which an app loads
// with import() when someone opens the page, since nothing at start needs it (#380).

export {
  type CapabilityFilter,
  type CapabilityGroup,
  type CapabilityMatrix,
  type CapabilityOp,
  type CapabilityStatus,
  type CapabilitySurface,
  filterMatrix,
} from "./help/capabilities.js";
export { CapabilityPage, type CapabilityPageProps, StatusBadge } from "./help/capability-page.js";
