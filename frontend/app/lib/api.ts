import type { CasDocumentResponse, ListPostsResponse } from "@/app/types/ast";

/**
 * Base URL of the BlogGen backend server.
 * Set NEXT_PUBLIC_API_URL in .env.local to override.
 */
export const API_URL =
  process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3000";

/**
 * Fetch the list of all posts from the server.
 *
 * Uses the JSON debug endpoint which returns the same data as the primary
 * MessagePack endpoint but in JSON format for easy browser/frontend consumption.
 */
export async function listPosts(): Promise<ListPostsResponse> {
  const res = await fetch(`${API_URL}/posts/json`, {
    // Disable Next.js cache so we always see fresh posts during development
    cache: "no-store",
  });

  if (!res.ok) {
    throw new Error(`Failed to list posts: ${res.status} ${res.statusText}`);
  }

  return res.json() as Promise<ListPostsResponse>;
}

/**
 * Fetch the full CAS AST document for a single post.
 *
 * Returns the root hash and a flat map of { blake3-hex → AstNode }.
 * The client is responsible for traversing this tree.
 */
export async function getPostAst(slug: string): Promise<CasDocumentResponse> {
  const res = await fetch(`${API_URL}/posts/${encodeURIComponent(slug)}/ast/json`, {
    cache: "no-store",
  });

  if (!res.ok) {
    if (res.status === 404) {
      throw new Error(`Post not found: ${slug}`);
    }
    throw new Error(`Failed to fetch AST for "${slug}": ${res.status} ${res.statusText}`);
  }

  return res.json() as Promise<CasDocumentResponse>;
}

/**
 * Fetch the pre-rendered HTML for a single post.
 *
 * This is the simplest rendering approach — the server does all the work.
 * See frontend/README.md for a discussion of rendering approach trade-offs.
 */
export async function getPostHtml(slug: string): Promise<string> {
  const res = await fetch(`${API_URL}/posts/${encodeURIComponent(slug)}/html`, {
    cache: "no-store",
  });

  if (!res.ok) {
    if (res.status === 404) {
      throw new Error(`Post not found: ${slug}`);
    }
    throw new Error(`Failed to fetch HTML for "${slug}": ${res.status} ${res.statusText}`);
  }

  return res.text();
}
