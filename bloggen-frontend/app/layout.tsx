import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "bloggen",
  description: "blogging made confusing",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <div className="relative pb-24 overflow-hidden">
          <div className="flex flex-col items-center max-w-2xl w-full mx-auto">
            {children}
          </div>
        </div>
      </body>
    </html>
  );
}
