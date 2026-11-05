// @laminapdf/ui: the Paper components, presentational only (props in, events out). Import
// `@laminapdf/ui/styles.css` once for their look.

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
  Dialog,
  type DialogProps,
  type DialogSize,
} from "./overlays/dialog.js";
export {
  Menu,
  type MenuEntry,
  type MenuProps,
  MenuTrigger,
  Popover,
  type PopoverProps,
  PopoverTrigger,
} from "./overlays/popover.js";
export { BottomSheet, type BottomSheetProps, type SheetDetent } from "./overlays/sheet.js";
export {
  type ToastHandle,
  type ToastOptions,
  ToastQueue,
  ToastRegion,
  type ToastRegionProps,
} from "./overlays/toast-region.js";
export { Tooltip, type TooltipProps } from "./overlays/tooltip.js";
