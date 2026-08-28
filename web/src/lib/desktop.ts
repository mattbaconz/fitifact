const PROFILE_ID = /^[a-z0-9-]+\/[a-z0-9-]+$/i;

export function isProfileId(value: string): boolean {
  return PROFILE_ID.test(value.trim());
}

export interface DesktopTarget {
  profile?: string;
  constraintsJson?: string;
  constraintsYaml?: string;
}

export function isDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function desktopTargetFromText(text: string): DesktopTarget | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  const forMatch = trimmed.match(/^--for\s+(\S+)/i);
  if (forMatch) return { profile: forMatch[1] };
  if (PROFILE_ID.test(trimmed)) return { profile: trimmed };
  if (trimmed.startsWith("{")) return { constraintsJson: trimmed };
  if (/(^|\n)\s*schema:\s*/.test(trimmed)) return { constraintsYaml: trimmed };
  return null;
}

export function constraintsLookLikeImage(json: string | null): boolean {
  if (!json) return false;
  return json.includes("image.format") || json.includes("image.width") || json.includes("image.height");
}

export function constraintsLookLikeMedia(json: string | null): boolean {
  if (!json) return false;
  return json.includes("media.container") || json.includes("media.video") || json.includes("media.audio");
}

export function fileNameFromPath(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export function fileFromBytes(bytes: Uint8Array, name: string, type?: string): File {
  const copy = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(copy).set(bytes);
  return new File([copy], name, type ? { type } : undefined);
}
