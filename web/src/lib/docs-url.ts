/** Canonical public docs URL for sharing. The in-app href is always local. */
export const PUBLIC_DOCS_URL = "https://mattbaconz.github.io/fitifact/docs/";

export function publicDocsHref(baseUrl = import.meta.env.BASE_URL) {
  const base = baseUrl.endsWith("/") ? baseUrl : `${baseUrl}/`;
  return `${base}docs/`;
}

export function desktopGuideHref(baseUrl = import.meta.env.BASE_URL) {
  return `${publicDocsHref(baseUrl)}#desktop`;
}

export function needsDesktopHandoff(message: string) {
  return /desktop app/i.test(message);
}
