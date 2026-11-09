import { describe, expect, it } from "vitest";
import { InstallPrompt, type InstallScope } from "./install.tsx";

/** A window the test sends events through, and an offer whose answer it chooses. */
function fakes() {
  const listeners = new Map<string, (event: Event) => void>();
  const scope: InstallScope = { addEventListener: (type, l) => listeners.set(type, l) };
  const offer = (outcome: "accepted" | "dismissed") => {
    let prevented = false;
    let prompted = 0;
    const event = Object.assign(new Event("beforeinstallprompt"), {
      preventDefault: () => {
        prevented = true;
      },
      prompt: async () => {
        prompted++;
      },
      userChoice: Promise.resolve({ outcome }),
    });
    listeners.get("beforeinstallprompt")?.(event);
    return { prevented: () => prevented, prompted: () => prompted };
  };
  return { scope, offer, installed: () => listeners.get("appinstalled")?.(new Event("appinstalled")) };
}

describe("InstallPrompt", () => {
  it("keeps the browser's offer instead of its banner, and shows it on demand", async () => {
    const { scope, offer } = fakes();
    const prompt = new InstallPrompt(scope);
    expect(prompt.getState()).toBe(false);
    const made = offer("accepted");
    expect(made.prevented()).toBe(true);
    expect(prompt.getState()).toBe(true);
    await prompt.prompt();
    expect(made.prompted()).toBe(1);
    expect(prompt.getState()).toBe(false);
  });

  it("uses an offer once, and forgets it once the reader is installed", async () => {
    const { scope, offer, installed } = fakes();
    const prompt = new InstallPrompt(scope);
    offer("dismissed");
    await prompt.prompt();
    expect(prompt.getState()).toBe(false);
    offer("accepted");
    installed();
    expect(prompt.getState()).toBe(false);
  });
});
