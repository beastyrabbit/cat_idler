# UI/UX Review — cat-client (Bevy 0.19 top-down renderer)

> Post-review status: findings have been dispositioned and implemented where required. See
> [`RESOLUTION.md`](RESOLUTION.md); this file preserves the pre-fix evidence.

Date: 2026-07-16
Scope: usability heuristics + idle/god-game genre expectations for the Bevy client, plus a live
run of the native desktop client against a real server. Client *code* findings (perf, churn,
architecture) live in `CODE_REVIEW.md` Part 3; this document is player-facing UX.

## Verdict

**Well-architected and genuinely playable.** The live run rendered correctly on a real GPU
(AMD 7900 XTX / Vulkan), connected to the server, and showed a populated, readable top-down
colony with a coherent parchment/"Adventure" HUD, working command dock, minimap, and inspectors.
No correctness bugs were found in the client. The weaknesses are almost entirely **first-run
guidance and system-status legibility**, not correctness. The single most important issue: after
the first snapshot, a disconnect is nearly invisible — the last world stays on screen looking
live while the only warning is an 8-second toast, even though reconnect backoff runs up to 30 s.

Live run: **WORKED.** BRP is available (`CAT_BRP=1`, `RemotePlugin` only — no `brp_extras`, so
screenshots were taken with `grim`).

## Findings

### High

- **H1 — A disconnect is invisible after the first snapshot; the frozen world looks live.**
  `update_hud` shows "connecting…" only when `LatestSnapshot.0` is `None`
  (`cat-client/src/lib.rs:9877-9878`), but that field is set to `Some(..)` on every snapshot
  (`:5429`) and never cleared on disconnect — `schedule_reconnect` (`:5596`) resets connection
  state but leaves the last snapshot in place. The only disconnect signal is the feedback toast,
  whose error TTL is 8 s (`:5639-5640`) while `MAX_RECONNECT_DELAY_SECS` is 30 s (`:80`). For an
  idle game — exactly when the player isn't watching — the colony looks normal but is silently
  stale, with no persistent indicator.
  *Why it matters:* visibility of system status is the one heuristic an idle game can't afford
  to fail; players make decisions against data they believe is live.
  *Recommendation:* drive a persistent connection badge off `ConnectionState.phase` (it already
  tracks `WaitingToRetry`/`Connecting`) — an always-visible "Reconnecting… (attempt N)" chip
  and/or desaturate the world while `phase != Connected`; keep it alive for the whole backoff
  window instead of an 8-s toast.

- **H2 — No onboarding / first-run guidance.** No tutorial, hints, welcome, or help exist
  anywhere. A new player is auto-joined to the shared Grand Commons (good — not a blank screen;
  confirmed live), but the 6-family command dock (Inspect/Gather/Build/Territory/Scout/Village),
  the god-actions, and the 487-study ledger are entirely learn-by-clicking. The only nudge is a
  header line shown when not joined to any colony (`:9887`).
  *Why it matters:* this is a deep systems game (108 recipes, 487 studies, 7 officer roles); no
  first-run scaffolding is a steep wall.
  *Recommendation:* a dismissible first-run overlay naming the dock families and the
  `[L]`/`[G]`/`[C]`/`[U]` panels plus a "what to do first" line. Even a static legend is a large
  win.

### Medium

- **M1 — Cats are visually near-identical on the map; role is not glanceable, and the docs are
  wrong about how.** The cat body sprite group is *facing direction*, not role
  (`lib.rs:8292-8295`); every cat uses the same sheet. Specialization is conveyed only by a hat
  overlay, and only for 4 specializations (Hunter/Architect/Ritualist/Warrior,
  `:1997-2003`/`:8330-8345`). In the live capture all cats read as the same pinkish sprite at
  default zoom. **`CLAUDE.md`/`GAME_VISION.md` say cats are "colored by specialization" — the
  implementation does not tint by specialization; reconcile the doc** (cross-ref
  `FEATURE_REVIEW.md`).
  *Why it matters:* the DF-style "readable living workplace" pillar depends on seeing who does
  what; today you must click each cat.
  *Recommendation:* a stronger role cue (role-tinted collar/body tint or a larger role glyph)
  covering all roles, legible at default zoom. Keep the shape-based approach (hats) for
  colorblind users — extend it rather than relying on color alone.

- **M2 — No keyboard-shortcut reference (recognition vs recall).** Useful shortcuts exist —
  L/G/C/U panels, O officers, P orders, R camera reset, `/` ledger search (`:135-137`) — and
  some buttons embed the key (e.g. "Log [L]", `:4495`), but there is no consolidated in-app
  list. *Recommendation:* a `?`/H help overlay listing shortcuts and dock families.

- **M3 — Resource-spending actions fire on a single click with no confirmation.**
  Offerings/tithes (`:7508-7525`), officer appointment (`:7635`), boost toggle (`:7677`), and
  research purchases (`research_ui.rs handle_research_purchase`) all dispatch on a single
  `Interaction::Pressed`; no confirm dialog exists anywhere. Purchases are affordability-gated
  (`research_ui.rs:1349`) and every result is echoed via the feedback panel (`lib.rs:5464-5473`),
  which mitigates this — but a misclick still spends irreversibly.
  *Recommendation:* no modal needed for cheap/regenerating actions; consider a "spent X"
  confirmation and/or hold-to-confirm for the largest offerings. Acceptable as-is given gating +
  echo; flagged for awareness.

### Low

- **L1 — Reconnect countdown text is static.** The banner says "Retrying in 30s (attempt N)"
  once (`:5613`); the number never ticks down (`:5619`). Fix with H1.
- **L2 — Dead FUTURE research UI paths remain.** The ledger still renders a "FUTURE" state and
  "Planned content — not yet researchable" copy (`research_ui.rs:1145,1323`,
  `PurchaseState::Future`) even though sim tests assert no node is future. Harmless dead code but
  can mislead a maintainer.
- **L3 — Hardcoded nickname "Desktop Cat"** sent on officer/boost actions (`lib.rs:7637,7679`) —
  cosmetic, native-only.

### Strengths (UX)

- Stale-count discipline is excellent — the HUD prefixes approximate stores with `~` when the
  Accountant ledger is inaccurate (`lib.rs:9906-9917`), matching the DF-bookkeeper design
  (confirmed live: "STORES STALE ~").
- Boosted cats get a gold star, carried goods show a semantic icon, the selected cat gets a ring
  (`:8323-8355`).
- Responsive/compact HUD at the 1024 px floor; a vanished-village selection is handled
  gracefully with a user-facing message (`:5439-5443`).

## Live run — what was seen

Server + desktop client launched clean (Vulkan adapter, WS connect, BRP on :15702). Captured via
`grim` (Wayland). The scene: top tab bar (IDLE CAT FOREST / Log [L] / Stores [G] / Census [C] /
Tree [U]) and a top-right event ticker; colony panel "THE GRAND COMMONS [THRIVING], LEADER
FLINTLEAF · LV 3, POP 30/30 · HOUSED 30 · WAIT 0 · OUT 0, THREAT CALM 0"; resource bars (FOOD
~100/600, WATER ~200/200, MATERIALS ~120/200, MEDICINE ~07/100, "JOBS 10/10 · STORES STALE ~");
a walled village with paved paths, red-roofed residences, open workshops, a golden central
shrine, visible stockpiles, cats, and water on the east edge; the bottom dock
(Inspect/Gather/Build/Territory/Scout/Village) and a circular minimap. No black-screen, no
sprite clipping/overlap, good contrast, legible HUD; the window scaled cleanly from its 1024×768
initial resolution. (BRP `world.query` for Text returns 0 entities — the client deliberately
doesn't register render/UI types for reflection (`:3607`); not a defect.)

## Priorities

1. **H1** — persistent disconnect status (highest ROI, small change).
2. **H2** — first-run onboarding overlay.
3. **M1** — glanceable cat roles (and fix the "colored by specialization" doc claim).
4. The main-thread double-parse (see `CODE_REVIEW.md` Part 3) is the one perf item worth
   addressing before scaling world size.
