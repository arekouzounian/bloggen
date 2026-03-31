import { getPostAst } from "@/app/lib/api";
import { AstViewer } from "@/app/components/AstViewer";
import Link from "next/link";

interface PostPageProps {
  params: { slug: string };
}

export default async function PostPage({ params }: PostPageProps) {
  const { slug } = params;
  let doc = null;
  let error: string | null = null;

  try {
    doc = await getPostAst(slug);
  } catch (e) {
    error = e instanceof Error ? e.message : "Unknown error";
  }

  return (
    <div>
      <Link href="/" className="text-sm text-gray-500 hover:text-gray-900">
        ← All posts
      </Link>

      <h1 className="text-2xl font-bold mt-4 mb-2 font-mono">{slug}</h1>

      {error ? (
        <div className="border border-red-300 bg-red-50 rounded p-4 text-sm text-red-700 mt-4">
          <p className="font-semibold">Failed to load post</p>
          <p className="mt-1 font-mono text-xs">{error}</p>
        </div>
      ) : doc ? (
        <div className="mt-4">
          <p className="text-xs text-gray-400 mb-4">
            ⚠ Proof-of-concept view: displaying raw AST. See{" "}
            <a
              href="https://github.com/arekouzounian/bloggen/blob/v2/frontend/README.md"
              target="_blank"
              rel="noreferrer"
              className="underline"
            >
              frontend/README.md
            </a>{" "}
            for a discussion of rendering approaches.
          </p>
          <AstViewer doc={doc} />
        </div>
      ) : null}
    </div>
  );
}
