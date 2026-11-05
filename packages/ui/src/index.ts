// @laminapdf/ui: the Paper components, presentational only (props in, events out). Import
// `@laminapdf/ui/styles.css` once for their look.

export { PageAppearanceChips, type PageAppearanceChipsProps } from "./components/appearance-chips.js";
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
export { type Swatch, SwatchGroup, type SwatchGroupProps } from "./components/swatches.js";
export {
  SegmentedTabs,
  type SegmentedTabsProps,
  type TabItem,
  TabPanel,
  type TabPanelProps,
} from "./components/tabs.js";
export { ToggleButton, type ToggleButtonProps, ToolToggle, type ToolToggleProps } from "./components/toggle.js";
export { type Density, DensityProvider, type DensityProviderProps, useDensity } from "./density.js";
