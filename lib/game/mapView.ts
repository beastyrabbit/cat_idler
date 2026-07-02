/**
 * Map View Math
 *
 * Pure viewport math for the map UI: which tiles and chunks are visible
 * for a given pan/zoom state. Extracted from the map components so the
 * culling logic is unit-testable.
 */

export interface ViewState {
	/** Content translate X in px (MapViewport transform). */
	tx: number;
	/** Content translate Y in px. */
	ty: number;
	scale: number;
	/** Viewport size in px. */
	width: number;
	height: number;
}

export interface MapGeometry {
	/** Tile size in content px at scale 1. */
	tileSize: number;
	/** Tiles per chunk edge. */
	chunkSize: number;
	/** Tile coordinate rendered at content (0, 0). */
	originX: number;
	originY: number;
}

export interface TileRect {
	minX: number;
	maxX: number;
	minY: number;
	maxY: number;
}

export interface ChunkCoord {
	chunkX: number;
	chunkY: number;
}

const TILE_MARGIN = 1;

/** Tile-coordinate rectangle visible in the viewport, with a 1-tile margin. */
export function visibleTileRect(view: ViewState, geom: MapGeometry): TileRect {
	const cx0 = (0 - view.tx) / view.scale;
	const cx1 = (view.width - view.tx) / view.scale;
	const cy0 = (0 - view.ty) / view.scale;
	const cy1 = (view.height - view.ty) / view.scale;

	return {
		minX: Math.floor(cx0 / geom.tileSize) + geom.originX - TILE_MARGIN,
		maxX: Math.floor(cx1 / geom.tileSize) + geom.originX + TILE_MARGIN,
		minY: Math.floor(cy0 / geom.tileSize) + geom.originY - TILE_MARGIN,
		maxY: Math.floor(cy1 / geom.tileSize) + geom.originY + TILE_MARGIN,
	};
}

/** Chunks covering the visible tile rectangle. */
export function getVisibleChunks(
	view: ViewState,
	geom: MapGeometry,
): ChunkCoord[] {
	const rect = visibleTileRect(view, geom);

	const minChunkX = Math.floor(rect.minX / geom.chunkSize);
	const maxChunkX = Math.floor(rect.maxX / geom.chunkSize);
	const minChunkY = Math.floor(rect.minY / geom.chunkSize);
	const maxChunkY = Math.floor(rect.maxY / geom.chunkSize);

	const chunks: ChunkCoord[] = [];
	for (let chunkY = minChunkY; chunkY <= maxChunkY; chunkY++) {
		for (let chunkX = minChunkX; chunkX <= maxChunkX; chunkX++) {
			chunks.push({ chunkX, chunkY });
		}
	}
	return chunks;
}

export function chunkKey(chunk: ChunkCoord): string {
	return `${chunk.chunkX},${chunk.chunkY}`;
}
