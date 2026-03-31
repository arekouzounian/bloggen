import Link from "next/link";
import type { PostSummary } from "@/app/types/ast";

interface PostListProps {
  posts: PostSummary[];
}

/** Formats an ISO 8601 timestamp into a human-readable date string */
function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

export function PostList({ posts }: PostListProps) {
  if (posts.length === 0) {
    return (
      <p className="text-gray-500 italic">
        No posts yet. Upload one with the <code>bgc</code> CLI.
      </p>
    );
  }

  return (
    <ul className="space-y-6">
      {posts.map((post) => (
        <li key={post.slug} className="border-b pb-4 last:border-b-0">
          <Link
            href={`/posts/${post.slug}`}
            className="text-xl font-semibold text-blue-700 hover:underline"
          >
            {post.title ?? post.slug}
          </Link>
          <p className="text-sm text-gray-500 mt-1">
            {formatDate(post.created_at)}
            {!post.published && (
              <span className="ml-2 px-1.5 py-0.5 bg-yellow-100 text-yellow-800 rounded text-xs font-medium">
                draft
              </span>
            )}
          </p>
          <p className="text-xs text-gray-400 font-mono mt-1">/{post.slug}</p>
        </li>
      ))}
    </ul>
  );
}
