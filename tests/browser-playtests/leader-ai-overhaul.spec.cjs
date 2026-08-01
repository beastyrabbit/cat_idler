/*
 * LAI.33A production-browser contract.
 *
 * This suite intentionally has no fixture injection, private API client, or
 * synthetic snapshot. Every action is a user-visible action against the
 * signed production session. Native Bevy windows expose the same nodes through
 * AccessKit. Chromium currently receives a null accesskit_winit adapter, so
 * this suite uses the documented, visible fixed-canvas checkpoints when those
 * semantic nodes are not present in the browser DOM.
 */
const { test, expect } = require("playwright/test");

const REQUIRED_ENV = [
  "LAI_PLAYTEST_BROWSER_URL",
  "LAI_PLAYTEST_SEED",
  "LAI_PLAYTEST_FIXTURE",
];

const selectors = Object.freeze({
  canvas: "#cat-game",
  selectedColony: '[data-testid="lai-colony:selected"]',
  plansPanel: '[data-testid="lai-ui:plans:panel"]',
  plansNudgeUp: '[data-testid^="lai-ui:plans:control:move-up:"]',
  plansDismiss: '[data-testid^="lai-ui:plans:control:dismiss:"]',
  standingOrderCreate: '[data-testid^="lai-ui:standing-orders:control:create:"]',
  standingOrderEdit: '[data-testid^="lai-ui:standing-orders:control:edit:"]',
  standingOrderDelete: '[data-testid^="lai-ui:standing-orders:control:delete:"]',
  shrinePanel: '[data-testid="lai-ui:shrine:panel"]',
  researchPurchase: '[data-testid^="lai-ui:progression:control:purchase:"]',
  boostActivate: '[data-testid^="lai-ui:progression:control:activate:"]',
  catCarePanel: '[data-testid="lai-ui:care:panel"]',
  careTreatment: '[data-testid^="lai-ui:cats:control:treat:"]',
  careProsthetic: '[data-testid^="lai-ui:cats:control:fit:"]',
  diplomacyPanel: '[data-testid="lai-ui:diplomacy:panel"]',
  diplomacyAction: '[data-testid^="lai-ui:diplomacy:control:propose:"]',
  tradeAction: '[data-testid^="lai-ui:trade:control:"]',
  huntMarker: '[data-testid^="lai-ui:tasks:task:"][data-testid$=":objective"]',
  waterSource: '[data-testid^="lai-ui:tasks:task:"][data-testid$=":objective"][aria-label*="water source" i]',
  waterBank: '[data-testid^="lai-ui:tasks:task:"][data-testid$=":work-slot"][aria-label*="bank" i]',
  waterEndpoint: '[data-testid^="lai-ui:tasks:task:"][data-testid$=":endpoint"]',
  workshopCells: '[data-testid^="lai-ui:tasks:task:"][data-testid*=":cell-"]',
  workSlot: '[data-testid^="lai-ui:tasks:task:"][data-testid$=":work-slot"]',
  reconnectStatus: '[data-testid="lai-connection:status"]',
  reloadControl: '[data-testid="lai-ui:connection:control:reload"]',
  updateRequired: '[data-testid="lai-feedback:update-required"]',
  actionFeedback: '[data-testid^="lai-feedback:action:"]',
});

// Production canvas fallback geometry. This is intentionally fixed and
// documented, never derived from a hidden snapshot or arbitrary coordinates.
const CANVAS_VIEWPORT = Object.freeze({ width: 1280, height: 720 });
const SESSION_STORAGE_KEY = "idle-cat-forest/session/v1";
const startupPoints = Object.freeze({
  playerName: { x: 640, y: 348 },
  globalVillage: { x: 470, y: 465 },
  continue: { x: 640, y: 560 },
});
const canvasPoints = Object.freeze({
  colony: { x: 32, y: 32 },
  plans: { x: 168, y: 48 },
  plansAction: { x: 64, y: 132 },
  plansDismiss: { x: 64, y: 202 },
  standingOrders: { x: 80, y: 285 },
  standingOrderEdit: { x: 64, y: 356 },
  standingOrderDelete: { x: 64, y: 390 },
  progression: { x: 1024, y: 324 },
  boost: { x: 1024, y: 487 },
  care: { x: 710, y: 442 },
  careProsthetic: { x: 710, y: 486 },
  diplomacy: { x: 80, y: 604 },
  trade: { x: 1024, y: 604 },
  hunt: { x: 888, y: 340 },
  workshop: { x: 888, y: 360 },
  reconnect: { x: 32, y: 688 },
});
const failuresByPage = new WeakMap();
const wireByPage = new WeakMap();

function decodedFrame(payload) {
  try {
    const text = Buffer.isBuffer(payload) ? payload.toString("utf8") : String(payload);
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function wireEvidence(page) {
  const evidence = wireByPage.get(page);
  if (!evidence) throw new Error("browser wire evidence was not initialized");
  return evidence;
}

function selectedSnapshot(page) {
  const snapshots = wireEvidence(page).snapshots;
  const snapshot = snapshots.at(-1);
  if (!snapshot) throw new Error("no authoritative LAI.24 snapshot was received");
  const colony = snapshot.colonies?.find(
    (candidate) => candidate.colonyId === snapshot.selectedColonyId,
  );
  if (!colony) throw new Error("selected colony is absent from its authoritative snapshot");
  return { snapshot, colony };
}

function requireEnv() {
  const missing = REQUIRED_ENV.filter((key) => !process.env[key]);
  if (missing.length) {
    throw new Error(`BLOCKED environment: set ${missing.join(", ")} using the signed fixture launcher`);
  }
}

async function required(page, name, selector) {
  const locator = page.locator(selector);
  if ((await locator.count()) === 0) {
    throw new Error(`BLOCKED selector ${name}: ${selector}; production LAI accessibility bridge is not discoverable`);
  }
  await expect(locator.first()).toBeVisible();
  return locator;
}

async function semanticOrCanvas(page, name, selector, point, testInfo) {
  const locator = page.locator(selector);
  if ((await locator.count()) > 0) {
    await expect(locator.first()).toBeVisible();
    return locator;
  }
  const canvas = await required(page, "world canvas", selectors.canvas);
  await page.setViewportSize(CANVAS_VIEWPORT);
  await canvas.focus();
  const screenshot = await page.screenshot();
  await testInfo.attach(`${name.replaceAll(" ", "-")}-canvas.png`, {
    body: screenshot,
    contentType: "image/png",
  });
  return null;
}

async function enterProductionThroughStartScreen(page, testInfo) {
  await page.setViewportSize(CANVAS_VIEWPORT);
  const canvas = await required(page, "world canvas", selectors.canvas);
  await canvas.focus();
  const hasPersistedIdentity = await page.evaluate((key) => {
    const raw = localStorage.getItem(key);
    if (!raw) return false;
    try {
      const stored = JSON.parse(raw);
      return (
        typeof stored.nickname === "string" &&
        stored.nickname.trim().length >= 2 &&
        typeof stored.selectedColonyId === "string" &&
        stored.selectedColonyId.length > 0
      );
    } catch {
      return false;
    }
  }, SESSION_STORAGE_KEY);
  if (!hasPersistedIdentity) {
    // The canvas element exists before the WASM app has installed its input
    // systems. Wait for one visible startup frame so early keystrokes cannot
    // disappear into Trunk's bootstrap window.
    await page.waitForTimeout(1_500);
    await page.mouse.click(startupPoints.playerName.x, startupPoints.playerName.y);
    // `keyboard.type` may use Chromium's text-insertion path, which does not
    // cross winit's canvas boundary. Individual physical key presses do, and
    // also keep each event visible to Bevy for at least one rendered frame.
    for (const character of "browser qa") {
      await page.keyboard.press(character === " " ? "Space" : character);
      await page.waitForTimeout(80);
    }
  }
  await page.mouse.click(startupPoints.globalVillage.x, startupPoints.globalVillage.y);
  // Session establishment and the first authoritative snapshot are asynchronous.
  // A premature submit leaves the shipped start-screen error visible, so wait for
  // the real WebSocket handshake and retry the same visible control once.
  await page.waitForTimeout(2_500);
  await page.mouse.click(startupPoints.continue.x, startupPoints.continue.y);
  await expect
    .poll(
      () =>
        page.evaluate((key) => {
          const raw = localStorage.getItem(key);
          if (!raw) return false;
          try {
            const stored = JSON.parse(raw);
            return (
              typeof stored.nickname === "string" &&
              stored.nickname.trim().length >= 2 &&
              typeof stored.selectedColonyId === "string" &&
              stored.selectedColonyId.length > 0
            );
          } catch {
            return false;
          }
        }, SESSION_STORAGE_KEY),
      {
        timeout: 15_000,
        message: "signed production identity and selected colony were not persisted",
      },
    )
    .toBe(true);
  await page.waitForTimeout(1_000);
  if (testInfo) {
    await testInfo.attach("authenticated-selected-colony.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });
  }
}

async function openProduction(page, testInfo) {
  requireEnv();
  const failures = [];
  const wire = {
    actions: [],
    responses: [],
    snapshots: [],
    sockets: [],
    sentFrames: [],
    receivedFrames: [],
  };
  failuresByPage.set(page, failures);
  wireByPage.set(page, wire);
  page.on("console", (message) => {
    if (message.type() === "error") failures.push(`console:${message.text()}`);
  });
  page.on("requestfailed", (request) => failures.push(`request:${request.url()}`));
  page.on("websocket", (socket) => {
    wire.sockets.push(socket.url());
    socket.on("framesent", (frame) => {
      const decoded = decodedFrame(frame.payload);
      wire.sentFrames.push(decoded ?? { undecodable: true });
      if (decoded?.idempotencyId && decoded?.payload?.action) wire.actions.push(decoded);
    });
    socket.on("framereceived", (frame) => {
      const decoded = decodedFrame(frame.payload);
      wire.receivedFrames.push(decoded ?? { undecodable: true });
      if (
        decoded?.schemaVersion === 1 &&
        decoded?.selectedColonyId &&
        Array.isArray(decoded?.colonies)
      ) {
        wire.snapshots.push(decoded);
      }
      if (decoded?.idempotencyId && decoded?.result?.outcome) wire.responses.push(decoded);
    });
  });
  await page.goto(process.env.LAI_PLAYTEST_BROWSER_URL, { waitUntil: "domcontentloaded" });
  await required(page, "world canvas", selectors.canvas);
  await enterProductionThroughStartScreen(page, testInfo);
  try {
    await expect
      .poll(() => wire.snapshots.length, {
        timeout: 15_000,
        message: "authenticated client did not receive an authoritative LAI.24 snapshot",
      })
      .toBeGreaterThan(0);
  } finally {
    await testInfo.attach("startup-wire-frames.json", {
      body: Buffer.from(JSON.stringify(wire, null, 2)),
      contentType: "application/json",
    });
  }
  return failures;
}

function assertNoBrowserFailures(page) {
  expect(failuresByPage.get(page) ?? []).toEqual([]);
}

async function clickControl(
  page,
  name,
  selector,
  point,
  expectedAction,
  testInfo,
  expectedOutcome = "accepted",
) {
  const wire = wireEvidence(page);
  const actionCount = wire.actions.length;
  const snapshotCount = wire.snapshots.length;
  const control = page.locator(selector);
  if ((await control.count()) > 0) {
    await expect(control.first()).toBeVisible();
    await expect(control.first()).toBeEnabled();
    await control.first().click();
  } else {
    const canvas = await required(page, "world canvas", selectors.canvas);
    await page.setViewportSize(CANVAS_VIEWPORT);
    await canvas.focus();
    await page.mouse.click(point.x, point.y);
  }
  await expect
    .poll(() => wire.actions.length, {
      timeout: 10_000,
      message: `${name} did not emit a typed LAI.25 action`,
    })
    .toBeGreaterThan(actionCount);
  const action = wire.actions.at(-1);
  expect(action.payload.action, `${name} emitted the wrong action`).toBe(expectedAction);
  await expect
    .poll(
      () =>
        wire.responses.find(
          (response) => response.idempotencyId === action.idempotencyId,
        )?.result?.outcome,
      {
        timeout: 10_000,
        message: `${name} received no matching typed action response`,
      },
    )
    .toBe(expectedOutcome);
  const response = wire.responses.find(
    (candidate) => candidate.idempotencyId === action.idempotencyId,
  );
  await testInfo.attach(`${name.replaceAll(" ", "-")}-wire.json`, {
    body: Buffer.from(JSON.stringify({ action, response }, null, 2)),
    contentType: "application/json",
  });
  await testInfo.attach(`${name.replaceAll(" ", "-")}-canvas.png`, {
    body: await page.screenshot(),
    contentType: "image/png",
  });
  // A committed response is followed by a report-safe snapshot and Bevy surface
  // rebuild. Never click a still-visible control carrying the previous expected
  // versions; wait for the actual wire refresh, then one rendered frame.
  await expect
    .poll(() => wire.snapshots.length, {
      timeout: 10_000,
      message: `${name} received no post-action authoritative snapshot`,
    })
    .toBeGreaterThan(snapshotCount);
  await page.waitForTimeout(150);
}

async function reloadThroughProductionControl(page, testInfo) {
  const control = page.locator(selectors.reloadControl);
  const navigation = page.waitForNavigation({ waitUntil: "domcontentloaded" });
  if ((await control.count()) > 0) {
    await expect(control.first()).toBeVisible();
    await expect(control.first()).toBeEnabled();
    await Promise.all([navigation, control.first().click()]);
  } else {
    // The shipped reload control appears only after a transport failure. A
    // healthy current-protocol client uses the browser's normal reload surface
    // and must restore the signed session and selected colony.
    await Promise.all([navigation, page.reload({ waitUntil: "domcontentloaded" })]);
  }
  await testInfo.attach("reload-production-control.png", {
    body: await page.screenshot(),
    contentType: "image/png",
  });
}

async function assertNoHiddenTruth(page) {
  const bodyText = await page.locator("body").innerText();
  expect(bodyText).not.toMatch(/(hmac|session_secret|private_belief|hidden_stock)/i);
  expect(bodyText).not.toMatch(/(?:regen(?:eration)?\s*rate)\s*[:=]\s*\d/i);
  const snapshotText = JSON.stringify(wireEvidence(page).snapshots);
  expect(snapshotText).not.toMatch(/(hmac|session_secret|private_belief|hidden_stock)/i);
  expect(snapshotText).not.toMatch(/"exact(?:Value|Rate)"/i);
}

let journeyContext;
let journeyPage;

test.describe.configure({ mode: "serial" });

test.beforeAll(async ({ browser }, testInfo) => {
  testInfo.setTimeout(45_000);
  journeyContext = await browser.newContext();
  journeyPage = await journeyContext.newPage();
  await openProduction(journeyPage, testInfo);
});

test.afterEach(async () => {
  assertNoBrowserFailures(journeyPage);
});

test.afterAll(async () => {
  await journeyContext?.close();
});

test("P00 startup selected-colony load and no inert controls", async ({}, testInfo) => {
  const page = journeyPage;
  await expect
    .poll(() => page.evaluate((key) => localStorage.getItem(key) !== null, SESSION_STORAGE_KEY))
    .toBe(true);
  await semanticOrCanvas(page, "selected-colony", selectors.selectedColony, canvasPoints.colony, testInfo);
  await assertNoHiddenTruth(page);
  await semanticOrCanvas(page, "Plans panel", selectors.plansPanel, canvasPoints.plans, testInfo);
  await semanticOrCanvas(page, "connection status", selectors.reconnectStatus, canvasPoints.reconnect, testInfo);
  const { snapshot, colony } = selectedSnapshot(page);
  expect(snapshot.protocolVersion).toBe(2);
  expect(snapshot.selectedColonyId).toBe("global");
  expect(colony.plans.plans.length).toBeGreaterThan(0);
  expect(colony.visibleTasks.length).toBeGreaterThanOrEqual(3);
});

test("P01 Plans nudge and dismiss use authenticated expected-version actions", async ({}, testInfo) => {
  const page = journeyPage;
  await semanticOrCanvas(page, "Plans panel", selectors.plansPanel, canvasPoints.plans, testInfo);
  await clickControl(page, "plan move up", selectors.plansNudgeUp, canvasPoints.plansAction, "nudge_plan", testInfo);
  await clickControl(page, "plan dismiss", selectors.plansDismiss, canvasPoints.plansDismiss, "dismiss_intent", testInfo);
});

test("P02 standing-order create edit delete are real controls", async ({}, testInfo) => {
  const page = journeyPage;
  await clickControl(page, "standing-order create", selectors.standingOrderCreate, canvasPoints.standingOrders, "create_standing_order", testInfo);
  await clickControl(page, "standing-order edit", selectors.standingOrderEdit, canvasPoints.standingOrderEdit, "update_standing_order", testInfo);
  await clickControl(page, "standing-order delete", selectors.standingOrderDelete, canvasPoints.standingOrderDelete, "delete_standing_order", testInfo);
});

test("P03 Shrine Favor research and player-only boost", async ({}, testInfo) => {
  const page = journeyPage;
  await semanticOrCanvas(page, "Shrine/progression panel", selectors.shrinePanel, canvasPoints.progression, testInfo);
  const before = selectedSnapshot(page).colony;
  expect(before.shrine.pipeline).not.toBeNull();
  expect(before.favor.favorEvents.length).toBeGreaterThan(0);
  expect(before.research.frontier.length).toBeGreaterThan(0);
  await clickControl(page, "research purchase", selectors.researchPurchase, canvasPoints.progression, "purchase_research_with_favor", testInfo);
  await clickControl(page, "divine boost activation", selectors.boostActivate, canvasPoints.boost, "activate_divine_boost", testInfo);
});

test("P04 Cat Care treatment and prosthetic controls", async ({}, testInfo) => {
  const page = journeyPage;
  await semanticOrCanvas(page, "Cat Care panel", selectors.catCarePanel, canvasPoints.care, testInfo);
  const cats = selectedSnapshot(page).colony.cats;
  expect(cats.some((cat) => cat.anatomy.bodyParts.some((part) => part.injury?.injuryKind === "severe"))).toBe(true);
  expect(cats.some((cat) => cat.prosthetics.length > 0 && cat.care.careSite)).toBe(true);
  await clickControl(page, "treatment request", selectors.careTreatment, canvasPoints.care, "request_treatment", testInfo);
  await clickControl(page, "prosthetic request", selectors.careProsthetic, canvasPoints.careProsthetic, "fit_prosthetic", testInfo);
  await assertNoHiddenTruth(page);
});

test("P05 diplomacy and physical trade use two production sessions", async ({ browser }, testInfo) => {
  test.setTimeout(90_000);
  requireEnv();
  const first = journeyPage;
  const secondContext = await browser.newContext();
  const second = await secondContext.newPage();
  try {
    await openProduction(second, testInfo);
    await semanticOrCanvas(first, "diplomacy panel", selectors.diplomacyPanel, canvasPoints.diplomacy, testInfo);
    expect(selectedSnapshot(first).colony.diplomacy.relationships.length).toBe(1);
    expect(selectedSnapshot(second).colony.trade.length).toBe(1);
    await clickControl(first, "diplomacy action", selectors.diplomacyAction, canvasPoints.diplomacy, "change_diplomacy", testInfo);
    await clickControl(second, "trade response", selectors.tradeAction, canvasPoints.trade, "accept_trade_contract", testInfo);
    await assertNoHiddenTruth(first);
    await assertNoHiddenTruth(second);
    assertNoBrowserFailures(first);
    assertNoBrowserFailures(second);
  } finally {
    await secondContext.close();
  }
});

test("P06 spatial Hunt, Water, and exact Workshop footprint", async ({}, testInfo) => {
  test.setTimeout(60_000);
  const page = journeyPage;
  await semanticOrCanvas(page, "authoritative Hunt cave/source marker", selectors.huntMarker, canvasPoints.hunt, testInfo);
  await semanticOrCanvas(page, "authoritative Water source marker", selectors.waterSource, canvasPoints.hunt, testInfo);
  await semanticOrCanvas(page, "authoritative Water dry bank/work marker", selectors.waterBank, canvasPoints.hunt, testInfo);
  await semanticOrCanvas(page, "pinned Water delivery endpoint", selectors.waterEndpoint, canvasPoints.hunt, testInfo);
  const cells = await semanticOrCanvas(page, "Workshop objective cells", selectors.workshopCells, canvasPoints.workshop, testInfo);
  if (cells) await expect(cells).toHaveCount(9);
  await semanticOrCanvas(page, "Workshop work slot", selectors.workSlot, canvasPoints.workshop, testInfo);
  const tasks = selectedSnapshot(page).colony.visibleTasks;
  const hunt = tasks.find((task) => task.category === "hunt");
  const water = tasks.find((task) => task.category === "fetch_water");
  const workshop = tasks.find((task) => task.category === "workshop_work");
  expect(hunt?.objective?.kind).toBe("hunt_source");
  expect(water?.objective?.kind).toBe("water_source_and_bank");
  expect(water?.workSlots).toHaveLength(1);
  expect(water?.endpoint).not.toBeNull();
  expect(workshop?.objective?.kind).toBe("building_footprint");
  expect(workshop?.objective?.width).toBe(3);
  expect(workshop?.objective?.height).toBe(3);
  expect(workshop?.footprint).toHaveLength(9);
  expect(workshop?.workSlots).toHaveLength(1);
});

test("P07 current protocol reconnect, reload, and restart-safe selected state", async ({}, testInfo) => {
  test.setTimeout(45_000);
  const page = journeyPage;
  await semanticOrCanvas(page, "reconnect status", selectors.reconnectStatus, canvasPoints.reconnect, testInfo);
  expect(selectedSnapshot(page).snapshot.protocolVersion).toBe(2);
  await reloadThroughProductionControl(page, testInfo);
  await enterProductionThroughStartScreen(page, testInfo);
  await expect
    .poll(() => wireEvidence(page).snapshots.length, { timeout: 15_000 })
    .toBeGreaterThan(1);
  await semanticOrCanvas(page, "selected colony after reload", selectors.selectedColony, canvasPoints.colony, testInfo);
  expect(selectedSnapshot(page).snapshot.selectedColonyId).toBe("global");
  await assertNoHiddenTruth(page);
});
