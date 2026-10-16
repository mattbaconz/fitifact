import { source } from "@/lib/source";
import {
  DocsBody,
  DocsDescription,
  DocsPage,
  DocsTitle,
} from "fumadocs-ui/page";
import defaultComponents from "fumadocs-ui/mdx";
import { notFound } from "next/navigation";
import { CopyableCode } from "../copyable-code";

export default async function Page(props: {
  params: Promise<{ slug?: string[] }>;
}) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();
  const Content = page.data.body;
  return (
    <DocsPage toc={page.data.toc} tableOfContent={{ style: "normal" }}>
      <nav aria-label="Breadcrumb" className="fitifact-breadcrumb">
        <a href={`${process.env.NEXT_PUBLIC_DOCS_BASE}/`}>Docs</a>
        <span aria-hidden="true">/</span>
        <span aria-current="page">{page.data.title}</span>
      </nav>
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
      <DocsBody>
        <Content components={{ ...defaultComponents, pre: CopyableCode }} />
      </DocsBody>
    </DocsPage>
  );
}
export function generateStaticParams() {
  return source.generateParams();
}
export async function generateMetadata(props: {
  params: Promise<{ slug?: string[] }>;
}) {
  const page = source.getPage((await props.params).slug);
  return { title: page?.data.title, description: page?.data.description };
}
