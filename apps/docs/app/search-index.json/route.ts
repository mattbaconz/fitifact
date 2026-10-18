import { source } from "@/lib/source";
import { createFromSource } from "fumadocs-core/search/server";

// Export a real JSON file so search works on ordinary static hosts without
// route rewrites, redirects, or a running Next server.
export const dynamic = "force-static";
export const { staticGET: GET } = createFromSource(source);
