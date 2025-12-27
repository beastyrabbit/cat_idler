#!/usr/bin/env python3
"""
Isometric Asset Generator for Cat Colony Idle Game

Uses OpenRouter API with Google Gemini 2.5 Flash Image model to generate
consistent isometric tile assets for the game.

NOTE: Cat sprites are NOT generated here - they use the dynamic renderer
at https://web.beastyrabbit.com/ (see lib/cat-renderer/api.ts)

Usage:
    python scripts/generate_isometric_assets.py --api-key YOUR_API_KEY
    
    # Or set environment variable:
    export OPENROUTER_API_KEY=your_key
    python scripts/generate_isometric_assets.py

Options:
    --api-key       OpenRouter API key
    --output-dir    Output directory (default: public/images)
    --biome         Generate only specific biome (e.g., oak_forest)
    --biomes        Generate base biome tiles
    --autotile      Generate autotile variants for biomes (16 per biome)
    --buildings     Generate isometric building assets
    --all           Generate all tiles and buildings
    --dry-run       Print prompts without generating images

Examples:
    # Preview all prompts without API calls
    python scripts/generate_isometric_assets.py --dry-run --all
    
    # Generate just oak_forest base tile
    python scripts/generate_isometric_assets.py --biome oak_forest
    
    # Generate all autotile variants
    python scripts/generate_isometric_assets.py --autotile
"""

import argparse
import base64
import json
import os
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

try:
    import requests
except ImportError:
    print("Please install requests: pip install requests")
    sys.exit(1)

try:
    from PIL import Image
    import io
    HAS_PIL = True
except ImportError:
    HAS_PIL = False
    print("Warning: Pillow not installed. Background removal disabled.")
    print("Install with: pip install Pillow")


# =============================================================================
# Configuration
# =============================================================================

OPENROUTER_API_URL = "https://openrouter.ai/api/v1/chat/completions"
OPENROUTER_API_KEY = "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"
MODEL = "google/gemini-2.5-flash-image-preview"  # Image generation model (native image output)

# Tile dimensions
TILE_WIDTH = 128
TILE_HEIGHT = 64

# Retry configuration
MAX_RETRIES = 3
RETRY_DELAY = 2  # seconds

# =============================================================================
# Style Constants for Consistent Prompts
# Following Google's hyper-specific prompting guidelines for Gemini 2.5 Flash Image
# Structure: (A) Asset role + camera, (B) Subject, (C) Style, (D) Constraints, (E) Output
# Uses semantic positives instead of negatives (e.g., "clean background" not "no background")
# =============================================================================

# Style bible - consistent across all assets (Stardew Valley / Cozy Grove aesthetic)
STYLE_BIBLE = "hand-painted, soft gradients, warm colors, slightly stylized, cozy game aesthetic like Stardew Valley, clean outlines, readable silhouettes"

# Lighting description - consistent across all assets
LIGHTING = "soft studio light from top-left, subtle drop shadow underneath"

# Background color for chroma keying (will be removed in post-processing)
# Using bright magenta (#FF00FF) as it's unlikely to appear in natural game assets
CHROMA_KEY_BG = "solid bright magenta (#FF00FF) background"

# Constraint template for tiles
TILE_CONSTRAINTS = f"single tile only, centered, {CHROMA_KEY_BG}, crisp edges, consistent density"

# Constraint template for buildings  
BUILDING_CONSTRAINTS = f"single building only, centered, {CHROMA_KEY_BG}, crisp edges, readable silhouette"


# =============================================================================
# Post-Processing: Chroma Key Background Removal
# =============================================================================

def remove_chroma_key_background(image_bytes: bytes, tolerance: int = 70, sample_size: int = 10) -> bytes:
    """
    Remove background by sampling corner regions and flood filling.
    
    1. Sample 10x10 pixel regions from each corner
    2. Calculate average color from these samples
    3. Flood fill from edges, removing pixels similar to the sampled background
    
    Args:
        image_bytes: Raw image bytes from API
        tolerance: Color matching tolerance for flood fill (0-255)
        sample_size: Size of corner sample region (e.g., 10 = 10x10 pixels)
    
    Returns:
        PNG image bytes with transparent background
    """
    if not HAS_PIL:
        return image_bytes
    
    from collections import deque
    
    # Load image
    img = Image.open(io.BytesIO(image_bytes))
    
    # Convert to RGBA if needed
    if img.mode != 'RGBA':
        img = img.convert('RGBA')
    
    pixels = img.load()
    width, height = img.size
    
    def color_distance(c1, c2):
        """Calculate Euclidean color distance between two RGB tuples."""
        return ((c1[0] - c2[0])**2 + (c1[1] - c2[1])**2 + (c1[2] - c2[2])**2) ** 0.5
    
    def sample_corner_region(start_x, start_y, size):
        """Sample a region and return list of RGB colors."""
        colors = []
        for dy in range(size):
            for dx in range(size):
                x = start_x + dx
                y = start_y + dy
                if 0 <= x < width and 0 <= y < height:
                    r, g, b, a = pixels[x, y]
                    colors.append((r, g, b))
        return colors
    
    # Sample 10x10 regions from all 4 corners
    corner_regions = [
        (0, 0),                                    # Top-left
        (width - sample_size, 0),                  # Top-right
        (0, height - sample_size),                 # Bottom-left
        (width - sample_size, height - sample_size)  # Bottom-right
    ]
    
    all_bg_colors = []
    for cx, cy in corner_regions:
        all_bg_colors.extend(sample_corner_region(cx, cy, sample_size))
    
    if not all_bg_colors:
        print("  (No corner samples found)")
        return image_bytes
    
    # Calculate average background color from all corner samples
    avg_r = sum(c[0] for c in all_bg_colors) // len(all_bg_colors)
    avg_g = sum(c[1] for c in all_bg_colors) // len(all_bg_colors)
    avg_b = sum(c[2] for c in all_bg_colors) // len(all_bg_colors)
    bg_color = (avg_r, avg_g, avg_b)
    
    print(f"  (Detected background color: RGB({avg_r}, {avg_g}, {avg_b}))")
    
    # Flood fill from all edges
    visited = set()
    to_remove = set()
    queue = deque()
    
    # Start from all edge pixels
    for x in range(width):
        queue.append((x, 0))
        queue.append((x, height - 1))
    for y in range(height):
        queue.append((0, y))
        queue.append((width - 1, y))
    
    while queue:
        x, y = queue.popleft()
        
        if (x, y) in visited:
            continue
        if x < 0 or x >= width or y < 0 or y >= height:
            continue
            
        visited.add((x, y))
        r, g, b, a = pixels[x, y]
        
        # Check if this pixel is close to the background color
        dist = color_distance((r, g, b), bg_color)
        if dist <= tolerance:
            to_remove.add((x, y))
            # Add 4-connected neighbors
            queue.append((x + 1, y))
            queue.append((x - 1, y))
            queue.append((x, y + 1))
            queue.append((x, y - 1))
    
    # Remove background pixels (make transparent)
    for x, y in to_remove:
        pixels[x, y] = (0, 0, 0, 0)
    
    print(f"  (Removed {len(to_remove)} background pixels)")
    
    # Save to bytes
    output = io.BytesIO()
    img.save(output, format='PNG')
    return output.getvalue()


# =============================================================================
# Biome Definitions with Detailed Prompts
# =============================================================================

@dataclass
class BiomeConfig:
    """Configuration for a biome type with detailed prompt information."""
    name: str
    display_name: str
    base_prompt: str
    color_palette: str
    features: str
    mood: str


BIOMES: dict[str, BiomeConfig] = {
    "oak_forest": BiomeConfig(
        name="oak_forest",
        display_name="Oak Forest",
        base_prompt="A lush oak forest floor tile with rich brown soil, fallen acorns, scattered autumn leaves in orange and gold, and patches of soft green moss",
        color_palette="warm browns, forest greens, touches of orange and gold, earthy tones",
        features="oak leaf litter, acorns, moss patches, small mushrooms, twigs",
        mood="peaceful, dappled sunlight filtering through trees, cozy autumn forest"
    ),
    "pine_forest": BiomeConfig(
        name="pine_forest",
        display_name="Pine Forest",
        base_prompt="A dark evergreen pine forest floor with scattered pine needles, pine cones, and patches of low ferns",
        color_palette="deep greens, dark browns, hints of blue-green, shadowy tones",
        features="pine needles carpet, pine cones, small ferns, rocks with lichen",
        mood="mysterious, slightly darker, peaceful coniferous forest"
    ),
    "jungle": BiomeConfig(
        name="jungle",
        display_name="Jungle",
        base_prompt="A dense tropical jungle floor with large tropical leaves, exotic flowers, vines, and humid atmosphere",
        color_palette="vibrant greens, tropical pinks and purples, yellow accents, lush colors",
        features="large monstera-like leaves, colorful flowers, hanging vines, tropical plants",
        mood="lush, exotic, humid, teeming with life"
    ),
    "dead_forest": BiomeConfig(
        name="dead_forest",
        display_name="Dead Forest",
        base_prompt="A spooky dead forest floor with bare twisted roots, dead leaves, and eerie fog wisps",
        color_palette="grays, muted browns, purple-gray shadows, desaturated tones",
        features="dead leaves, bare roots, cracked dry soil, wisps of fog, dead branches",
        mood="eerie, haunting, but still somewhat cozy in a gothic way"
    ),
    "mountains": BiomeConfig(
        name="mountains",
        display_name="Mountains",
        base_prompt="A rocky mountain terrain tile with gray stone, alpine gravel, small hardy plants between rocks",
        color_palette="grays, slate blues, hints of green, cool stone tones",
        features="rocky ground, gravel, small alpine flowers, lichen on stones",
        mood="rugged, high altitude, crisp mountain air feeling"
    ),
    "swamp": BiomeConfig(
        name="swamp",
        display_name="Swamp",
        base_prompt="A murky swamp tile with shallow muddy water, lily pads, cattails, and mossy patches",
        color_palette="murky greens, muddy browns, dark water blues, moss colors",
        features="shallow water, lily pads, cattails, moss, algae, muddy patches",
        mood="mysterious, damp, but with a cozy wetland charm"
    ),
    "desert": BiomeConfig(
        name="desert",
        display_name="Desert",
        base_prompt="A sandy desert tile with golden dunes, scattered pebbles, and occasional desert succulent",
        color_palette="warm golds, sandy yellows, terracotta, sun-bleached tones",
        features="sand ripples, small rocks, tiny cactus or succulent, desert flowers",
        mood="warm, sun-baked, peaceful desert oasis feeling"
    ),
    "tundra": BiomeConfig(
        name="tundra",
        display_name="Tundra",
        base_prompt="A frozen tundra tile with snow patches, ice crystals, frozen ground, and hardy arctic moss",
        color_palette="whites, icy blues, pale greens, frost tones",
        features="snow patches, ice crystals, frozen ground, arctic moss, small stones",
        mood="cold but beautiful, arctic serenity, winter wonderland"
    ),
    "meadow": BiomeConfig(
        name="meadow",
        display_name="Meadow",
        base_prompt="A sunny meadow tile with lush green grass, wildflowers in various colors, and butterflies",
        color_palette="bright greens, cheerful yellows, pinks, purples, sunny colors",
        features="tall grass, daisies, buttercups, lavender, clover, small butterflies",
        mood="cheerful, sunny, peaceful pastoral scene"
    ),
    "cave_entrance": BiomeConfig(
        name="cave_entrance",
        display_name="Cave Entrance",
        base_prompt="A dark cave entrance tile with rocky ground, mysterious shadows, and glowing crystals",
        color_palette="dark grays, deep purples, hints of crystal blue/purple glow",
        features="cave rocks, mysterious shadows, small glowing crystals, stalactite hints",
        mood="mysterious, adventurous, hint of treasure within"
    ),
    "enemy_lair": BiomeConfig(
        name="enemy_lair",
        display_name="Enemy Lair",
        base_prompt="A threatening enemy territory tile with scorched earth, bones, claw marks, and ominous atmosphere",
        color_palette="dark reds, charred blacks, bone whites, threatening tones",
        features="scorched ground, small bones, claw marks in dirt, dead vegetation",
        mood="dangerous, threatening, but still game-appropriate (not too scary)"
    ),
}


# =============================================================================
# Autotile Variant Descriptions
# =============================================================================

# 4-bit neighbor system: N=1, E=2, S=4, W=8
# This gives us 16 variants (0-15) based on which neighbors match

AUTOTILE_VARIANTS = {
    0: ("isolated", "completely surrounded by different terrain, island tile"),
    1: ("north_only", "connects to same terrain on north edge only"),
    2: ("east_only", "connects to same terrain on east edge only"),
    3: ("north_east", "connects to same terrain on north and east edges (corner piece)"),
    4: ("south_only", "connects to same terrain on south edge only"),
    5: ("north_south", "connects to same terrain on north and south (vertical strip)"),
    6: ("east_south", "connects to same terrain on east and south edges (corner piece)"),
    7: ("north_east_south", "connects on north, east, and south (open to west)"),
    8: ("west_only", "connects to same terrain on west edge only"),
    9: ("north_west", "connects to same terrain on north and west edges (corner piece)"),
    10: ("east_west", "connects to same terrain on east and west (horizontal strip)"),
    11: ("north_east_west", "connects on north, east, and west (open to south)"),
    12: ("south_west", "connects to same terrain on south and west edges (corner piece)"),
    13: ("north_south_west", "connects on north, south, and west (open to east)"),
    14: ("east_south_west", "connects on east, south, and west (open to north)"),
    15: ("full", "fully surrounded by same terrain on all sides, center tile"),
}


# =============================================================================
# Building Definitions
# =============================================================================

BUILDINGS = {
    "den": {
        "size": (192, 128),  # Larger building
        "prompt": "A cozy cat den/shelter made of woven branches and leaves, with a small entrance, comfortable-looking interior visible, warm and inviting",
        "features": "woven branch structure, leaf roof, soft bedding visible inside, small paw prints near entrance"
    },
    "food_storage": {
        "size": (128, 96),
        "prompt": "A rustic food storage hut with hanging dried fish and meat, wooden shelves with preserved foods",
        "features": "wooden structure, hanging fish, clay pots, wooden shelves"
    },
    "water_bowl": {
        "size": (96, 64),
        "prompt": "A large stone water bowl with fresh clear water, lily pad floating, small ripples",
        "features": "carved stone bowl, clear water, small plants around base"
    },
    "beds": {
        "size": (128, 96),
        "prompt": "Cozy sleeping area with multiple soft beds made of moss, leaves, and feathers for cats",
        "features": "multiple soft nests, feathers, dried grass bedding, cozy arrangement"
    },
    "herb_garden": {
        "size": (128, 96),
        "prompt": "A small medicinal herb garden with various healing plants, lavender, mint, and catnip",
        "features": "organized plant beds, labeled herbs, small fence, aromatic plants"
    },
    "nursery": {
        "size": (160, 112),
        "prompt": "A protected nursery area for kittens with extra soft bedding, toys, and protective walls",
        "features": "extra soft bedding, small toys, protective woven walls, warm lighting"
    },
    "elder_corner": {
        "size": (128, 96),
        "prompt": "A comfortable resting area for elder cats with extra cushioning and sun-catching design",
        "features": "thick cushioned beds, sun patch, gentle slope for easy access"
    },
    "walls": {
        "size": (128, 64),
        "prompt": "A defensive wall section made of woven branches and thorns, sturdy but natural-looking",
        "features": "woven branches, thorny vines, sturdy construction"
    },
    "mouse_farm": {
        "size": (160, 112),
        "prompt": "A small enclosed area for farming mice, with little mouse houses and feeding stations",
        "features": "small enclosure, tiny mouse houses, grain storage, humane design"
    },
}


# =============================================================================
# NOTE: Cat sprites are NOT generated here!
# Cat sprites use the dynamic renderer at https://web.beastyrabbit.com/
# See lib/cat-renderer/api.ts for the integration
# =============================================================================


# =============================================================================
# API Client
# =============================================================================

class OpenRouterClient:
    """Client for OpenRouter API image generation."""
    
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.session = requests.Session()
        self.session.headers.update({
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "HTTP-Referer": "https://github.com/cat-colony-idle",
            "X-Title": "Cat Colony Idle Game Asset Generator"
        })
        # Cost tracking
        self.total_cost = 0.0
        self.total_requests = 0
        self.successful_requests = 0
    
    def get_cost_summary(self) -> dict:
        """Get summary of API usage and costs."""
        return {
            "total_requests": self.total_requests,
            "successful_requests": self.successful_requests,
            "failed_requests": self.total_requests - self.successful_requests,
            "total_cost_usd": self.total_cost,
        }
    
    def generate_image(
        self, 
        prompt: str, 
        aspect_ratio: str = "16:9",
        retries: int = MAX_RETRIES
    ) -> Optional[bytes]:
        """
        Generate an image using the Gemini 2.5 Flash model.
        
        Args:
            prompt: The image generation prompt
            aspect_ratio: Aspect ratio for the image. Supported: 1:1, 2:3, 3:2, 3:4, 4:3, 4:5, 5:4, 9:16, 16:9, 21:9
            retries: Number of retry attempts
        
        Returns the image bytes or None if generation fails.
        """
        payload = {
            "model": MODEL,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            # Required for image generation
            "modalities": ["image", "text"],
            # Gemini-specific image configuration
            "image_config": {
                "aspect_ratio": aspect_ratio,
                "image_size": "1K"
            }
        }
        
        for attempt in range(retries):
            try:
                self.total_requests += 1
                response = self.session.post(
                    OPENROUTER_API_URL,
                    json=payload,
                    timeout=120  # Image generation can take a while
                )
                
                if response.status_code == 200:
                    data = response.json()
                    
                    # Track cost from response (OpenRouter includes usage data)
                    if "usage" in data:
                        usage = data["usage"]
                        # OpenRouter may include total_cost directly
                        if "total_cost" in usage:
                            self.total_cost += float(usage["total_cost"])
                        # Or calculate from native_tokens_total and cost headers
                        elif "cost" in usage:
                            self.total_cost += float(usage["cost"])
                    
                    # Handle OpenRouter Gemini image generation response format
                    # Images are in: choices[0].message.images[0].image_url.url
                    if "choices" in data and len(data["choices"]) > 0:
                        message = data["choices"][0].get("message", {})
                        
                        # Primary format: images array with image_url objects
                        images = message.get("images", [])
                        if images:
                            # Get the first image
                            image_obj = images[0]
                            # Handle both possible structures
                            image_url = (
                                image_obj.get("image_url", {}).get("url") or 
                                image_obj.get("imageUrl", {}).get("url") or
                                image_obj.get("url")
                            )
                            
                            if image_url:
                                # Check if it's a base64 data URL
                                if image_url.startswith("data:image"):
                                    base64_data = image_url.split(",")[1] if "," in image_url else image_url
                                    self.successful_requests += 1
                                    return base64.b64decode(base64_data)
                                # Or a regular URL to download
                                elif image_url.startswith("http"):
                                    img_response = requests.get(image_url, timeout=60)
                                    if img_response.status_code == 200:
                                        self.successful_requests += 1
                                        return img_response.content
                        
                        # Fallback: check if content itself is base64 image data
                        content = message.get("content", "")
                        if content and content.startswith("data:image"):
                            base64_data = content.split(",")[1] if "," in content else content
                            self.successful_requests += 1
                            return base64.b64decode(base64_data)
                    
                    print(f"  Unexpected response format: {json.dumps(data, indent=2)[:500]}")
                    
                elif response.status_code == 429:
                    # Rate limited
                    wait_time = RETRY_DELAY * (attempt + 1) * 2
                    print(f"  Rate limited. Waiting {wait_time}s...")
                    time.sleep(wait_time)
                    continue
                    
                else:
                    print(f"  API error {response.status_code}: {response.text[:200]}")
                    
            except requests.exceptions.Timeout:
                print(f"  Request timed out (attempt {attempt + 1}/{retries})")
            except Exception as e:
                print(f"  Error: {e}")
            
            if attempt < retries - 1:
                time.sleep(RETRY_DELAY)
        
        return None


# =============================================================================
# Asset Generation Functions
# =============================================================================

def generate_biome_tile(
    client: OpenRouterClient,
    biome: BiomeConfig,
    output_dir: Path,
    variant: Optional[int] = None,
    dry_run: bool = False
) -> bool:
    """Generate a single biome tile image."""
    
    # Build hyper-specific prompt following (A)(B)(C)(D)(E) structure
    # Reference: Google's Gemini 2.5 Flash Image prompting guide
    
    if variant is not None:
        variant_name, variant_desc = AUTOTILE_VARIANTS[variant]
        # Autotile variant with edge blending info
        prompt = f"""Generate an isometric 2D game terrain tile, orthographic 3/4 top-down view, diamond shape. Subject: {biome.base_prompt}. Tile variant {variant} ({variant_name}): {variant_desc}. Visual elements: {biome.features}. Color palette: {biome.color_palette}. Mood: {biome.mood}. Style: {STYLE_BIBLE}. Constraints: {TILE_CONSTRAINTS}, seamless edges where connecting to same terrain. Lighting: {LIGHTING}. Generate 1 image."""
        filename = f"{variant}.png"
        subdir = output_dir / biome.name
    else:
        # Center tile (variant 15) - seamless on all sides
        prompt = f"""Generate an isometric 2D game terrain tile, orthographic 3/4 top-down view, diamond shape. Subject: {biome.base_prompt}. This is a center tile that seamlessly connects on all edges. Visual elements: {biome.features}. Color palette: {biome.color_palette}. Mood: {biome.mood}. Style: {STYLE_BIBLE}. Constraints: {TILE_CONSTRAINTS}, perfectly seamless on all edges. Lighting: {LIGHTING}. Generate 1 image."""
        filename = f"{biome.name}.png"
        subdir = output_dir
    
    # Ensure output directory exists
    subdir.mkdir(parents=True, exist_ok=True)
    output_path = subdir / filename
    
    if dry_run:
        print(f"\n{'='*60}")
        print(f"PROMPT for {biome.name}" + (f" variant {variant}" if variant is not None else ""))
        print(f"{'='*60}")
        print(prompt)
        return True
    
    print(f"Generating {biome.display_name}" + (f" variant {variant}" if variant is not None else "") + "...")
    
    # Use 16:9 aspect ratio (closest to 2:1 for isometric tiles)
    # Supported ratios: 1:1, 2:3, 3:2, 3:4, 4:3, 4:5, 5:4, 9:16, 16:9, 21:9
    image_data = client.generate_image(prompt, aspect_ratio="16:9")
    
    if image_data:
        # Post-process: remove magenta background, add true transparency
        if HAS_PIL:
            image_data = remove_chroma_key_background(image_data)
            print(f"  ✓ Background removed")
        output_path.write_bytes(image_data)
        print(f"  ✓ Saved to {output_path}")
        return True
    else:
        print(f"  ✗ Failed to generate")
        return False


def generate_building(
    client: OpenRouterClient,
    building_name: str,
    config: dict,
    output_dir: Path,
    dry_run: bool = False
) -> bool:
    """Generate a building asset."""
    
    width, height = config["size"]
    building_display_name = building_name.replace('_', ' ')
    
    # Build hyper-specific prompt following (A)(B)(C)(D)(E) structure
    # Reference: Google's Gemini 2.5 Flash Image prompting guide
    prompt = f"""Generate an isometric 2D game building, orthographic 3/4 top-down view, for a cat colony game. Subject: {config["prompt"]}. Visual details: {config["features"]}. Scale: cat-sized structure, cozy proportions. Style: {STYLE_BIBLE}, natural materials like wood, stone, leaves, branches. Constraints: {BUILDING_CONSTRAINTS}, warm inviting safe-feeling design. Lighting: {LIGHTING}. Generate 1 image."""
    
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / f"{building_name}.png"
    
    if dry_run:
        print(f"\n{'='*60}")
        print(f"PROMPT for building: {building_name}")
        print(f"{'='*60}")
        print(prompt)
        return True
    
    print(f"Generating building: {building_name}...")
    
    # Calculate aspect ratio from building size for better proportions
    width, height = config["size"]
    # Map to closest supported aspect ratio
    ratio = width / height
    if ratio >= 1.5:
        aspect_ratio = "3:2"  # Wide buildings
    elif ratio >= 1.2:
        aspect_ratio = "4:3"  # Slightly wide
    else:
        aspect_ratio = "1:1"  # Square-ish
    
    image_data = client.generate_image(prompt, aspect_ratio=aspect_ratio)
    
    if image_data:
        # Post-process: remove magenta background, add true transparency
        if HAS_PIL:
            image_data = remove_chroma_key_background(image_data)
            print(f"  ✓ Background removed")
        output_path.write_bytes(image_data)
        print(f"  ✓ Saved to {output_path}")
        return True
    else:
        print(f"  ✗ Failed to generate")
        return False


# =============================================================================
# Main Entry Point
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Generate isometric game assets using AI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    
    parser.add_argument(
        "--api-key",
        default=os.environ.get("OPENROUTER_API_KEY", OPENROUTER_API_KEY),
        help="OpenRouter API key (or set OPENROUTER_API_KEY env var)"
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("public/images"),
        help="Base output directory"
    )
    parser.add_argument(
        "--biome",
        choices=list(BIOMES.keys()),
        help="Generate only specific biome"
    )
    parser.add_argument(
        "--biomes",
        action="store_true",
        help="Generate base biome tiles (no autotile variants)"
    )
    parser.add_argument(
        "--autotile",
        action="store_true",
        help="Generate autotile variants for all biomes"
    )
    parser.add_argument(
        "--buildings",
        action="store_true",
        help="Generate isometric building assets"
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="Generate all assets"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print prompts without generating images"
    )
    
    args = parser.parse_args()
    
    # Validate API key
    if not args.api_key and not args.dry_run:
        print("Error: OpenRouter API key required")
        print("Set OPENROUTER_API_KEY environment variable or use --api-key")
        sys.exit(1)
    
    # Default to generating biomes if nothing specified
    if not any([args.biomes, args.autotile, args.buildings, args.all, args.biome]):
        print("No generation target specified. Use --biomes, --autotile, --buildings, or --all")
        print("Note: Cat sprites use the beastyrabbit.com renderer, not AI generation.")
        parser.print_help()
        sys.exit(1)
    
    # Initialize client
    client = OpenRouterClient(args.api_key) if not args.dry_run else None
    
    success_count = 0
    fail_count = 0
    
    # Generate biome tiles
    if args.biomes or args.all or args.biome:
        print("\n" + "="*60)
        print("GENERATING BIOME TILES")
        print("="*60)
        
        biomes_to_generate = [args.biome] if args.biome else list(BIOMES.keys())
        
        for biome_name in biomes_to_generate:
            biome = BIOMES[biome_name]
            if generate_biome_tile(
                client, biome,
                args.output_dir / "tiles" / "isometric",
                dry_run=args.dry_run
            ):
                success_count += 1
            else:
                fail_count += 1
            
            # Rate limiting delay
            if not args.dry_run:
                time.sleep(1)
    
    # Generate autotile variants
    if args.autotile or args.all:
        print("\n" + "="*60)
        print("GENERATING AUTOTILE VARIANTS")
        print("="*60)
        
        biomes_to_generate = [args.biome] if args.biome else list(BIOMES.keys())
        
        for biome_name in biomes_to_generate:
            biome = BIOMES[biome_name]
            print(f"\nBiome: {biome.display_name}")
            
            for variant in range(16):
                if generate_biome_tile(
                    client, biome,
                    args.output_dir / "tiles" / "isometric",
                    variant=variant,
                    dry_run=args.dry_run
                ):
                    success_count += 1
                else:
                    fail_count += 1
                
                # Rate limiting delay
                if not args.dry_run:
                    time.sleep(1)
    
    # Generate buildings
    if args.buildings or args.all:
        print("\n" + "="*60)
        print("GENERATING BUILDINGS")
        print("="*60)
        
        for name, config in BUILDINGS.items():
            if generate_building(
                client, name, config,
                args.output_dir / "buildings" / "isometric",
                dry_run=args.dry_run
            ):
                success_count += 1
            else:
                fail_count += 1
            
            if not args.dry_run:
                time.sleep(1)
    
    # Note: Cat sprites are rendered dynamically via beastyrabbit.com API
    # See lib/cat-renderer/api.ts - no static cat assets needed
    
    # Summary
    print("\n" + "="*60)
    print("GENERATION COMPLETE")
    print("="*60)
    print(f"Successful: {success_count}")
    print(f"Failed: {fail_count}")
    
    # Cost summary
    if client:
        cost_summary = client.get_cost_summary()
        print("\n" + "-"*40)
        print("API USAGE & COST")
        print("-"*40)
        print(f"Total API requests: {cost_summary['total_requests']}")
        print(f"Successful: {cost_summary['successful_requests']}")
        print(f"Failed: {cost_summary['failed_requests']}")
        if cost_summary['total_cost_usd'] > 0:
            print(f"Total cost: ${cost_summary['total_cost_usd']:.4f} USD")
        else:
            print("Total cost: (not available in response)")
    
    if fail_count > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()

