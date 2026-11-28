// Edit mode's toolbar for adding content (Paper A01, §7.24), under the top bar while Edit is the
// mode: Select · Edit text · Add text · Image · Shape ▾ · Page marks ▾, then the hint for the tool on. Each tool is its
// command's view (useCommand), so it is disabled, with the reason, when the command can't run.
// Shape ▾ is a split tool button: the toggle turns on the last shape used, and the chevron opens
// the menu of the four.

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import {
  Button as AriaButton,
  Menu as AriaMenu,
  Popover as AriaPopover,
  MenuItem,
  MenuSection,
  MenuTrigger,
  Text,
  Toolbar,
} from "react-aria-components";
import { useCommand } from "../commands/react.js";
import { ToolToggle } from "../components/toggle.js";
import { laterMessage } from "../later-messages.js";
import { POPOVER_LAYER } from "../overlays/popover.js";
import {
  CONTENT_SHAPE_ICONS,
  CONTENT_SHAPES,
  CONTENT_TOOL_ICONS,
  type ContentShape,
  type ContentTool,
  contentToolCommand,
} from "./commands.js";

/** Props of `ContentToolbar`. */
export interface ContentToolbarProps {
  /** The tool on. */
  readonly tool: ContentTool;
  /** The shape Shape ▾ draws. */
  readonly shape: ContentShape;
  /** The one-line hint for the tool on, which a polite live region says. */
  readonly hint: string;
}

/** One of the three plain tools as a toggle, pressed while it is on. */
function Tool({ tool, on }: { tool: Exclude<ContentTool, "shape">; on: boolean }): ReactNode {
  const view = useCommand(contentToolCommand(tool));
  if (view.availability.state === "hidden") return null;
  const reason = view.availability.state === "disabled" ? view.availability.reason : undefined;
  return (
    <ToolToggle
      icon={CONTENT_TOOL_ICONS[tool]}
      isSelected={on}
      isDisabled={reason !== undefined}
      data-tool={tool}
      {...(reason === undefined ? {} : { "aria-description": reason })}
      onChange={() => view.run()}
    >
      {laterMessage(`content.tool.${tool}`)}
    </ToolToggle>
  );
}

/** Shape ▾: the toggle for the last shape, and a chevron ("Choose shape") that opens the four. */
function Shapes({ on, shape }: { on: boolean; shape: ContentShape }): ReactNode {
  const view = useCommand(contentToolCommand("shape"));
  if (view.availability.state === "hidden") return null;
  const disabled = view.availability.state === "disabled";
  const reason = view.availability.state === "disabled" ? view.availability.reason : undefined;
  return (
    <div className="paper-comment-split" data-selected={on || undefined}>
      <ToolToggle
        icon={CONTENT_SHAPE_ICONS[shape]}
        isSelected={on}
        isDisabled={disabled}
        data-tool="shape"
        {...(reason === undefined ? {} : { "aria-description": reason })}
        onChange={() => view.run()}
      >
        {laterMessage("content.tool.shape")}
      </ToolToggle>
      <MenuTrigger>
        <AriaButton
          className="paper-tool-toggle paper-comment-split-menu"
          data-selected={on || undefined}
          isDisabled={disabled}
          aria-label={laterMessage("content.shapes")}
          {...(reason === undefined ? {} : { "aria-description": reason })}
        >
          <Icon name="chevron-down" size={16} />
        </AriaButton>
        <AriaPopover className="paper-popover paper-menu-popover" placement="bottom start" style={POPOVER_LAYER}>
          <AriaMenu
            className="paper-menu"
            aria-label={laterMessage("content.shapes")}
            selectionMode="single"
            selectedKeys={[shape]}
            onAction={(key) => view.run(key as ContentShape)}
          >
            <MenuSection>
              {CONTENT_SHAPES.map((kind) => (
                <MenuItem
                  key={kind}
                  id={kind}
                  textValue={laterMessage(`content.shape.${kind}`)}
                  className="paper-menu-item"
                >
                  <Icon name={CONTENT_SHAPE_ICONS[kind]} />
                  <Text slot="label" className="paper-menu-label">
                    {laterMessage(`content.shape.${kind}`)}
                  </Text>
                </MenuItem>
              ))}
            </MenuSection>
          </AriaMenu>
        </AriaPopover>
      </MenuTrigger>
    </div>
  );
}

/** The page marks commands the menu lists, with the words each shows. */
const PAGE_MARKS_MENU = [
  { id: "edit.add-watermark", label: "pagemarks.menu.watermark" },
  { id: "edit.add-header-footer", label: "pagemarks.menu.headerFooter" },
  { id: "edit.add-bates", label: "pagemarks.menu.bates" },
  { id: "edit.manage-page-marks", label: "pagemarks.menu.manage" },
] as const;

/** Page marks ▾: Watermark…, Header & footer…, Bates numbers…, then Manage page marks…; each is its command's view. */
function PageMarksMenu(): ReactNode {
  const views = [
    useCommand(PAGE_MARKS_MENU[0].id),
    useCommand(PAGE_MARKS_MENU[1].id),
    useCommand(PAGE_MARKS_MENU[2].id),
    useCommand(PAGE_MARKS_MENU[3].id),
  ];
  const shown = views.flatMap((view, i) => (view.availability.state === "hidden" ? [] : [{ view, at: i }]));
  if (shown.length === 0) return null;
  // The menu is off only when every command in it is, and says the first one's reason.
  const reasons = shown.flatMap(({ view }) =>
    view.availability.state === "disabled" ? [view.availability.reason] : [],
  );
  const reason = reasons.length === shown.length ? reasons[0] : undefined;
  return (
    <MenuTrigger>
      <AriaButton
        className="paper-tool-toggle paper-marks-menu-button"
        isDisabled={reason !== undefined}
        {...(reason === undefined ? {} : { "aria-description": reason })}
      >
        <Icon name="page-marks" size={16} />
        <span>{laterMessage("pagemarks.menu")}</span>
        <Icon name="chevron-down" size={16} />
      </AriaButton>
      <AriaPopover className="paper-popover paper-menu-popover" placement="bottom start" style={POPOVER_LAYER}>
        <AriaMenu
          className="paper-menu"
          aria-label={laterMessage("pagemarks.menu")}
          onAction={(key) => views[Number(key)]?.run()}
        >
          {shown.map(({ view, at }) => (
            <MenuItem
              key={view.id}
              id={String(at)}
              textValue={laterMessage(PAGE_MARKS_MENU[at]?.label ?? "pagemarks.menu")}
              isDisabled={view.availability.state === "disabled"}
              className="paper-menu-item"
            >
              <Text slot="label" className="paper-menu-label">
                {laterMessage(PAGE_MARKS_MENU[at]?.label ?? "pagemarks.menu")}
              </Text>
            </MenuItem>
          ))}
        </AriaMenu>
      </AriaPopover>
    </MenuTrigger>
  );
}

/** Edit mode's toolbar for adding content. */
export function ContentToolbar({ tool, shape, hint }: ContentToolbarProps): ReactNode {
  return (
    <Toolbar className="paper-content-toolbar" aria-label={laterMessage("content.toolbar")}>
      <Tool tool="select" on={tool === "select"} />
      <Tool tool="edit-text" on={tool === "edit-text"} />
      <Tool tool="text" on={tool === "text"} />
      <Tool tool="image" on={tool === "image"} />
      <Shapes on={tool === "shape"} shape={shape} />
      <PageMarksMenu />
      <span className="paper-content-hint" role="status">
        {hint}
      </span>
    </Toolbar>
  );
}
