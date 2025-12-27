"use client";

import { useState, useEffect, useMemo } from "react";
import type { Cat, CatSpriteProps } from "@/types/game";
import { renderCat } from "@/lib/cat-renderer/api";
import type { CatSpriteParams } from "@/lib/cat-renderer/types";

/**
 * Cat Sprite Component
 * 
 * Displays cat sprites using the external renderer from beastyrabbit.com
 * Falls back to placeholder if renderer fails or no sprite params.
 */
export function CatSprite({ cat, size = "medium", animated = true, onClick }: CatSpriteProps) {
  const [spriteUrl, setSpriteUrl] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Memoize sprite params to avoid unnecessary re-renders
  const spriteParams = useMemo(() => {
    return cat.spriteParams as CatSpriteParams | null;
  }, [cat.spriteParams]);

  useEffect(() => {
    let cancelled = false;

    async function loadSprite() {
      if (!spriteParams) {
        setIsLoading(false);
        return;
      }

      setIsLoading(true);
      setError(null);

      try {
        const imageDataUrl = await renderCat(spriteParams);
        if (!cancelled) {
          setSpriteUrl(imageDataUrl);
          setIsLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          console.warn("Failed to render cat sprite:", err);
          setError("Render failed");
          setIsLoading(false);
        }
      }
    }

    loadSprite();

    return () => {
      cancelled = true;
    };
  }, [spriteParams]);

  const sizeClasses = {
    small: "w-8 h-8",
    medium: "w-16 h-16",
    large: "w-32 h-32",
  };

  const animationClass = animated ? "animate-cozy-bob" : "";

  // Fallback: determine local asset variant
  const getVariant = (name: string) => {
    const variants = ["orange-tabby", "gray-tabby", "black", "white", "calico", "tuxedo"];
    const hash = name.split("").reduce((acc, char) => acc + char.charCodeAt(0), 0);
    return variants[hash % variants.length];
  };

  const variant = getVariant(cat.name);
  const localImageSrc = `/images/cats/${variant}/idle.png`;

  // Show rendered sprite if available
  if (spriteUrl && !error) {
    return (
      <img
        src={spriteUrl}
        alt={cat.name}
        className={`${sizeClasses[size]} ${animationClass} cursor-pointer drop-shadow-[0_10px_16px_rgba(43,33,23,0.18)] object-contain`}
        onClick={onClick}
        data-testid="cat-sprite"
        style={{ imageRendering: "pixelated" }}
      />
    );
  }

  // Show loading state
  if (isLoading && spriteParams) {
    return (
      <div
        className={`${sizeClasses[size]} ${animationClass} relative cursor-pointer drop-shadow-[0_10px_16px_rgba(43,33,23,0.18)] bg-gray-200 rounded flex items-center justify-center`}
        onClick={onClick}
        data-testid="cat-sprite-loading"
      >
        <div className="w-4 h-4 border-2 border-gray-400 border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  // Fallback to local asset or placeholder
  return (
    <div
      className={`${sizeClasses[size]} ${animationClass} relative cursor-pointer drop-shadow-[0_10px_16px_rgba(43,33,23,0.18)]`}
      onClick={onClick}
      data-testid="cat-sprite-local"
    >
      <img 
        src={localImageSrc} 
        alt={`${cat.name} (${variant})`}
        className="w-full h-full object-contain"
        onError={(e) => {
          // If image fails to load, show placeholder
          const target = e.target as HTMLImageElement;
          target.style.display = "none";
          const parent = target.parentElement;
          if (parent) {
            parent.className += " bg-gray-300";
            parent.textContent = cat.name[0].toUpperCase();
          }
        }}
      />
    </div>
  );
}



