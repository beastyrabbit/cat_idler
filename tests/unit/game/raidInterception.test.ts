import { describe, expect, it } from "vitest";

import {
  RAID_INTERCEPTION_RADIUS,
  resolveRaidInterception,
  selectRaidInterceptions,
  type InterceptableCat,
  type InterceptingRaider,
} from "@/lib/game/raidInterception";
import { MAX_RAID_CASUALTIES } from "@/lib/game/threat";
import { rollSeeded } from "@/lib/game/seededRng";

const cat = (overrides: Partial<InterceptableCat> = {}): InterceptableCat => ({
  id: "cat-a",
  position: { x: 10, y: 10 },
  activity: "traveling",
  currentTask: "hunt_expedition",
  carrying: null,
  deathTime: null,
  stats: { attack: 50, defense: 50 },
  specialization: null,
  ageHours: 30,
  roleXp: null,
  ...overrides,
});

const raider = (
  overrides: Partial<InterceptingRaider> = {},
): InterceptingRaider => ({
  id: "raider-a",
  position: { x: 12, y: 10 },
  hp: 40,
  strength: 40,
  status: "advancing",
  ...overrides,
});

describe("raid interception", () => {
  it("selects cats within the Chebyshev radius and excludes the boundary outside it", () => {
    const inside = () => false;
    expect(selectRaidInterceptions([raider()], [cat()], inside)).toHaveLength(
      1,
    );
    expect(
      selectRaidInterceptions(
        [raider()],
        [cat({ position: { x: 9 - RAID_INTERCEPTION_RADIUS, y: 10 } })],
        inside,
      ),
    ).toHaveLength(0);
  });

  it("selects crossings along accelerated movement trails", () => {
    const inside = () => false;
    const pairs = selectRaidInterceptions(
      [
        raider({
          position: { x: 8, y: 5 },
          trail: [
            { x: 8, y: 5 },
            { x: 8, y: 4 },
            { x: 8, y: 3 },
            { x: 8, y: 2 },
          ],
        }),
      ],
      [
        cat({
          position: { x: 0, y: 0 },
          trail: [
            { x: 0, y: 0 },
            { x: 1, y: 0 },
            { x: 2, y: 0 },
            { x: 3, y: 0 },
            { x: 4, y: 0 },
            { x: 5, y: 0 },
            { x: 6, y: 0 },
            { x: 7, y: 0 },
            { x: 8, y: 0 },
          ],
        }),
      ],
      inside,
    );

    expect(pairs).toHaveLength(1);
    expect(pairs[0]?.distance).toBe(RAID_INTERCEPTION_RADIUS);
  });

  it("does not select cats behind the fence", () => {
    expect(
      selectRaidInterceptions([raider()], [cat()], () => true),
    ).toHaveLength(0);
  });

  it("resolves seeded skirmish outcomes deterministically", () => {
    const rollA = rollSeeded(101).value;
    const rollB = rollSeeded(101).value;
    const first = resolveRaidInterception(
      cat({ stats: { attack: 35, defense: 35 }, specialization: "hunter" }),
      raider({ hp: 45, strength: 45 }),
      rollA,
      0,
    );
    const second = resolveRaidInterception(
      cat({ stats: { attack: 35, defense: 35 }, specialization: "hunter" }),
      raider({ hp: 45, strength: 45 }),
      rollB,
      0,
    );
    expect(first).toEqual(second);
    expect(first.outcome).toBe("flee");
  });

  it("returns the legible outcome bands", () => {
    expect(
      resolveRaidInterception(
        cat({ stats: { attack: 80, defense: 80 }, specialization: "warrior" }),
        raider({ hp: 40, strength: 40 }),
        0.5,
        0,
      ).outcome,
    ).toBe("escape");
    expect(
      resolveRaidInterception(cat(), raider({ hp: 55, strength: 55 }), 0, 0)
        .outcome,
    ).toBe("wounded");
    expect(
      resolveRaidInterception(cat(), raider({ hp: 300, strength: 300 }), 0, 0)
        .outcome,
    ).toBe("killed");
  });

  it("enforces the raid casualty cap by downgrading lethal interceptions to wounds", () => {
    const result = resolveRaidInterception(
      cat(),
      raider({ hp: 300, strength: 300 }),
      0,
      MAX_RAID_CASUALTIES,
    );
    expect(result.outcome).toBe("wounded");
    expect(result.casualty).toBe(false);
  });

  it("selects returning cats only while they still carry yield or field work", () => {
    const inside = () => false;
    expect(
      selectRaidInterceptions(
        [raider()],
        [
          cat({
            activity: "returning",
            currentTask: null,
            carrying: { kind: "food", amount: 3 },
          }),
        ],
        inside,
      ),
    ).toHaveLength(1);
    expect(
      selectRaidInterceptions(
        [raider()],
        [cat({ activity: "returning", currentTask: null, carrying: null })],
        inside,
      ),
    ).toHaveLength(0);
  });
});
