/**
 * Threat director (pure rules) — Roadmap 4, Military.
 *
 * Raid pressure builds as the colony gets richer, larger, better-defended and
 * older, then discharges as a raid when it crosses a threshold. Young colonies
 * get a grace window so a fresh run isn't sacked in its first minutes. Enemy
 * warbands scale to what they're attacking — wealth and the standing warrior
 * count both make the next raid bigger — so success invites a stiffer test.
 *
 * All randomness is injected (a `roll` in [0,1)) so the caller drives it from
 * the seeded chain and raids stay deterministic under setTestRngSeed.
 */

export interface ThreatSnapshot {
	/** Total stored value the colony presents as loot (see {@link colonyWealth}). */
	wealth: number;
	/** Living cats. */
	population: number;
	/** Trained warriors currently standing. */
	warriors: number;
	/** Game-seconds since the run started (accelerated clock). */
	colonyAgeSec: number;
}

/** Grace window: no pressure builds for this many game-seconds into a run. */
export const RAID_GRACE_SEC = 6 * 3600;
/** Pressure at which a raid launches. */
export const RAID_SPAWN_THRESHOLD = 100;
/** Largest warband the director will field at once. */
export const MAX_RAID_SIZE = 12;
/** Base strength of a single raider before scaling. */
export const RAIDER_BASE_STRENGTH = 30;

/** Threat-indicator band for the HUD. */
export type ThreatBand = "calm" | "rising" | "imminent";

/**
 * Loot value a colony presents — the sum the threat curve reads as "wealth".
 * Refined goods and forged gear are worth more than raw stores.
 */
export function colonyWealth(resources: {
	food?: number;
	water?: number;
	herbs?: number;
	materials?: number;
	refined?: number;
	weapons?: number;
	armor?: number;
}): number {
	return (
		(resources.food ?? 0) +
		(resources.water ?? 0) +
		(resources.herbs ?? 0) +
		(resources.materials ?? 0) +
		(resources.refined ?? 0) * 3 +
		(resources.weapons ?? 0) * 5 +
		(resources.armor ?? 0) * 5
	);
}

/**
 * Pressure gained per game-hour. Zero during the grace window; otherwise a
 * baseline plus contributions from wealth (sub-linear so a hoard doesn't
 * explode), population, the standing warrior count (enemies scale to your
 * defenses), and colony age (a slow, capped creep).
 */
export function threatRatePerHour(s: ThreatSnapshot): number {
	if (s.colonyAgeSec < RAID_GRACE_SEC) {
		return 0;
	}
	const wealthTerm = Math.sqrt(Math.max(0, s.wealth)) * 0.12;
	const popTerm = Math.max(0, s.population) * 0.25;
	const warriorTerm = Math.max(0, s.warriors) * 0.5;
	const ageTerm = Math.min(10, (s.colonyAgeSec / 3600) * 0.04);
	return 1 + wealthTerm + popTerm + warriorTerm + ageTerm;
}

/** Add the pressure earned over `elapsedGameSec` to the running total. */
export function accrueThreat(
	pressure: number,
	s: ThreatSnapshot,
	elapsedGameSec: number,
): number {
	if (elapsedGameSec <= 0) {
		return Math.max(0, pressure);
	}
	const gained = threatRatePerHour(s) * (elapsedGameSec / 3600);
	return Math.max(0, pressure + gained);
}

/** A raid launches once accrued pressure reaches the threshold. */
export function shouldSpawnRaid(pressure: number): boolean {
	return pressure >= RAID_SPAWN_THRESHOLD;
}

/**
 * HUD threat band from the current pressure: calm below a third of the
 * threshold, rising up to two thirds, imminent above.
 */
export function threatBand(pressure: number): ThreatBand {
	if (pressure >= RAID_SPAWN_THRESHOLD * (2 / 3)) {
		return "imminent";
	}
	if (pressure >= RAID_SPAWN_THRESHOLD / 3) {
		return "rising";
	}
	return "calm";
}

export interface RaidPlan {
	/** Number of raider units in the warband. */
	count: number;
	/** Strength (and starting hp) of each raider. */
	strengthEach: number;
}

/**
 * Size the warband for a snapshot. The count scales with the standing warrior
 * count and the square-root of wealth (clamped to {@link MAX_RAID_SIZE}); each
 * raider's strength creeps up with colony age so late-game raids bite harder.
 */
export function planRaid(s: ThreatSnapshot): RaidPlan {
	const fromWarriors = Math.floor(Math.max(0, s.warriors) * 0.7);
	const fromWealth = Math.floor(Math.sqrt(Math.max(0, s.wealth)) / 6);
	const count = Math.max(
		1,
		Math.min(MAX_RAID_SIZE, 1 + fromWarriors + fromWealth),
	);
	const ageBonus = Math.min(40, (s.colonyAgeSec / 3600) * 0.5);
	const strengthEach = Math.round(RAIDER_BASE_STRENGTH + ageBonus);
	return { count, strengthEach };
}

export interface RaidOutcome {
	defendersWin: boolean;
	/** 0..1 share of stores the raiders carry off on a successful raid. */
	lootFraction: number;
	/** Cats killed defending (only on a loss, and only when it's close). */
	defenderCasualties: number;
	/** How decisively it went, for flavor: >1 rout, ~1 close, <1 defeat. */
	margin: number;
}

/** Below this power ratio a defeat also costs a defender's life. */
const CASUALTY_RATIO = 0.6;
/** Most of the stores a single raid can carry off. */
const MAX_LOOT_FRACTION = 0.35;

/**
 * Resolve a raid: mustered defense power vs the raiders' remaining strength.
 * A roll in [0,1) adds +/-25% swing so evenly-matched fights aren't fully
 * predetermined. Defenders win when their swung power meets or beats the
 * raiders; a rout leaves no casualties, a defeat bleeds stores (and, if badly
 * outmatched, a life).
 */
export function resolveRaid(
	defensePower: number,
	raiderPower: number,
	roll: number,
): RaidOutcome {
	const swing = 0.75 + 0.5 * Math.min(0.999999, Math.max(0, roll));
	const effective = defensePower * swing;
	const enemy = Math.max(1, raiderPower);
	const margin = effective / enemy;
	const defendersWin = effective >= enemy;

	if (defendersWin) {
		return {
			defendersWin: true,
			lootFraction: 0,
			defenderCasualties: 0,
			margin,
		};
	}

	// The worse the defenders lose, the more the raiders haul off.
	const shortfall = 1 - Math.min(1, margin);
	const lootFraction = Math.min(MAX_LOOT_FRACTION, 0.1 + shortfall * 0.3);
	const defenderCasualties = margin < CASUALTY_RATIO ? 1 : 0;
	return { defendersWin: false, lootFraction, defenderCasualties, margin };
}
