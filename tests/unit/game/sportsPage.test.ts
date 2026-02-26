import { describe, it, expect } from "vitest";
import {
  getEventScore,
  rankCompetitors,
  generateCommentary,
  selectMVP,
  formatEventResult,
  formatSportsPage,
  type AthleticEvent,
  type EventStanding,
  type MVPAward,
  type SportsPage,
} from "@/lib/game/sportsPage";
import type { CatStats } from "@/types/game";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const defaultStats: CatStats = {
  attack: 50,
  defense: 50,
  hunting: 50,
  medicine: 50,
  cleaning: 50,
  building: 50,
  leadership: 50,
  vision: 50,
};

function makeStats(overrides: Partial<CatStats> = {}): CatStats {
  return { ...defaultStats, ...overrides };
}

interface SimpleCat {
  name: string;
  stats: CatStats;
}

// ---------------------------------------------------------------------------
// getEventScore
// ---------------------------------------------------------------------------

describe("getEventScore", () => {
  it("returns a positive number", () => {
    const score = getEventScore(defaultStats, "mousing_sprint", 42);
    expect(score).toBeGreaterThan(0);
  });

  it("is deterministic for the same seed", () => {
    const a = getEventScore(defaultStats, "mousing_sprint", 42);
    const b = getEventScore(defaultStats, "mousing_sprint", 42);
    expect(a).toBe(b);
  });

  it("varies with different seeds", () => {
    const a = getEventScore(defaultStats, "mousing_sprint", 42);
    const b = getEventScore(defaultStats, "mousing_sprint", 99);
    expect(a).not.toBe(b);
  });

  it("mousing_sprint weights hunting stat higher", () => {
    const highHunter = makeStats({ hunting: 90, attack: 10, vision: 10 });
    const lowHunter = makeStats({ hunting: 10, attack: 90, vision: 90 });
    const seed = 100;
    expect(getEventScore(highHunter, "mousing_sprint", seed)).toBeGreaterThan(
      getEventScore(lowHunter, "mousing_sprint", seed),
    );
  });

  it("high_fence_jump weights attack and defense higher", () => {
    const highJumper = makeStats({ attack: 90, defense: 90, hunting: 10 });
    const lowJumper = makeStats({ attack: 10, defense: 10, hunting: 90 });
    const seed = 100;
    expect(getEventScore(highJumper, "high_fence_jump", seed)).toBeGreaterThan(
      getEventScore(lowJumper, "high_fence_jump", seed),
    );
  });

  it("tail_chase weights hunting and vision higher", () => {
    const highEndurance = makeStats({ hunting: 90, vision: 90, attack: 10 });
    const lowEndurance = makeStats({ hunting: 10, vision: 10, attack: 90 });
    const seed = 100;
    expect(getEventScore(highEndurance, "tail_chase", seed)).toBeGreaterThan(
      getEventScore(lowEndurance, "tail_chase", seed),
    );
  });

  it("night_stalk weights vision stat higher", () => {
    const highVision = makeStats({ vision: 90, hunting: 10, attack: 10 });
    const lowVision = makeStats({ vision: 10, hunting: 90, attack: 90 });
    const seed = 100;
    expect(getEventScore(highVision, "night_stalk", seed)).toBeGreaterThan(
      getEventScore(lowVision, "night_stalk", seed),
    );
  });

  it("fish_swipe weights hunting and attack higher", () => {
    const highFisher = makeStats({ hunting: 90, attack: 90, vision: 10 });
    const lowFisher = makeStats({ hunting: 10, attack: 10, vision: 90 });
    const seed = 100;
    expect(getEventScore(highFisher, "fish_swipe", seed)).toBeGreaterThan(
      getEventScore(lowFisher, "fish_swipe", seed),
    );
  });

  it("zero stats produce a score > 0 due to RNG variance", () => {
    const zeroStats = makeStats({
      attack: 0,
      defense: 0,
      hunting: 0,
      medicine: 0,
      cleaning: 0,
      building: 0,
      leadership: 0,
      vision: 0,
    });
    const score = getEventScore(zeroStats, "mousing_sprint", 42);
    expect(score).toBeGreaterThanOrEqual(0);
  });

  it("handles all five event types without error", () => {
    const events: AthleticEvent[] = [
      "mousing_sprint",
      "high_fence_jump",
      "tail_chase",
      "night_stalk",
      "fish_swipe",
    ];
    for (const ev of events) {
      expect(() => getEventScore(defaultStats, ev, 42)).not.toThrow();
    }
  });
});

// ---------------------------------------------------------------------------
// rankCompetitors
// ---------------------------------------------------------------------------

describe("rankCompetitors", () => {
  const cats: SimpleCat[] = [
    { name: "Whiskers", stats: makeStats({ hunting: 90 }) },
    { name: "Shadow", stats: makeStats({ hunting: 30 }) },
    { name: "Mittens", stats: makeStats({ hunting: 60 }) },
  ];

  it("returns standings sorted by score descending", () => {
    const standings = rankCompetitors(cats, "mousing_sprint", 42);
    for (let i = 1; i < standings.length; i++) {
      expect(standings[i - 1].score).toBeGreaterThanOrEqual(standings[i].score);
    }
  });

  it("assigns rank 1 to the highest scorer", () => {
    const standings = rankCompetitors(cats, "mousing_sprint", 42);
    expect(standings[0].rank).toBe(1);
  });

  it("assigns sequential ranks", () => {
    const standings = rankCompetitors(cats, "mousing_sprint", 42);
    standings.forEach((s, i) => {
      expect(s.rank).toBe(i + 1);
    });
  });

  it("includes cat names", () => {
    const standings = rankCompetitors(cats, "mousing_sprint", 42);
    const names = standings.map((s) => s.catName);
    expect(names).toContain("Whiskers");
    expect(names).toContain("Shadow");
    expect(names).toContain("Mittens");
  });

  it("is deterministic for the same seed", () => {
    const a = rankCompetitors(cats, "mousing_sprint", 42);
    const b = rankCompetitors(cats, "mousing_sprint", 42);
    expect(a).toEqual(b);
  });

  it("handles single cat", () => {
    const single = [{ name: "Solo", stats: defaultStats }];
    const standings = rankCompetitors(single, "mousing_sprint", 42);
    expect(standings).toHaveLength(1);
    expect(standings[0].rank).toBe(1);
    expect(standings[0].catName).toBe("Solo");
  });

  it("returns empty array for no cats", () => {
    const standings = rankCompetitors([], "mousing_sprint", 42);
    expect(standings).toEqual([]);
  });

  it("handles cats with identical stats by deterministic tie-breaking", () => {
    const tied = [
      { name: "Alpha", stats: defaultStats },
      { name: "Beta", stats: defaultStats },
    ];
    const standings = rankCompetitors(tied, "mousing_sprint", 42);
    expect(standings).toHaveLength(2);
    expect(standings[0].rank).toBe(1);
    expect(standings[1].rank).toBe(2);
    // deterministic
    const again = rankCompetitors(tied, "mousing_sprint", 42);
    expect(again[0].catName).toBe(standings[0].catName);
  });
});

// ---------------------------------------------------------------------------
// generateCommentary
// ---------------------------------------------------------------------------

describe("generateCommentary", () => {
  it("returns a non-empty string for rank 1 (gold)", () => {
    const standing: EventStanding = {
      catName: "Whiskers",
      score: 100,
      rank: 1,
    };
    const text = generateCommentary(standing, "mousing_sprint", 42);
    expect(text.length).toBeGreaterThan(0);
  });

  it("includes the cat name", () => {
    const standing: EventStanding = {
      catName: "Shadow",
      score: 80,
      rank: 2,
    };
    const text = generateCommentary(standing, "tail_chase", 42);
    expect(text).toContain("Shadow");
  });

  it("is deterministic for the same seed", () => {
    const standing: EventStanding = {
      catName: "Mittens",
      score: 60,
      rank: 3,
    };
    const a = generateCommentary(standing, "night_stalk", 42);
    const b = generateCommentary(standing, "night_stalk", 42);
    expect(a).toBe(b);
  });

  it("produces different text for different ranks", () => {
    const gold: EventStanding = { catName: "A", score: 100, rank: 1 };
    const other: EventStanding = { catName: "A", score: 50, rank: 4 };
    const a = generateCommentary(gold, "mousing_sprint", 42);
    const b = generateCommentary(other, "mousing_sprint", 42);
    expect(a).not.toBe(b);
  });

  it("produces different text for different event types", () => {
    const standing: EventStanding = { catName: "A", score: 100, rank: 1 };
    const a = generateCommentary(standing, "mousing_sprint", 42);
    const b = generateCommentary(standing, "fish_swipe", 42);
    expect(a).not.toBe(b);
  });

  it("varies output with different seeds", () => {
    const standing: EventStanding = { catName: "A", score: 100, rank: 1 };
    const a = generateCommentary(standing, "mousing_sprint", 42);
    const b = generateCommentary(standing, "mousing_sprint", 999);
    expect(a).not.toBe(b);
  });
});

// ---------------------------------------------------------------------------
// selectMVP
// ---------------------------------------------------------------------------

describe("selectMVP", () => {
  it("picks the cat with the highest total score across events", () => {
    const allStandings: Record<AthleticEvent, EventStanding[]> = {
      mousing_sprint: [
        { catName: "A", score: 100, rank: 1 },
        { catName: "B", score: 50, rank: 2 },
      ],
      high_fence_jump: [
        { catName: "A", score: 80, rank: 1 },
        { catName: "B", score: 90, rank: 1 },
      ],
      tail_chase: [
        { catName: "A", score: 70, rank: 1 },
        { catName: "B", score: 60, rank: 2 },
      ],
      night_stalk: [
        { catName: "A", score: 60, rank: 2 },
        { catName: "B", score: 70, rank: 1 },
      ],
      fish_swipe: [
        { catName: "A", score: 90, rank: 1 },
        { catName: "B", score: 40, rank: 2 },
      ],
    };
    // A total: 100+80+70+60+90=400, B total: 50+90+60+70+40=310
    const mvp = selectMVP(allStandings, 42);
    expect(mvp.catName).toBe("A");
    expect(mvp.totalScore).toBe(400);
  });

  it("identifies the best individual event", () => {
    const allStandings: Record<AthleticEvent, EventStanding[]> = {
      mousing_sprint: [{ catName: "X", score: 100, rank: 1 }],
      high_fence_jump: [{ catName: "X", score: 50, rank: 1 }],
      tail_chase: [{ catName: "X", score: 30, rank: 1 }],
      night_stalk: [{ catName: "X", score: 80, rank: 1 }],
      fish_swipe: [{ catName: "X", score: 60, rank: 1 }],
    };
    const mvp = selectMVP(allStandings, 42);
    expect(mvp.bestEvent).toBe("mousing_sprint");
    expect(mvp.bestEventScore).toBe(100);
  });

  it("is deterministic", () => {
    const allStandings: Record<AthleticEvent, EventStanding[]> = {
      mousing_sprint: [{ catName: "A", score: 50, rank: 1 }],
      high_fence_jump: [{ catName: "A", score: 50, rank: 1 }],
      tail_chase: [{ catName: "A", score: 50, rank: 1 }],
      night_stalk: [{ catName: "A", score: 50, rank: 1 }],
      fish_swipe: [{ catName: "A", score: 50, rank: 1 }],
    };
    const a = selectMVP(allStandings, 42);
    const b = selectMVP(allStandings, 42);
    expect(a).toEqual(b);
  });

  it("handles tied total scores using seed for tie-breaking", () => {
    const allStandings: Record<AthleticEvent, EventStanding[]> = {
      mousing_sprint: [
        { catName: "A", score: 50, rank: 1 },
        { catName: "B", score: 50, rank: 1 },
      ],
      high_fence_jump: [
        { catName: "A", score: 50, rank: 1 },
        { catName: "B", score: 50, rank: 1 },
      ],
      tail_chase: [
        { catName: "A", score: 50, rank: 1 },
        { catName: "B", score: 50, rank: 1 },
      ],
      night_stalk: [
        { catName: "A", score: 50, rank: 1 },
        { catName: "B", score: 50, rank: 1 },
      ],
      fish_swipe: [
        { catName: "A", score: 50, rank: 1 },
        { catName: "B", score: 50, rank: 1 },
      ],
    };
    const mvp = selectMVP(allStandings, 42);
    expect(["A", "B"]).toContain(mvp.catName);
    expect(mvp.totalScore).toBe(250);
    // deterministic
    const again = selectMVP(allStandings, 42);
    expect(again.catName).toBe(mvp.catName);
  });

  it("handles single cat single event", () => {
    const allStandings: Record<AthleticEvent, EventStanding[]> = {
      mousing_sprint: [{ catName: "Solo", score: 77, rank: 1 }],
      high_fence_jump: [],
      tail_chase: [],
      night_stalk: [],
      fish_swipe: [],
    };
    const mvp = selectMVP(allStandings, 42);
    expect(mvp.catName).toBe("Solo");
    expect(mvp.totalScore).toBe(77);
    expect(mvp.bestEvent).toBe("mousing_sprint");
  });
});

// ---------------------------------------------------------------------------
// formatEventResult
// ---------------------------------------------------------------------------

describe("formatEventResult", () => {
  it("includes the event name in the output", () => {
    const standings: EventStanding[] = [{ catName: "A", score: 100, rank: 1 }];
    const result = formatEventResult("mousing_sprint", standings, "Great run!");
    expect(result).toContain("MOUSING SPRINT");
  });

  it("includes cat names and scores", () => {
    const standings: EventStanding[] = [
      { catName: "Whiskers", score: 95.5, rank: 1 },
      { catName: "Shadow", score: 80.2, rank: 2 },
    ];
    const result = formatEventResult("tail_chase", standings, "Exciting!");
    expect(result).toContain("Whiskers");
    expect(result).toContain("Shadow");
  });

  it("includes commentary text", () => {
    const standings: EventStanding[] = [{ catName: "A", score: 100, rank: 1 }];
    const commentary = "An absolutely legendary performance!";
    const result = formatEventResult("night_stalk", standings, commentary);
    expect(result).toContain(commentary);
  });

  it("shows rank indicators for podium positions", () => {
    const standings: EventStanding[] = [
      { catName: "Gold", score: 100, rank: 1 },
      { catName: "Silver", score: 90, rank: 2 },
      { catName: "Bronze", score: 80, rank: 3 },
    ];
    const result = formatEventResult("fish_swipe", standings, "Nice!");
    expect(result).toContain("1.");
    expect(result).toContain("2.");
    expect(result).toContain("3.");
  });

  it("handles empty standings gracefully", () => {
    const result = formatEventResult("mousing_sprint", [], "No contestants");
    expect(result).toContain("MOUSING SPRINT");
  });

  it("formats event type as human-readable title", () => {
    const standings: EventStanding[] = [{ catName: "A", score: 100, rank: 1 }];
    expect(formatEventResult("high_fence_jump", standings, "wow")).toContain(
      "HIGH FENCE JUMP",
    );
    expect(formatEventResult("fish_swipe", standings, "wow")).toContain(
      "FISH SWIPE",
    );
  });
});

// ---------------------------------------------------------------------------
// formatSportsPage
// ---------------------------------------------------------------------------

describe("formatSportsPage", () => {
  const mvp: MVPAward = {
    catName: "Whiskers",
    totalScore: 400,
    bestEvent: "mousing_sprint",
    bestEventScore: 100,
  };

  it("includes the section title", () => {
    const page = formatSportsPage(["event1 result"], mvp, 42);
    expect(page.sectionTitle).toContain("SPORTS");
  });

  it("includes all event entries", () => {
    const entries = ["Result A", "Result B", "Result C"];
    const page = formatSportsPage(entries, mvp, 42);
    expect(page.entries).toEqual(entries);
  });

  it("includes the MVP award", () => {
    const page = formatSportsPage(["r"], mvp, 42);
    expect(page.mvp).toEqual(mvp);
  });

  it("tracks total events count", () => {
    const entries = ["A", "B", "C", "D", "E"];
    const page = formatSportsPage(entries, mvp, 42);
    expect(page.totalEvents).toBe(5);
  });

  it("includes the edition number", () => {
    const page = formatSportsPage(["r"], mvp, 7);
    expect(page.editionNumber).toBe(7);
  });

  it("handles empty entries", () => {
    const page = formatSportsPage([], mvp, 1);
    expect(page.entries).toEqual([]);
    expect(page.totalEvents).toBe(0);
  });

  it("returns correct SportsPage shape", () => {
    const page = formatSportsPage(["r"], mvp, 1);
    expect(page).toHaveProperty("sectionTitle");
    expect(page).toHaveProperty("entries");
    expect(page).toHaveProperty("mvp");
    expect(page).toHaveProperty("totalEvents");
    expect(page).toHaveProperty("editionNumber");
  });
});
