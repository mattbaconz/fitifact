const fallback = new Map<string, string>();
const volatile = new Set<string>();

function unavailable() {
  if (typeof window !== "undefined")
    window.dispatchEvent(new Event("fitifact-storage-unavailable"));
}

export function readStored(key: string): string | null {
  if (volatile.has(key)) return fallback.get(key) ?? null;
  try {
    return localStorage.getItem(key);
  } catch {
    return fallback.get(key) ?? null;
  }
}

export function writeStored(key: string, value: string) {
  fallback.set(key, value);
  try {
    localStorage.setItem(key, value);
    volatile.delete(key);
  } catch {
    volatile.add(key);
    unavailable();
  }
}

export function removeStored(key: string) {
  fallback.delete(key);
  try {
    localStorage.removeItem(key);
    volatile.delete(key);
  } catch {
    volatile.add(key);
    unavailable();
  }
}
