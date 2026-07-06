import {
	chunkWindow,
	DEFAULT_ISO_GEOMETRY,
	type IsoGeometry,
	isoContentSize,
} from "@/lib/game/isoProjection";

export const ISO: IsoGeometry = DEFAULT_ISO_GEOMETRY;
export const ISO_CONTENT = isoContentSize(ISO);

export const CHUNK_SIZE = ISO.chunkSize;

/**
 * Renderable chunk window (25x25 chunks, ±12 around the village). Derived from
 * the content-plane geometry so it can never drift from the drawable area.
 */
const CHUNK_WINDOW = chunkWindow(ISO);
export const CHUNK_MIN = CHUNK_WINDOW.min;
export const CHUNK_MAX = CHUNK_WINDOW.max;

export {
	FENCE_X_SPRITE,
	FENCE_Y_SPRITE,
	FOG_SHADES,
	GATE_SPRITE,
	ROAD_DIR,
	ROAD_SPRITES,
	ROAD_WORN_FILTER,
	roadSpriteFor,
	STUMP_SPRITE,
	TILE_SPRITES,
	VILLAGE_RING_RADIUS,
	WATER_SPRITE,
} from "@/lib/game/tileVisuals";

export const BUILDING_SPRITES: Record<string, string> = {
	shrine: "/images/iso/buildings/shrine.png",
	den: "/images/iso/buildings/den.png",
	food_storage: "/images/iso/buildings/food-storage.png",
	water_bowl: "/images/iso/buildings/water-bowl.png",
	beds: "/images/iso/buildings/beds.png",
	herb_garden: "/images/iso/buildings/herb-garden.png",
	nursery: "/images/iso/buildings/nursery.png",
	elder_corner: "/images/iso/buildings/elder-corner.png",
	walls: "/images/iso/buildings/walls.png",
	mouse_farm: "/images/iso/buildings/mouse-farm.png",
	workshop: "/images/iso/buildings/workshop.png",
	field: "/images/iso/buildings/field.png",
	// Reuse curated pieces that read the part: elders at study for the
	// research hut, kittens at their books for the school.
	research_hut: "/images/iso/buildings/elder-corner.png",
	school: "/images/iso/buildings/nursery.png",
};
export const BUILDING_SPRITE_FALLBACK = "/images/iso/buildings/default.png";

/** Flat biome colors — used for the fog-of-war shimmer and loading states. */
export const TILE_COLORS: Record<string, string> = {
	field: "#b5cf8f",
	meadow: "#a9d18a",
	forest: "#6f9e58",
	oak_forest: "#7aa75e",
	pine_forest: "#527a5a",
	dense_woods: "#4c7a44",
	jungle: "#3f8f5f",
	dead_forest: "#8a8270",
	river: "#6aa9d8",
	mountains: "#9aa0a8",
	swamp: "#6e7d5a",
	desert: "#e0c98f",
	tundra: "#dfe7ea",
	cave_entrance: "#7d7468",
	enemy_territory: "#b07a7a",
	enemy_lair: "#9b5a5a",
};

/** CSS clip-path for a 2:1 iso ground diamond. */
export const DIAMOND_CLIP = "polygon(50% 0%, 100% 50%, 50% 100%, 0% 50%)";
