import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "BlogGen",
  description: "A BlogGen v2 frontend",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="min-h-screen bg-white text-gray-900 antialiased">
        <header className="border-b">
          <div className="max-w-3xl mx-auto px-4 py-4 flex items-center justify-between">
            <a href="/" className="text-xl font-bold tracking-tight hover:opacity-75">
              BlogGen
            </a>
            <nav className="text-sm text-gray-500 space-x-4">
              <a href="/" className="hover:text-gray-900">Posts</a>
            </nav>
          </div>
        </header>
        <main className="max-w-3xl mx-auto px-4 py-8">
          {children}
        </main>
        <footer className="border-t mt-16">
          <div className="max-w-3xl mx-auto px-4 py-6 text-xs text-gray-400">
            BlogGen v2 — content-addressable blogging
          </div>
        </footer>
      </body>
    </html>
  );
}
