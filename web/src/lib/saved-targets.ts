import { readStored, writeStored } from "./storage";
const STORAGE_KEY = "fitifact.saved-targets.v1";
const MAX_TARGETS = 20;

export interface SavedTarget {
  id: string;
  name: string;
  requirements: string;
  constraintsJson: string;
  savedAt: string;
}

function readStore(): SavedTarget[] {
  try {
    const raw = readStored(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item): item is SavedTarget =>
      Boolean(
        item &&
          typeof item === "object" &&
          typeof (item as SavedTarget).id === "string" &&
          typeof (item as SavedTarget).name === "string" &&
          typeof (item as SavedTarget).requirements === "string" &&
          typeof (item as SavedTarget).constraintsJson === "string" &&
          typeof (item as SavedTarget).savedAt === "string",
      ),
    );
  } catch {
    return [];
  }
}

function writeStore(targets: SavedTarget[]) {
  writeStored(STORAGE_KEY, JSON.stringify(targets.slice(0, MAX_TARGETS)));
}

export function listSavedTargets(): SavedTarget[] {
  return readStore();
}

export function saveTarget(input: Omit<SavedTarget, "id" | "savedAt"> & { id?: string }): SavedTarget {
  const name = input.name.trim();
  if (!name) throw new Error("Name this target before saving it.");
  const targets = readStore();
  if (targets.some((item) => item.id !== input.id && item.name.toLocaleLowerCase() === name.toLocaleLowerCase()))
    throw new Error("That name is already in use. Choose a different name.");
  if (!targets.some((item) => item.id === input.id) && targets.length >= MAX_TARGETS)
    throw new Error("You have 20 saved targets. Remove one before saving another.");
  const next: SavedTarget = {
    id: input.id ?? crypto.randomUUID(),
    name,
    requirements: input.requirements,
    constraintsJson: input.constraintsJson,
    savedAt: new Date().toISOString(),
  };
  const existing = targets.filter((item) => item.id !== next.id);
  writeStore([next, ...existing]);
  return next;
}

export function deleteSavedTarget(id: string) {
  writeStore(readStore().filter((item) => item.id !== id));
}
