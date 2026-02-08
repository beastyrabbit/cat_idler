import type { Metadata } from "next";
import "./globals.css";
import { ConvexProvider } from "@/components/providers/ConvexProvider";
import { ErrorBoundary } from "@/components/ErrorBoundary";

export const metadata: Metadata = {
  title: "Global Cat Colony Idle",
  description: "A shared browser idle game with an always-on backend simulation",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="h-full">
      <body className="min-h-screen bg-meadow">
        <ErrorBoundary>
          <ConvexProvider>{children}</ConvexProvider>
        </ErrorBoundary>
      </body>
    </html>
  );
}

