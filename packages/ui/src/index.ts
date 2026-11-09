// @laminapdf/ui: the Paper components, presentational only (props in, events out). Import
// `@laminapdf/ui/styles.css` once for their look.

export { PHONE_QUERY } from "./breakpoints.js";
export {
  choiceArgs,
  pageArgs,
  parseChoice,
  parsePage,
  parsePageRanges,
  parseUrl,
  parseZoomPercent,
  rangeArgs,
  urlArgs,
  zoomArgs,
} from "./commands/args.js";
export {
  matchText,
  type Rankable,
  type Ranked,
  RecentCommands,
  rankCommands,
  type TextMatch,
} from "./commands/rank.js";
export {
  CommandsProvider,
  type CommandsProviderProps,
  type CommandView,
  chipKeys,
  type ShortcutGroup,
  useCommand,
  useCommands,
  useShortcutMap,
} from "./commands/react.js";
export { readerCommands, registerReaderCommands } from "./commands/reader-commands.js";
export { CommandError, CommandRegistry } from "./commands/registry.js";
export { RegistryPalette, type RegistryPaletteProps } from "./commands/registry-palette.js";
export {
  BROWSER_KEYS,
  bindingKey,
  type DispatcherOptions,
  type KeyPress,
  NEVER_BIND,
  OVERRIDES,
  pressKey,
  type Shell,
  shellOf,
  shortcutDispatcher,
  shortcutProblems,
} from "./commands/shortcuts.js";
export type {
  AnyCommand,
  ArgsSchema,
  Availability,
  Command,
  CommandContext,
  CommandGroup,
  CommandHome,
  CommandId,
  ParseResult,
  Shortcut,
  ShortcutScope,
} from "./commands/types.js";
export { PageAppearanceChips, type PageAppearanceChipsProps } from "./components/appearance-chips.js";
export { Badge, type BadgeProps, type BadgeTone } from "./components/badge.js";
export {
  Button,
  type ButtonProps,
  type ButtonSize,
  type ButtonVariant,
  IconButton,
  type IconButtonProps,
  type IconButtonSize,
} from "./components/button.js";
export {
  Checkbox,
  type CheckboxProps,
  Radio,
  RadioGroup,
  type RadioGroupProps,
  type RadioProps,
  Switch,
  type SwitchProps,
} from "./components/choice.js";
export { EmptyState, type EmptyStateProps } from "./components/empty-state.js";
export {
  type FieldText,
  Select,
  type SelectOption,
  type SelectProps,
  TextArea,
  TextInput,
  type TextInputProps,
} from "./components/field.js";
export { type Mode, ModeSwitcher, type ModeSwitcherProps } from "./components/mode-switcher.js";
export {
  Notice,
  type NoticeProps,
  type NoticeTone,
  Toast,
  type ToastAction,
  type ToastProps,
} from "./components/notice.js";
export { OutlineRow, type OutlineRowProps, type OutlineToggle } from "./components/outline-row.js";
export { ProgressBar, type ProgressBarProps, Skeleton, type SkeletonProps } from "./components/progress.js";
export {
  type KeyPlatform,
  ShortcutChip,
  type ShortcutChipProps,
  type ShortcutKeys,
} from "./components/shortcut-chip.js";
export {
  NetworkIndicator,
  type NetworkIndicatorProps,
  PageLabel,
  type PageLabelProps,
  RepairedNotice,
  type RepairedNoticeProps,
  StatusBarItem,
  type StatusBarItemProps,
} from "./components/status-bar.js";
export { type Swatch, SwatchGroup, type SwatchGroupProps } from "./components/swatches.js";
export {
  SegmentedTabs,
  type SegmentedTabsProps,
  type TabItem,
  TabPanel,
  type TabPanelProps,
} from "./components/tabs.js";
export { Thumbnail, type ThumbnailProps, type ThumbnailSize } from "./components/thumbnail.js";
export { ToggleButton, type ToggleButtonProps, ToolToggle, type ToolToggleProps } from "./components/toggle.js";
export { type Density, DensityProvider, type DensityProviderProps, useDensity } from "./density.js";
export { type Catalogue, ENGLISH, type MessageId, message } from "./messages.js";
export {
  Dialog,
  type DialogProps,
  type DialogSize,
} from "./overlays/dialog.js";
export {
  CommandPalette,
  type CommandPaletteProps,
  type PaletteItem,
  type PaletteSection,
} from "./overlays/palette.js";
export {
  Menu,
  type MenuEntry,
  type MenuProps,
  MenuTrigger,
  Popover,
  type PopoverProps,
  PopoverTrigger,
} from "./overlays/popover.js";
export {
  SelectionToolbar,
  SelectionToolbarDivider,
  type SelectionToolbarProps,
} from "./overlays/selection-toolbar.js";
export { BottomSheet, type BottomSheetProps, type SheetDetent } from "./overlays/sheet.js";
export {
  type ToastHandle,
  type ToastOptions,
  ToastQueue,
  ToastRegion,
  type ToastRegionProps,
} from "./overlays/toast-region.js";
export { Tooltip, type TooltipProps } from "./overlays/tooltip.js";
export { type CapabilityState, PlatformProvider, useCapability, usePlatform } from "./platform/context.js";
export type {
  Capabilities,
  Capability,
  Platform,
  PlatformFile,
  PlatformStorage,
  PrintJob,
  RecentFile,
  RecentFiles,
  SaveResult,
} from "./platform/types.js";
export { createWebPlatform, detectCapabilities, fromFile, type WebScope } from "./platform/web.js";
export { type DebugBackend, DebugMenu, type DebugMenuProps } from "./reader/debug-menu.js";
export { type GatedEntry, useGatedEntries } from "./reader/gate.js";
export { WholeFileDownload, type WholeFileDownloadProps } from "./reader/loading.js";
export { OpenDropZone, type OpenDropZoneProps } from "./reader/open-drop-zone.js";
export {
  currentOutlineItem,
  type OutlinePlace,
  OutlineTree,
  type OutlineTreeItem,
  type OutlineTreeProps,
} from "./reader/outline-tree.js";
export { PageInput, type PageInputProps } from "./reader/page-input.js";
export { PasswordDialog, type PasswordDialogProps } from "./reader/password-dialog.js";
export {
  OpenFailure,
  type OpenFailureProps,
  RemoteFailure,
  type RemoteFailureProps,
  type RemoteFailureReason,
  RepairDetails,
  type RepairDetailsProps,
  type RepairKind,
} from "./reader/repair.js";
export { ReaderShell, type ReaderShellProps } from "./reader/shell.js";
export { ReaderTopBar, type ReaderTopBarProps } from "./reader/top-bar.js";
export {
  MAX_ZOOM,
  MIN_ZOOM,
  pageFromInput,
  type ViewerControls,
  type ViewerSlice,
  ZOOM_PRESETS,
  type Zoom,
  zoomIn,
  zoomOut,
} from "./reader/viewer-controls.js";
export { ZoomControl, type ZoomControlProps } from "./reader/zoom-control.js";
export { type ShellActions, shellActions } from "./shell/actions.js";
export { FocusModeItem } from "./shell/focus-mode.js";
export { Inspector, type InspectorProps } from "./shell/inspector.js";
export {
  clampRailWidth,
  type InspectorContent,
  RAIL_DEFAULT_WIDTH,
  RAIL_MAX_WIDTH,
  RAIL_MIN_WIDTH,
  ShellLayout,
  type ShellLayoutState,
} from "./shell/layout.js";
export { LeftRail, type LeftRailProps, RAIL_DRAWER_QUERY, type RailTab } from "./shell/left-rail.js";
export { ModeRegistry, type ModesState, type ShellMode } from "./shell/modes.js";
export {
  type MoreItems,
  MoreSheet,
  PhoneBottomBar,
  type PhoneBottomBarProps,
  usePhone,
} from "./shell/phone.js";
export { useShell } from "./shell/use-shell.js";
export { type ExternalStore, useMediaQuery, useReaderState, useStoreState } from "./state/hooks.js";
export { type OpenedDocument, type Opener, ReaderController } from "./state/reader-controller.js";
export {
  EMPTY,
  type InteractionMode,
  type ReaderError,
  type ReaderEvent,
  type ReaderState,
  transition,
} from "./state/reader-machine.js";
