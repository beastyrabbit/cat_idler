import { rollSeeded } from "./seededRng";

// ── Types ────────────────────────────────────────────────────────────

export type TradeResource = "food" | "water" | "herbs" | "materials";

export interface TradeRoute {
  destination: string;
  resource: TradeResource;
  amount: number;
  capacity: number;
  distance: number;
  dangerLevel: number;
  demandMultiplier: number;
}

export interface TradeRunResult {
  destination: string;
  resource: TradeResource;
  goodsSent: number;
  goodsArrived: number;
  profit: number;
  success: boolean;
}

export interface TradeBatchResult {
  runs: TradeRunResult[];
  totalProfit: number;
  successfulRuns: number;
  failedRuns: number;
}

// ── Constants ────────────────────────────────────────────────────────

const BASE_VALUES: Record<TradeResource, number> = {
  food: 1,
  water: 1.5,
  herbs: 3,
  materials: 2,
};

const SURPLUS_THRESHOLD = 0.5;
const SCARCITY_THRESHOLD = 0.2;
const SURPLUS_MODIFIER = 1.2;
const SCARCITY_MODIFIER = 1.5;

// ── Functions ────────────────────────────────────────────────────────

/**
 * Calculate the trade value of goods based on resource type, amount, and capacity.
 * Surplus (>50% capacity) adds 20% value. Scarcity (<20% capacity) adds 50% value.
 */
export function calculateGoodsValue(
  resource: TradeResource,
  amount: number,
  capacity: number,
): number {
  if (amount === 0) return 0;

  const baseValue = BASE_VALUES[resource];
  const ratio = capacity > 0 ? amount / capacity : 0;

  let modifier = 1.0;
  if (ratio > SURPLUS_THRESHOLD) {
    modifier = SURPLUS_MODIFIER;
  } else if (ratio < SCARCITY_THRESHOLD) {
    modifier = SCARCITY_MODIFIER;
  }

  return Math.round(amount * baseValue * modifier);
}

/**
 * Score route profitability: revenue minus distance cost minus danger-adjusted expected losses.
 */
export function scoreRouteProfitability(
  goodsValue: number,
  distance: number,
  dangerLevel: number,
  demandMultiplier: number,
): number {
  const revenue = goodsValue * demandMultiplier;
  const dangerFraction = dangerLevel / 100;
  const expectedLoss = goodsValue * demandMultiplier * dangerFraction;
  return Math.round(revenue - distance - expectedLoss);
}

/**
 * Simulate a single trade run with seeded RNG. Returns goods arrived and profit.
 */
export function simulateTradeRun(
  route: TradeRoute,
  rngSeed: number,
): TradeRunResult {
  const goodsValue = calculateGoodsValue(
    route.resource,
    route.amount,
    route.capacity,
  );
  const dangerFraction = route.dangerLevel / 100;

  const roll = rollSeeded(rngSeed);
  // Success if roll >= danger fraction (0 danger = always success)
  const success = roll.value >= dangerFraction;

  let goodsArrived: number;
  if (success) {
    goodsArrived = route.amount;
  } else {
    // Lose a portion proportional to danger level
    const lostFraction = dangerFraction;
    goodsArrived = Math.max(0, Math.round(route.amount * (1 - lostFraction)));
  }

  const arrivedValue = calculateGoodsValue(
    route.resource,
    goodsArrived,
    route.capacity,
  );
  const profit = Math.round(
    arrivedValue * route.demandMultiplier - route.distance,
  );

  return {
    destination: route.destination,
    resource: route.resource,
    goodsSent: route.amount,
    goodsArrived,
    profit,
    success,
  };
}

/**
 * Evaluate a batch of trade routes with chained seeded RNG.
 */
export function evaluateTradeBatch(
  routes: TradeRoute[],
  rngSeed: number,
): TradeBatchResult {
  if (routes.length === 0) {
    return { runs: [], totalProfit: 0, successfulRuns: 0, failedRuns: 0 };
  }

  const runs: TradeRunResult[] = [];
  let currentSeed = rngSeed;

  for (const route of routes) {
    const run = simulateTradeRun(route, currentSeed);
    runs.push(run);
    // Chain the seed for the next route
    const nextRoll = rollSeeded(currentSeed);
    currentSeed = nextRoll.nextSeed;
  }

  const totalProfit = runs.reduce((sum, r) => sum + r.profit, 0);
  const successfulRuns = runs.filter((r) => r.success).length;
  const failedRuns = runs.filter((r) => !r.success).length;

  return { runs, totalProfit, successfulRuns, failedRuns };
}

/**
 * Generate a newspaper "Market & Commerce" report from trade batch results.
 */
export function generateMarketReport(
  batchResult: TradeBatchResult,
  colonyName: string,
): string {
  const { runs, totalProfit, successfulRuns, failedRuns } = batchResult;

  if (runs.length === 0) {
    return `MARKET & COMMERCE — ${colonyName}\n\nA quiet day on the trade front. No caravans departed from ${colonyName} today.`;
  }

  const tradeWord =
    runs.length === 1 ? "1 trade route" : `${runs.length} trade routes`;
  const lines: string[] = [
    `MARKET & COMMERCE — ${colonyName}`,
    "",
    `${colonyName} dispatched goods along ${tradeWord} today.`,
    "",
  ];

  for (const run of runs) {
    const status = run.success ? "arrived safely" : "encountered trouble";
    lines.push(
      `• ${run.resource} to ${run.destination}: ${run.goodsSent} sent, ${run.goodsArrived} ${status}. Profit: ${run.profit}.`,
    );
  }

  lines.push("");

  if (failedRuns > 0) {
    lines.push(
      `Of ${runs.length} trade runs, ${successfulRuns} succeeded and ${failedRuns} met misfortune.`,
    );
  } else {
    lines.push(`All ${runs.length} trade runs completed successfully.`);
  }

  lines.push(`Total profit for ${colonyName}: ${totalProfit}.`);

  return lines.join("\n");
}
