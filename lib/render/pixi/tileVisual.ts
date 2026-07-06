import { ISO } from "@/components/map/constants";

export {
	buildOrganicVillageView,
	computeFogDim,
	computeRoadSprites,
	fenceSprites,
	isExplored,
	type OrganicVillageView,
	tileGround,
} from "@/lib/game/tileVisuals";

/** Sprite draw box: full 256x512 canvas at (left, top - surfaceOffset). */
export const SPRITE_W = ISO.tileWidth;
export const SPRITE_H = ISO.imageHeight;
export const SPRITE_TOP_OFFSET = ISO.surfaceOffset;
