import { describe, expect, it } from "vitest";
import {
	ROAD_DIR,
	ROAD_SPRITES,
	roadSpriteFor,
} from "@/components/map/constants";

const { E, W, N, S } = ROAD_DIR;

describe("roadSpriteFor", () => {
	it("picks a straight along the x-axis for an E/W run", () => {
		expect(roadSpriteFor(E | W)).toBe(ROAD_SPRITES.straightX);
	});

	it("picks a straight along the y-axis for an N/S run", () => {
		expect(roadSpriteFor(N | S)).toBe(ROAD_SPRITES.straightY);
	});

	it("reads a lone neighbour as a dead-end oriented toward it", () => {
		expect(roadSpriteFor(E)).toBe(ROAD_SPRITES.endE);
		expect(roadSpriteFor(W)).toBe(ROAD_SPRITES.endW);
		expect(roadSpriteFor(N)).toBe(ROAD_SPRITES.endN);
		expect(roadSpriteFor(S)).toBe(ROAD_SPRITES.endS);
	});

	it("renders an isolated road tile as a clearing stub", () => {
		expect(roadSpriteFor(0)).toBe(ROAD_SPRITES.clearing);
	});

	it("picks the matching L-corner for each perpendicular pair", () => {
		expect(roadSpriteFor(E | N)).toBe(ROAD_SPRITES.cornerEN);
		expect(roadSpriteFor(E | S)).toBe(ROAD_SPRITES.cornerES);
		expect(roadSpriteFor(W | N)).toBe(ROAD_SPRITES.cornerWN);
		expect(roadSpriteFor(W | S)).toBe(ROAD_SPRITES.cornerWS);
	});

	it("picks a crossing for a 3- or 4-way junction", () => {
		expect(roadSpriteFor(E | W | N)).toBe(ROAD_SPRITES.crossing);
		expect(roadSpriteFor(E | W | N | S)).toBe(ROAD_SPRITES.crossing);
	});
});
