import type { Metadata } from "next";
import { Ubuntu } from "next/font/google";
import "./globals.css";

// const font = Inter({ subsets: ["latin"] });
const font = Ubuntu({ subsets: ["latin"], weight: "400" });

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
      <body className={font.className+" bg-white dark:bg-blue-950 dark:text-white "}>
        <div className="relative pb-24 overflow-hidden">
          <div className="flex flex-col items-center max-w-2xl w-full mx-auto">
            {children}
          </div>
        </div>
      </body>
    </html>
  );
}
