import { afterEach, describe, expect, it } from "vitest";
import { declareSetup, loadSetup } from "./setup";

const memory = new Map<string, string>();
Object.defineProperty(globalThis, "localStorage", {
  configurable: true,
  value: {
    getItem(key: string) { return memory.get(key) ?? null; },
    setItem(key: string, value: string) { memory.set(key, value); },
    removeItem(key: string) { memory.delete(key); },
    clear() { memory.clear(); },
  },
});

afterEach(() => {
  localStorage.clear();
});

describe("destination setup", () => {
  it("defaults to undeclared Free with every family available", () => {
    expect(loadSetup()).toMatchObject({
      completed: false,
      discordCap: "free",
      families: [
        "discord",
        "gmail",
        "github",
        "whatsapp",
        "x",
        "slack",
        "jpeg",
        "generic-video",
      ],
    });
  });

  it("persists a declared Discord cap without inventing detection", () => {
    const saved = declareSetup({ completed: true, discordCap: "nitro-basic", families: ["discord"] });
    expect(saved.completed).toBe(true);
    expect(loadSetup().discordCap).toBe("nitro-basic");
    expect(loadSetup().families).toEqual(["discord"]);
    expect(JSON.parse(localStorage.getItem("fitifact.setup.v1") ?? "{}").discordCap).toBe("nitro-basic");
  });
});
