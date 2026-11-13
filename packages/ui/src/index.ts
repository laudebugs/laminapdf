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
export {
  readerCommands,
  registerReaderCommands,
  registerTabCommands,
  tabCommands,
} from "./commands/reader-commands.js";
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
export {
  type EditBlockReason,
  type EditControls,
  type EditSlice,
  type EditStepSlice,
  inSentence,
  type RevertedSlice,
  saveStateText,
  sessionStarted,
  stepToRedo,
  stepToUndo,
} from "./edit/edit-controls.js";
export { type Catalogue, countMessage, ENGLISH, type MessageId, message } from "./messages.js";
export {
  Dialog,
  type DialogProps,
  type DialogSize,
} from "./overlays/dialog.js";
export { LinkPreview, type LinkPreviewImage, type LinkPreviewProps } from "./overlays/link-preview.js";
export {
  CommandPalette,
  type CommandPaletteProps,
  type PaletteItem,
  type PaletteSection,
} from "./overlays/palette.js";
export { type Box, selectionBox } from "./overlays/place.js";
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
  PrintPage,
  ReadingPosition,
  ReadingPositions,
  RecentDetails,
  RecentFile,
  RecentFiles,
  SaveResult,
} from "./platform/types.js";
export { createWebPlatform, detectCapabilities, fromFile, type WebScope } from "./platform/web.js";
export { ANNOUNCE_DELAY, type Announcements, Announcer, LiveRegions } from "./reader/announcer.js";
export {
  ACCENTS,
  type Accent,
  type Appearance,
  AppearanceFields,
  type AppearanceFieldsProps,
  AppearanceSettings,
  appearanceSummary,
  applyAppearance,
  DEFAULT_APPEARANCE,
  MOTIONS,
  type Motion,
  parseAppearance,
  UI_THEMES,
  type UiTheme,
} from "./reader/appearance.js";
export { formatBytes } from "./reader/bytes.js";
export { type DebugBackend, DebugMenu, type DebugMenuProps } from "./reader/debug-menu.js";
export { type DocumentTab, DocumentTabs, type DocumentTabsProps } from "./reader/document-tabs.js";
export {
  ExternalLinkDialog,
  type ExternalLinkDialogProps,
  type ExternalLinkTarget,
} from "./reader/external-link.js";
export {
  FindBar,
  type FindBarProps,
  type FindControls,
  type FindOptionsSlice,
  type FindSlice,
  findCount,
  findNote,
} from "./reader/find-bar.js";
export {
  type FindResultItem,
  FindResults,
  type FindResultsControls,
  type FindResultsProps,
} from "./reader/find-results.js";
export { type GatedEntry, useGatedEntries } from "./reader/gate.js";
export { InstallDialog, type InstallDialogProps } from "./reader/install-dialog.js";
export { WholeFileDownload, type WholeFileDownloadProps } from "./reader/loading.js";
export {
  NetworkActivityPanel,
  type NetworkActivityPanelProps,
  type NetworkAppRow,
  type NetworkBlockedRow,
  type NetworkOriginRow,
  networkIndicatorText,
} from "./reader/network.js";
export { OfflineNotice, type OfflineNoticeProps } from "./reader/offline-notice.js";
export { DropOverlay, OpenDropZone, type OpenDropZoneProps } from "./reader/open-drop-zone.js";
export {
  currentOutlineItem,
  type OutlinePlace,
  OutlineTree,
  type OutlineTreeItem,
  type OutlineTreeProps,
  outlineItemAt,
} from "./reader/outline-tree.js";
export {
  PageAppearanceFields,
  type PageAppearanceFieldsProps,
  type PageAppearanceScope,
  pageAppearanceName,
} from "./reader/page-appearance.js";
export { PageInput, type PageInputProps } from "./reader/page-input.js";
export { PasswordDialog, type PasswordDialogProps } from "./reader/password-dialog.js";
export { type PrintPagesResult, parsePrintPages } from "./reader/print-pages.js";
export {
  type DocumentPropertiesData,
  formatFileSize,
  type PropertiesCipher,
  type PropertiesContext,
  type PropertyRow,
  type PropertySection,
  pageSizeSummary,
  propertiesText,
  readXmp,
  type XmpFields,
} from "./reader/properties.js";
export {
  DEFAULT_READER_SETTINGS,
  READER_TEXT_SIZES,
  type ReaderLineLength,
  type ReaderLineSpacing,
  type ReaderSettings,
  readerTextSize,
} from "./reader/reader-settings.js";
export type {
  ReaderApproximation,
  ReaderBlock,
  ReaderSettingsPanelProps,
  ReaderViewContent,
  ReaderViewProps,
} from "./reader/reader-view.js";
export { type ContinueReading, RecentList, type RecentListProps, recentMeta } from "./reader/recent-list.js";
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
export {
  SettingsDialog,
  type SettingsDialogProps,
  SettingsRegistry,
  type SettingsSection,
  useSettingsSection,
  ViewSettingsButton,
  type ViewSettingsButtonProps,
} from "./reader/settings.js";
export { ReaderShell, type ReaderShellProps } from "./reader/shell.js";
export { ShortcutsDialog, type ShortcutsDialogProps } from "./reader/shortcuts-dialog.js";
export { stepThreadPart, type ThreadData, type ThreadPartRef } from "./reader/threads.js";
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
  SelectionActionBar,
  type SelectionActionBarProps,
  usePhone,
} from "./shell/phone.js";
export { useShell } from "./shell/use-shell.js";
export { type ExternalStore, useMediaQuery, useReaderState, useStoreState } from "./state/hooks.js";
export { type CloseGuard, type OpenedDocument, type Opener, ReaderController } from "./state/reader-controller.js";
export {
  EMPTY,
  type ReaderError,
  type ReaderEvent,
  type ReaderState,
  transition,
} from "./state/reader-machine.js";
export type { InteractionMode, InteractionModes, ToolHost, ToolModule, ToolsState } from "./tools/tools.js";
