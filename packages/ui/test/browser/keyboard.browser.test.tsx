import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userEvent } from "vitest/browser";
import {
  Button,
  Checkbox,
  ModeSwitcher,
  PageAppearanceChips,
  Radio,
  RadioGroup,
  SegmentedTabs,
  Select,
  SwatchGroup,
  Switch,
  TabPanel,
  TextInput,
  ToggleButton,
} from "../../src/index.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(cleanup);

const get = (host: HTMLElement, selector: string) => host.querySelector(selector) as HTMLElement;
const all = (host: HTMLElement, selector: string) => [...host.querySelectorAll<HTMLElement>(selector)];
const focused = () => document.activeElement as HTMLElement;

function Modes({ onChange }: { onChange: (m: string) => void }) {
  const [mode, setMode] = useState<"read" | "comment">("read");
  const modes = [
    { id: "read", label: "Read" },
    { id: "comment", label: "Comment" },
  ] as const;
  return (
    <ModeSwitcher
      aria-label="Mode"
      modes={modes}
      value={mode}
      onChange={(m) => {
        onChange(m);
        setMode(m);
      }}
    />
  );
}

function Rail({ manual = false }: { manual?: boolean }) {
  const tabs = [
    { id: "pages", label: "Pages" },
    { id: "outline", label: "Outline" },
    { id: "files", label: "Files" },
  ] as const;
  const [tab, setTab] = useState<(typeof tabs)[number]["id"]>("pages");
  return (
    <SegmentedTabs
      aria-label="Sidebar"
      tabs={tabs}
      selectedKey={tab}
      onSelectionChange={setTab}
      keyboardActivation={manual ? "manual" : "automatic"}
    >
      {tabs.map((t) => (
        <TabPanel key={t.id} id={t.id}>
          {t.label} panel
        </TabPanel>
      ))}
    </SegmentedTabs>
  );
}

describe("Button", () => {
  it("presses on Enter and Space, and lets Escape through", async () => {
    const onPress = vi.fn();
    const onEscape = vi.fn();
    const listener = (e: KeyboardEvent) => e.key === "Escape" && onEscape();
    document.addEventListener("keydown", listener);
    const host = renderIn(<Button onPress={onPress}>Save</Button>);
    await userEvent.tab();
    expect(focused()).toBe(get(host, "button"));
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard(" ");
    expect(onPress).toHaveBeenCalledTimes(2);
    await userEvent.keyboard("{Escape}");
    document.removeEventListener("keydown", listener);
    expect(onEscape).toHaveBeenCalledTimes(1);
  });

  it("skips a disabled button and keeps a pending one focusable but unpressable", async () => {
    const onPress = vi.fn();
    renderIn(
      <>
        <Button isDisabled onPress={onPress}>
          Off
        </Button>
        <Button isPending onPress={onPress}>
          Saving
        </Button>
      </>,
    );
    await userEvent.tab();
    expect(focused().textContent).toBe("Saving");
    await userEvent.keyboard("{Enter}");
    expect(onPress).not.toHaveBeenCalled();
  });
});

describe("ToggleButton", () => {
  it("toggles aria-pressed on Space", async () => {
    const host = renderIn(<ToggleButton icon="sidebar">Sidebar</ToggleButton>);
    await userEvent.tab();
    expect(get(host, "button").getAttribute("aria-pressed")).toBe("false");
    await userEvent.keyboard(" ");
    expect(get(host, "button").getAttribute("aria-pressed")).toBe("true");
  });
});

describe("ModeSwitcher", () => {
  it("is a group of pressed buttons, each its own Tab stop", async () => {
    const onChange = vi.fn();
    const host = renderIn(<Modes onChange={onChange} />);
    expect(get(host, "[role=group]").getAttribute("aria-label")).toBe("Mode");
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    expect(onChange).not.toHaveBeenCalled();
    await userEvent.tab();
    expect(focused().textContent).toBe("Comment");
    await userEvent.keyboard("{Enter}");
    expect(onChange).toHaveBeenCalledWith("comment");
    expect(all(host, "button").map((b) => b.getAttribute("aria-pressed"))).toEqual(["false", "true"]);
  });
});

describe("SegmentedTabs", () => {
  const selected = (host: HTMLElement) => get(host, "[role=tab][aria-selected=true]").textContent;

  it("selects with the arrow keys, Home and End", async () => {
    const host = renderIn(<Rail />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowRight}");
    expect(selected(host)).toBe("Outline");
    await userEvent.keyboard("{End}");
    expect(selected(host)).toBe("Files");
    await userEvent.keyboard("{Home}");
    expect(selected(host)).toBe("Pages");
    expect(get(host, "[role=tabpanel]").textContent).toBe("Pages panel");
  });

  it("only moves focus with manual activation, until Enter", async () => {
    const host = renderIn(<Rail manual />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowRight}");
    expect(selected(host)).toBe("Pages");
    await userEvent.keyboard("{Enter}");
    expect(selected(host)).toBe("Outline");
  });
});

describe("fields", () => {
  it("wire the label, help and error to the input", () => {
    const host = renderIn(<TextInput label="Page" description="1 to 24" errorMessage="Enter a page from 1 to 24" />);
    const input = get(host, "input");
    expect(input.getAttribute("aria-invalid")).toBe("true");
    const described = (input.getAttribute("aria-describedby") ?? "")
      .split(" ")
      .map((id) => document.getElementById(id)?.textContent);
    expect(described).toContain("Enter a page from 1 to 24");
    expect(get(host, "label").getAttribute("for")).toBe(input.id);
  });

  it("wire the select the same way, and report the chosen value", async () => {
    const onChange = vi.fn();
    const options = [
      { value: "a4", label: "A4" },
      { value: "letter", label: "Letter" },
    ];
    const host = renderIn(
      <Select label="Size" options={options} value="a4" onChange={onChange} errorMessage="Not supported" />,
    );
    const select = get(host, "select") as HTMLSelectElement;
    expect(select.getAttribute("aria-invalid")).toBe("true");
    expect(document.getElementById(select.getAttribute("aria-describedby") ?? "")?.textContent).toBe("Not supported");
    expect(get(host, "label").getAttribute("for")).toBe(select.id);
    await userEvent.selectOptions(select, "letter");
    expect(onChange).toHaveBeenCalledWith("letter");
  });
});

describe("choices", () => {
  it("toggle a checkbox with Space, and show mixed as indeterminate", async () => {
    const host = renderIn(
      <>
        <Checkbox>One</Checkbox>
        <Checkbox isIndeterminate>All</Checkbox>
      </>,
    );
    const [one, mixed] = all(host, "input") as HTMLInputElement[];
    await userEvent.tab();
    await userEvent.keyboard(" ");
    expect(one?.checked).toBe(true);
    expect(mixed?.indeterminate).toBe(true);
  });

  it("move a radio group's choice with the arrow keys", async () => {
    const host = renderIn(
      <RadioGroup label="Pages" defaultValue="all">
        <Radio value="all">All</Radio>
        <Radio value="current">Current</Radio>
      </RadioGroup>,
    );
    expect(get(host, "[role=radiogroup]")).toBeTruthy();
    await userEvent.tab();
    await userEvent.keyboard("{ArrowDown}");
    expect((all(host, "input") as HTMLInputElement[]).map((i) => i.checked)).toEqual([false, true]);
  });

  it("toggle a switch with Space", async () => {
    const host = renderIn(<Switch>Thumbnails</Switch>);
    const input = get(host, "input") as HTMLInputElement;
    expect(input.getAttribute("role")).toBe("switch");
    await userEvent.tab();
    await userEvent.keyboard(" ");
    expect(input.checked).toBe(true);
  });
});

describe("pickers", () => {
  it("name each swatch by colour and press the chosen one", async () => {
    const onChange = vi.fn();
    const swatches = [
      { id: "yellow", label: "Yellow", color: "annotation-yellow" },
      { id: "blue", label: "Blue", color: "annotation-blue" },
    ] as const;
    const host = renderIn(<SwatchGroup label="Colour" swatches={swatches} value="yellow" onChange={onChange} />);
    expect(all(host, "button").map((b) => [b.getAttribute("aria-label"), b.getAttribute("aria-pressed")])).toEqual([
      ["Yellow", "true"],
      ["Blue", "false"],
    ]);
    await userEvent.tab();
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    expect(onChange).toHaveBeenCalledWith("blue");
  });

  it("press the chosen page appearance", async () => {
    const onChange = vi.fn();
    const labels = { original: "Original", sepia: "Sepia", dark: "Dark", "high-contrast": "High contrast" };
    const host = renderIn(
      <PageAppearanceChips aria-label="Page appearance" labels={labels} value="original" onChange={onChange} />,
    );
    expect(all(host, "button[aria-pressed=true]").map((b) => b.textContent)).toEqual(["AaOriginal"]);
    await userEvent.click(all(host, "button")[2] as HTMLElement);
    expect(onChange).toHaveBeenCalledWith("dark");
  });
});
