import type { Metadata } from "next";
import {
  JetBrains_Mono,
  Press_Start_2P,
  VT323,
  Inter,
  Caveat,
  Nunito,
  Space_Grotesk,
  Space_Mono,
  Playfair_Display,
  Special_Elite,
} from "next/font/google";
import "./globals.css";
import { ConvexProvider } from "@/components/providers/ConvexProvider";
import { ErrorBoundary } from "@/components/ErrorBoundary";

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains",
  display: "swap",
});

const pressStart2P = Press_Start_2P({
  weight: "400",
  subsets: ["latin"],
  variable: "--font-press-start",
  display: "swap",
});

const vt323 = VT323({
  weight: "400",
  subsets: ["latin"],
  variable: "--font-vt323",
  display: "swap",
});

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

const caveat = Caveat({
  subsets: ["latin"],
  variable: "--font-caveat",
  display: "swap",
});

const nunito = Nunito({
  subsets: ["latin"],
  variable: "--font-nunito",
  display: "swap",
});

const spaceGrotesk = Space_Grotesk({
  subsets: ["latin"],
  variable: "--font-space-grotesk",
  display: "swap",
});

const spaceMono = Space_Mono({
  weight: ["400", "700"],
  subsets: ["latin"],
  variable: "--font-space-mono",
  display: "swap",
});

const playfairDisplay = Playfair_Display({
  subsets: ["latin"],
  variable: "--font-playfair",
  display: "swap",
});

const specialElite = Special_Elite({
  weight: "400",
  subsets: ["latin"],
  variable: "--font-special-elite",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Global Cat Colony Idle",
  description:
    "A shared browser idle game with an always-on backend simulation",
};

const fonts = [
  jetbrainsMono,
  pressStart2P,
  vt323,
  inter,
  caveat,
  nunito,
  spaceGrotesk,
  spaceMono,
  playfairDisplay,
  specialElite,
];
const fontVariables = fonts.map((f) => f.variable).join(" ");

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={`h-full ${fontVariables}`}>
      <body className="min-h-screen bg-meadow">
        <ErrorBoundary>
          <ConvexProvider>{children}</ConvexProvider>
        </ErrorBoundary>
      </body>
    </html>
  );
}
