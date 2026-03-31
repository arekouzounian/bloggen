import { listPosts } from "@/app/lib/api";
import { PostList } from "@/app/components/PostList";

export default async function HomePage() {
  let posts = null;
  let error: string | null = null;

  try {
    const data = await listPosts();
    posts = data.posts;
  } catch (e) {
    error = e instanceof Error ? e.message : "Unknown error";
  }

  return (
    <div>
      <h1 className="text-3xl font-bold mb-6">Posts</h1>

      {error ? (
        <div className="border border-red-300 bg-red-50 rounded p-4 text-sm text-red-700">
          <p className="font-semibold">Could not connect to BlogGen server</p>
          <p className="mt-1 font-mono text-xs">{error}</p>
          <p className="mt-2 text-gray-600">
            Make sure the server is running at{" "}
            <code className="font-mono bg-gray-100 px-1 rounded">
              {process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3000"}
            </code>
            .
          </p>
        </div>
      ) : posts ? (
        <PostList posts={posts} />
      ) : null}
    </div>
  );
}
