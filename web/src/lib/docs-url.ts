/** Public Fumadocs on GitHub Pages. Local Vite has no /docs route. */
export const PUBLIC_DOCS_URL = "https://mattbaconz.github.io/fitifact/docs/";

export function publicDocsHref(baseUrl = import.meta.env.BASE_URL) {
  return baseUrl.includes("fitifact") ? `${baseUrl}docs/` : PUBLIC_DOCS_URL;
}
