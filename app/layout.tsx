import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";
import { ConvexProvider } from "@/components/providers/ConvexProvider";
import { ErrorBoundary } from "@/components/ErrorBoundary";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Cat Colony Idle Game",
  description: "A real-time idle game where a cat colony runs autonomously",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="h-full">
      <body className={`${inter.className} min-h-screen bg-meadow`}>
        <ErrorBoundary>
          <ConvexProvider>{children}</ConvexProvider>
        </ErrorBoundary>
      </body>
    </html>
  );
}



