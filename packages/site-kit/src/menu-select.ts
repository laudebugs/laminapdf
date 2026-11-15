// The behaviour of MenuSelect.astro: ARIA's select-only combobox
// (https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-select-only/). The button
// keeps the focus while the list is open and points at the active option with
// aria-activedescendant. Closed, Down, Up, Enter, Space or a letter opens it; open, Down and Up
// move (Home and End to the ends, a letter to the next option it starts), Enter, Space or Tab
// chooses, and Escape closes. A pointer chooses with a click; hovering highlights an option but
// never moves the keyboard's, so Tab never chooses what the pointer passed over. A click outside
// closes. Choosing sets `value` and dispatches `change`, which bubbles.

let count = 0;

/** HTMLElement, or a stand-in where there is none: Astro's renderer loads this module in Node. */
const Element_ = (globalThis.HTMLElement ?? class {}) as typeof HTMLElement;

/** <site-menu-select>: the chosen value in its `value` attribute, shown by the `data-for` elements. */
export class SiteMenuSelect extends Element_ {
  static readonly observedAttributes = ["value"];

  #button: HTMLButtonElement | null = null;
  #list: HTMLElement | null = null;
  #active = -1;
  #outside = (event: PointerEvent) => {
    if (!(event.target instanceof Node) || !this.contains(event.target)) this.#close();
  };

  /** The chosen value. */
  get value(): string {
    return this.getAttribute("value") ?? "";
  }

  set value(value: string) {
    this.setAttribute("value", value);
  }

  connectedCallback(): void {
    if (this.#button !== null) return;
    this.#button = this.querySelector(".site-menu-select-button");
    this.#list = this.querySelector(".site-menu-select-list");
    if (this.#button === null || this.#list === null) return;
    const button = this.#button;
    // Ids are given here, not in the markup: a page can hold the same select twice (the docs
    // site's header and phone menu), and only a working select needs them.
    count += 1;
    const id = `site-menu-select-${count}`;
    this.#list.id = `${id}-list`;
    for (const [index, option] of this.#options().entries()) option.id = `${id}-option-${index}`;
    button.setAttribute("aria-controls", this.#list.id);

    button.addEventListener("click", () => (this.#isOpen() ? this.#close() : this.#open(this.#selected())));
    button.addEventListener("keydown", (event) => this.#key(event));
    // Space is handled on keydown; its keyup would click the button again (Firefox does).
    button.addEventListener("keyup", (event) => {
      if (event.key === " ") event.preventDefault();
    });
    // A press on a shown label reaches the button, as a <label>'s would.
    this.querySelector(".site-menu-select-label")?.addEventListener("click", () => button.focus());
    this.#button.addEventListener("blur", () => this.#close());
    // The button keeps the focus: a press on the list must not take it.
    this.#list.addEventListener("pointerdown", (event) => event.preventDefault());
    this.#list.addEventListener("click", (event) => {
      const option = event.target instanceof Element ? event.target.closest("[role=option]") : null;
      if (option instanceof HTMLElement) this.#choose(this.#options().indexOf(option));
    });
    this.#show();
  }

  disconnectedCallback(): void {
    this.#close();
  }

  attributeChangedCallback(): void {
    this.#show();
  }

  #options(): HTMLElement[] {
    return this.#list === null ? [] : [...this.#list.querySelectorAll<HTMLElement>("[role=option]")];
  }

  #selected(): number {
    return Math.max(
      0,
      this.#options().findIndex((option) => option.dataset.value === this.value),
    );
  }

  #isOpen(): boolean {
    return this.#list !== null && !this.#list.hidden;
  }

  /** Marks the chosen option and shows its icon and label in the button. */
  #show(): void {
    for (const option of this.#options()) {
      option.setAttribute("aria-selected", String(option.dataset.value === this.value));
    }
    for (const current of this.querySelectorAll<HTMLElement>("[data-for]")) {
      current.hidden = current.dataset.for !== this.value;
    }
  }

  #open(active: number): void {
    if (this.#button === null || this.#list === null) return;
    this.#list.hidden = false;
    this.#button.setAttribute("aria-expanded", "true");
    // Up when the window has no room below and more above (the docs' phone menu ends at the bottom).
    this.classList.remove("site-menu-select-up");
    const button = this.#button.getBoundingClientRect();
    const height = this.#list.getBoundingClientRect().height;
    if (button.bottom + height > innerHeight && button.top > innerHeight - button.bottom) {
      this.classList.add("site-menu-select-up");
    }
    // Under the button's start when, under its end, the list would cross the window's start.
    this.classList.remove("site-menu-select-start");
    if (this.#list.getBoundingClientRect().left < 0) this.classList.add("site-menu-select-start");
    this.#activate(active);
    document.addEventListener("pointerdown", this.#outside);
  }

  #close(): void {
    if (this.#button === null || this.#list === null || !this.#isOpen()) return;
    this.#list.hidden = true;
    this.#button.setAttribute("aria-expanded", "false");
    this.#button.removeAttribute("aria-activedescendant");
    for (const option of this.#options()) delete option.dataset.active;
    this.#active = -1;
    document.removeEventListener("pointerdown", this.#outside);
  }

  #activate(index: number): void {
    const options = this.#options();
    const option = options[index];
    if (option === undefined || this.#button === null) return;
    for (const other of options) delete other.dataset.active;
    option.dataset.active = "";
    this.#active = index;
    this.#button.setAttribute("aria-activedescendant", option.id);
    option.scrollIntoView({ block: "nearest" });
  }

  #choose(index: number): void {
    const option = this.#options()[index];
    this.#close();
    if (option?.dataset.value === undefined) return;
    const changed = option.dataset.value !== this.value;
    this.value = option.dataset.value;
    if (changed) this.dispatchEvent(new Event("change", { bubbles: true }));
  }

  /** The index of the next option after `from` whose label starts with `letter`, or -1. */
  #byLetter(letter: string, from: number): number {
    const options = this.#options();
    for (let step = 1; step <= options.length; step += 1) {
      const index = (from + step) % options.length;
      const text = options[index]?.textContent?.trim().toLowerCase() ?? "";
      if (text.startsWith(letter.toLowerCase())) return index;
    }
    return -1;
  }

  #key(event: KeyboardEvent): void {
    if (this.#isOpen() ? this.#keyOpen(event) : this.#keyClosed(event)) event.preventDefault();
  }

  /** Opens the list on the option a key names; whether the key was one. */
  #keyClosed(event: KeyboardEvent): boolean {
    const selected = this.#selected();
    let target: number | undefined;
    if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) target = selected;
    else if (event.key === "Home") target = 0;
    else if (event.key === "End") target = this.#options().length - 1;
    else if (isLetter(event)) {
      const found = this.#byLetter(event.key, selected);
      target = found === -1 ? selected : found;
    }
    if (target === undefined) return false;
    this.#open(target);
    return true;
  }

  /** Moves, chooses or closes for a key; whether the key was one. */
  #keyOpen(event: KeyboardEvent): boolean {
    if (event.key === "Tab") {
      // Tab chooses and moves on: the focus leaves as usual.
      this.#choose(this.#active);
      return false;
    }
    if (event.key === "Enter" || event.key === " " || (event.key === "ArrowUp" && event.altKey)) {
      this.#choose(this.#active);
      return true;
    }
    if (event.key === "Escape") {
      this.#close();
      return true;
    }
    const target = this.#moveTo(event);
    if (target === undefined) return false;
    this.#activate(target);
    return true;
  }

  /** The option a key moves to in the open list, or undefined. */
  #moveTo(event: KeyboardEvent): number | undefined {
    const last = this.#options().length - 1;
    if (event.key === "ArrowDown") return Math.min(last, this.#active + 1);
    if (event.key === "ArrowUp") return Math.max(0, this.#active - 1);
    if (event.key === "Home") return 0;
    if (event.key === "End") return last;
    if (!isLetter(event)) return undefined;
    const found = this.#byLetter(event.key, this.#active);
    return found === -1 ? undefined : found;
  }
}

/** Whether a key is a letter typed to jump to an option (not Space, not a shortcut). */
function isLetter(event: KeyboardEvent): boolean {
  return event.key.length === 1 && event.key !== " " && !event.ctrlKey && !event.metaKey && !event.altKey;
}

/** Defines <site-menu-select> once, however many selects a page holds; nothing outside a browser. */
export function defineMenuSelect(): void {
  if (globalThis.customElements === undefined || customElements.get("site-menu-select") !== undefined) return;
  customElements.define("site-menu-select", SiteMenuSelect);
}
