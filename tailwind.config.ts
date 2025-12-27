import type { Config } from "tailwindcss";
import animate from "tailwindcss-animate";

const config: Config = {
  content: [
    "./pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        background: "var(--background)",
        foreground: "var(--foreground)",

        // Cozy palette
        cream: {
          50: "#FFFBF5",
          100: "#FFF7EB",
          200: "#FFEDD5",
          300: "#FED7AA",
          400: "#FDBA74",
          500: "#FB923C",
          600: "#F97316",
          700: "#EA580C",
          800: "#C2410C",
          900: "#9A3412",
        },
        sage: {
          50: "#F4F7F4",
          100: "#E8EFE8",
          200: "#D2E0D2",
          300: "#B7CFB9",
          400: "#9ABDA0",
          500: "#7A9E7E",
          600: "#5F8363",
          700: "#4B6A4F",
          800: "#3E5641",
          900: "#334737",
        },
        peach: {
          50: "#FFF5F0",
          100: "#FFE7DA",
          200: "#FFD0B6",
          300: "#FFB185",
          400: "#FF945A",
          500: "#F5A676",
          600: "#EA7B3C",
          700: "#D45E1F",
          800: "#AD4C1B",
          900: "#8A3F19",
        },
        sky: {
          50: "#F0F9FF",
          100: "#E0F2FE",
          200: "#BAE6FD",
          300: "#7DD3FC",
          400: "#38BDF8",
          500: "#87CEEB",
          600: "#0284C7",
          700: "#0369A1",
          800: "#075985",
          900: "#0C4A6E",
        },
        bark: {
          50: "#FAF6F3",
          100: "#F3EBE3",
          200: "#E7D8C7",
          300: "#D4BEA5",
          400: "#B9997A",
          500: "#8B7355",
          600: "#745E46",
          700: "#5E4A38",
          800: "#4D3D2F",
          900: "#3F3227",
        },
      },
      keyframes: {
        "cozy-bob": {
          "0%, 100%": { transform: "translateY(0)" },
          "50%": { transform: "translateY(-3px)" },
        },
        "cozy-float": {
          "0%, 100%": { transform: "translateY(0) scale(1)" },
          "50%": { transform: "translateY(-6px) scale(1.01)" },
        },
        "cozy-pulse": {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.85" },
        },
      },
      animation: {
        "cozy-bob": "cozy-bob 1.8s ease-in-out infinite",
        "cozy-float": "cozy-float 3.2s ease-in-out infinite",
        "cozy-pulse": "cozy-pulse 1.6s ease-in-out infinite",
      },
    },
  },
  plugins: [animate],
};
export default config;



