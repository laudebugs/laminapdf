// The download page's browser side (src/pages/download.astro): shows each channel's card for the
// visitor's computer, guessed from User-Agent Client Hints where the browser has them (Chromium)
// and the user agent string elsewhere (src/data/downloads.ts' visitorFrom), and turns on the copy
// buttons. Without it the page keeps "Choose your system below" and the full table.

import { offeredPlatforms, type Visitor, visitorFrom, WORDS } from "../data/downloads.js";

interface HighEntropyHints {
  platform?: string;
  architecture?: string;
  mobile?: boolean;
}

interface NavigatorWithHints {
  userAgentData?: { getHighEntropyValues(keys: string[]): Promise<HighEntropyHints> };
}

async function visitor(): Promise<Visitor> {
  const data = (navigator as Navigator & NavigatorWithHints).userAgentData;
  let hints: HighEntropyHints | undefined;
  try {
    hints = await data?.getHighEntropyValues(["platform", "architecture"]);
  } catch {
    hints = undefined;
  }
  return visitorFrom(hints, navigator.userAgent);
}

/** Shows `pane`'s card for `who`: their installers, the phone's note, or "Choose your system below". */
function showCard(pane: HTMLElement, who: Visitor): void {
  const offered = offeredPlatforms(who);
  const offers = [...pane.querySelectorAll<HTMLElement>("[data-card]")];
  const matching = offers.filter((card) => offered.includes(card.dataset.card ?? ""));
  const shown =
    matching.length > 0 ? matching : offers.filter((card) => card.dataset.card === (who.mobile ? "phone" : "default"));
  for (const card of offers) card.hidden = !shown.includes(card);
}

function enableCopy(status: HTMLElement | null): void {
  if (navigator.clipboard === undefined) return;
  for (const button of document.querySelectorAll<HTMLButtonElement>("[data-copy]")) {
    button.hidden = false;
    button.addEventListener("click", () => {
      navigator.clipboard.writeText(button.dataset.copy ?? "").then(
        () => {
          if (status !== null) status.textContent = WORDS.copied;
        },
        () => {
          if (status !== null) status.textContent = WORDS.copyFailed;
        },
      );
    });
  }
}

enableCopy(document.querySelector<HTMLElement>("[data-copy-status]"));
visitor().then(
  (who) => {
    for (const pane of document.querySelectorAll<HTMLElement>(".download-channels [data-pane]")) showCard(pane, who);
  },
  (error: unknown) => console.error("the download page couldn't read the browser's system", error),
);
