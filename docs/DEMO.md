# Idle Cat Forest demo brief

Idle Cat Forest is a forest colony game in development for Steam; the demo is free;
pricing undecided.

This is the shared brief for the demo push. It states what the demo is, what is in and
out of scope, the product decisions every change follows, what only the owner can do and
where each wave stands. The wave plan that splits the work into packages is kept outside
the repository; references below to "rule 2.x" and "section 3.x" point at that plan. The
appendix keeps the plan's section 3 numbering so those references still resolve.

Every agent reads this file together with [AGENTS.md](../AGENTS.md). If they disagree on
the demo's scope or positioning, this file wins.

## The demo bar and the journey

**Bar.** A stranger launches the app, plays for 25 to 35 minutes and wants to wishlist it.
Judge every change by what that stranger sees in a trailer and feels in the first minutes:
how the game looks, how the cats behave, the UI, onboarding, feedback, audio and stability.

**The demo journey (target timings at 1x speed, demo pacing profile, average player).**
These are the D6 targets. W2-INT-a asserts the simulation part, W4-03 tunes against them and
W4-INT and W5-INT measure them in the packaged app.

| Time | What the player sees and does |
|---|---|
| 0:00 | Branded boot card (1.5 s), then a Title screen over a slowly panning forest. Continue / New forest / Settings / Credits / Quit, plus a disabled "Wishlist (coming soon)" pill. |
| 0:15 | New forest: the player names the colony ("Mosslight" by default). A 3-second camera swoop drops from canopy height into a lit, shadowed clearing ringed by dense forest, with hills and a river beyond the fence. |
| 0:20 to 1:00 | First-minute hook: the swoop ends framing the colony Leader at work. The objective card says "Welcome to Mosslight. Hazel is cutting wood. Click her to help." The click gives a paw burst, a "+5 s" float and a meow. Thirty named, coat-varied cats are visibly hauling, napping in the sun, grooming and drinking at the pond. |
| 1:00 to 3:00 | The first research point arrives at about 1 minute (the Research Hut is auto-staffed). The attention seal lists 2 or 3 useful things to do. "Basic Tools" is affordable at about 2 minutes and is bought with a payoff toast and chime. |
| 3:00 to 7:00 | The player opens the build tray, places a Den on a green ghost footprint and watches staked ground become a frame and then a finished cottage (dust puff, chime). |
| 8:00 to 12:00 | A wanderer arrives at the gate. The first officer is appointed and takes over hauling. Food and water pills show real (≈ estimated) numbers that rise and fall. The objective card suggests a short direct-control walk with a favourite cat. |
| 12:00 to 14:00 | A pregnancy is announced. Dusk falls (one game day is 24 real minutes). |
| 13:00 to 17:00 | "Raiders spotted to the south" with a horn at about 13 minutes; a small fox band hits the south gate at about 15 minutes. The player rallies defenders and wins; a victory jingle plays. |
| 17:00 to 24:00 | A kitten is born (hearts, a named kitten at half size). The player trains a guard and sends an expedition into Root Hollow floor 1: lantern-lit underground, a readable fight, a chest. |
| 24:00 to 32:00 | The expedition comes home and unloads real loot. With the raid repelled and the floor-1 loot unloaded, the end card appears: run stats, a colony capture, a primary "Wishlist" button, "Keep playing this colony" and "Return to title". |
| Keep playing | Root Hollow floor 2, the rest of the demo studies, more kittens and wanderers. The end card also triggers when the last demo study is bought, and as a soft fallback after 60 minutes of play (D23). |

**In scope.** Single-player, offline, macOS Apple Silicon. Title, pause, settings, save slots
and recovery. The Glass Clearing HUD (village chip, resource pills, attention seal and popover,
dock, hint line, tethered card, drawers, build tray, reins bar). A curated research slice of
about 20 studies. Officers for the founding stations. Housing, construction, hauling and needs.
One scripted raid band. Root Hollow floor 1 (the ending) and floor 2 (keep playing). Direct
control with a hero-cat close-up look. Audio (ambience, real CC0 music, UI, cats, work).
A simple offline catch-up with a "While you were away" card. An end card with a wishlist call
to action. A trailer, ten store screenshots and store page copy produced from the build.

**Out of scope (hidden in the demo build, simulation kept intact).** The shared-world server
and connect UI, founding a personal village, elections and petitions, inter-village trade,
the merchant, Routes, rail and shipping, Deepstone Vault and deeper dungeon sites, studies
outside the curated slice (shown as "Full game" teasers), functionless item recipes, Steam
Deck, Steamworks overlay integration, notarized distribution. A Windows build is conditional
(W4-08, blocked until the owner installs Windows Build Support).

## Product decisions

Work proceeds on the default shown here; the owner confirms D1 and D6 before Wave 2
starts, and may change any other default.

| # | Question | Default applied | Why |
|---|---|---|---|
| D1 | Positioning text says "non-commercial", which conflicts with a Steam wishlist. | "A forest colony game in development for Steam; the demo is free; pricing undecided." Applied in W0-03. | Agents optimise for the stated goal; pricing can be decided later without rework. |
| D2 | Does the demo start in the communal Grand Commons or a personal 15-cat village? | The 30-cat founding colony, named in the New forest dialog and presented as the player's own colony; every shared-world surface hidden. | It already has a research hut and more life on screen; the personal blueprint lacks key buildings. |
| D3 | How are day-scale timers made visible in a demo? | A `Pacing` profile per world: `classic` (today's numbers; tests and the server) and `demo` (every new single-player world), split into per-domain partial files (section 3.2). | Keeps 700+ mechanics scenarios valid while the demo gets compressed timers, without ownership clashes. |
| D4 | What does the clock show? | `Day n · hh:mm` from `TimeSeconds × DayScale`; demo `DayScale = 60` (one game day = 24 real minutes at 1x), starting Day 1 06:00. A basic sun cycle follows the clock from W2-17; W4-01 adds the full day/night look. | Minutes tick visibly, and night behaviour never appears under a noon sun. |
| D5 | Is `Catalog.Generated.cs` still generated from frozen Rust? | No. It becomes authored C# owned by Simulation; `tools/catalog-export` stays as a provenance diff tool only. Demo number changes live in `DemoContent` overrides, not in classic catalog values. | Every balance fix needs to edit content; classic scenarios stay valid. |
| D6 | Demo length and end trigger. | 25 to 35 minutes. End card when the first raid is repelled and the first Root Hollow floor-1 loot is unloaded, or when the last demo study is bought. Root Hollow floor 2 and the remaining studies are "Keep playing" content on the same save. | Benchmarked demos of 15 to 30 minutes that stop at a peak convert as well or better; it avoids ending on the least proven system. |
| D7 | Built-in pipeline or URP? | Stay on BiRP: real shadows, trilight ambient, fog, custom foliage, water, terrain and cat fur shaders and a small custom post pass. | URP needs a shader port and a 196-FBX reimport; the big wins work on BiRP. |
| D8 | Platforms. | macOS Apple Silicon. A Windows Mono build is package W4-08, blocked until the owner installs Windows Build Support for 6000.6.0f1. No Mac-only code anywhere (rule 2.6). | Steam is mostly Windows; the module is not installed today. |
| D9 | External assets and music. | SIL OFL fonts (Manrope, Instrument Serif) and CC0 audio from named sources (OpenGameArt CC0, Freesound CC0, Kenney CC0), recorded per file with URL, author, licence and SHA-256. Music is real CC0 tracks with an owner listening gate; procedural synthesis only for UI clicks and noise beds. No purchases, no paid or AI generation. | Licence-clean, free and Steam-safe, and procedural music would not sell a cozy game. |
| D10 | Steam App ID and wishlist link. | Ship the wishlist plumbing with a disabled "Wishlist (coming soon)" state behind `ShellLinks.StoreUrl`; no Steamworks.NET. | No App ID exists. |
| D11 | Developer ID signing and notarization. | Not scripted in this push. Tester builds are ad-hoc signed and opened with right-click Open (documented). | Needs a paid Apple membership; scripting it now is dead code. |
| D12 | OpenRouter API key committed in public history (`scripts/generate_isometric_assets.py`). | W0-03 deletes `scripts/`; history is not rewritten; the owner rotates the key in the OpenRouter dashboard; CI's gitleaks job ignores that one historical finding by fingerprint only. | Deletion does not revoke the key; rewriting published history is the owner's call. |
| D13 | Offline progress. | Simple capped catch-up: up to 8 hours, the real `World.Step` in 60 s chunks on a worker thread, then a "While you were away" card. | Idle-game promise at low cost; demo players rarely return after hours. |
| D14 | Name clash with an existing paid "IDLE CAT FOREST". | Keep the working title in the demo build; do not change the bundle id or save path until the owner picks the final name. | Renaming moves saves; do it once. |
| D15 | First raid. | Scripted, telegraphed fox band of 3: warning at about 13 minutes, attack at about 15 minutes, beatable by rallying adults; losing costs stored food, never a cat. | Tension without punishing a stranger, before the ending. |
| D16 | Deaths in the demo. | No starvation or raid deaths in the demo profile (needs floor at critical, raid damage non-lethal); dungeon death only on an explicitly accepted "Deadly" expedition. | Strangers should not lose a named cat to a tuning slip. |
| D17 | Merchant, trade and routes. | Hidden in the demo; simulation untouched; their drawers are not edited. | They cannot reach the bar in this push. |
| D18 | Speed set. | Pause, 1x, 2x and 4x for players (Space, 1, 2, 3); 8x and 16x only in debug builds or with `--forest-dev-speeds` for QA. | A 25 to 35 minute demo should not be skipped at 16x; QA still needs fast runs. |
| D19 | Distribution channel for strangers. | Prepare for a Steam demo (store copy, capsules, screenshots, trailer, wishlist plumbing) and hand testers ad-hoc builds until the owner picks Steam (App ID, Next Fest timing) or itch.io first. | Without a channel strangers cannot get the build; the assets serve either path. |
| D20 | Performance target. | About 35 cats at 60 fps on a base Apple Silicon machine at native Retina fullscreen on the default preset; dev-machine proxy in rule 2.8; owner confirms on base hardware. | That is the hardware and colony size strangers will have. |
| D21 | Owner involvement. | Owner playtests after W2-INT-c and W3-INT with five questions; music listening gate; decisions confirmed before Wave 2. | The bar cannot be judged by agents alone. |
| D22 | CI scope. | Keep `.github/workflows/sonarqube.yml` and the Sonar configuration exactly as the owner set them up. Add one GitHub CI workflow for the .NET build, scenarios, server tests and gitleaks. Leave `.forgejo/` untouched. | The owner configured Sonar in the last three commits; players never see CI. |
| D23 | Soft fallback for the end card. | Show the end card once after 60 minutes of play if neither D6 trigger fired; "Keep playing" continues. | Every player reaches the wishlist ask. |

## Owner actions

Only the owner can do these. Agents never do them on the owner's behalf.

1. **Rotate the leaked OpenRouter key (D12).** An OpenRouter API key was committed in
   `scripts/generate_isometric_assets.py` (added in `ce4789e`, removed from the file in
   `4aff7f3`). The repository is public, so the key has been readable by anyone. Wave 0
   deleted `scripts/`, but that does not revoke the key: it is still in git history, and
   history is not rewritten. Revoke the key in the OpenRouter dashboard, create a new one
   if it is still needed and check the old key's usage history. Rewriting history stays the
   owner's call. Never paste the key anywhere.
2. **Install Windows Build Support (D8)** for Unity 6000.6.0f1 if a Windows build is wanted.
   Package W4-08 stays blocked until it is installed.
3. **Confirm 60 fps on a base Apple Silicon machine (D20)**, an M1 or M2 with 8 GB, with
   about 35 cats on the default preset at native Retina fullscreen. Asked at W4-INT.
4. **Choose the distribution channel (D19):** Steam (App ID, Next Fest timing) or itch.io
   first. Testers get ad-hoc builds until then.
5. **Pick the final name (D14).** The bundle id and save path change only once, after the
   name is chosen.
6. **Playtests and the music listening gate (D21).** Play the build after W2-INT-c and after
   W3-INT and answer the five questions in
   `artifacts/demo/wave-<n>/owner-playtest.md`; approve the CC0 music at the W3-17 listening
   gate before W4-INT.
7. **Confirm D1 (positioning) and D6 (demo length and end)** before Wave 2 starts.

## Status by wave

Integrators update this table when their wave lands (rule 2.12): the status, the local
tag and the path of the wave report under `artifacts/demo/`.

| Wave | Purpose | Packages | Status | Tag | Report |
|---|---|---|---|---|---|
| W0 | Guardrails: compile gate, worktrees, Unity slots, captures, CI, demo brief, seams | 5 (1 integration) | Done (2026-10-02; 4 of 4 merged) | `demo-w0` | `artifacts/demo/wave-0/report.md` |
| W1 | Behaviour-neutral splits and every frozen contract | 9 (1 integration) | Done (2026-10-02; 8 of 8 merged) | `demo-w1` | `artifacts/demo/wave-1/report.md` |
| W2 | Core rebuild in parallel; staged integration A, B, C | 29 (3 integration) | In progress: W2-INT-a done (2026-10-03; W2-01 to W2-10 merged, simulation wired, demo timing green); W2-INT-b done (2026-10-03; W2-11 to W2-16 merged: persistence, game loop, Glass chrome, tethered card, copy layer, widgets; copy coverage and UI PlayMode green); W2-INT-c pending | `demo-w2a`, `demo-w2b`, `demo-w2` | `artifacts/demo/wave-2/report.md` |
| W3 | Features: milestones, shell, settings, demo boundary, build mode, drawers, direct control, animation, art hookup, FX, markers, audio, art | 22 (1 integration) | Not started | `demo-w3` | `artifacts/demo/wave-3/report.md` |
| W4 | Polish and ship-readiness, Windows (conditional), playtest punch lists | 12 (1 integration) | Not started | `demo-w4` | `artifacts/demo/wave-4/report.md` |
| W5 | Docs, store page, trailer, release candidate | 4 (1 integration) | Not started | `demo-w5` | `artifacts/demo/wave-5/report.md` |

## Appendix: Frozen contracts

Copied from section 3 of the wave plan; integrators keep it current. **Frozen at W1-INT
(2026-10-02):** sections 3.1 to 3.7 are re-synced with the merged code, the integrator's rulings
on Wave 1 deviations are in 3.4, and section 3.8 lists every constant, knob, intent, hook, element
name and scenario prefix as generated from the code.

These names exist after Wave 1 so that no Wave 2 to Wave 5 package needs a file it does not own.
W0-04 creates the cross-cutting seams, W1 packages create the rest, W0-03 copies this section
into the appendix of `docs/DEMO.md`, and W1-INT re-syncs that appendix with the real code
(including the complete constant lists). Packages may add members; they must not rename or
remove a frozen member. If a frozen member cannot work as written, say so in the handoff file;
the integrator decides.

Path shorthand used only in this section: `Sim/` = `unity/Assets/Forest/Simulation/`,
`Pres/` = `unity/Assets/Forest/Presentation/`, `Auth/` = `unity/Assets/Forest/Authority/`.

### 3.1 Simulation seams (W0-04)

- `Sim/Pacing/Pacing.cs`: `public sealed partial class Pacing` with `public string Profile`,
  `public double DayScale = 1`, `public static readonly Pacing Classic`, `public static readonly
  Pacing Demo`, `public static Pacing For(string profile)` (Demo for `"demo"`, else Classic).
  `Demo` is built by `MakeDemo()`, which creates a classic instance and calls one
  `partial void Demo<Domain>()` per domain file: `DemoCore`, `DemoLife`, `DemoPopulation`,
  `DemoResearch`, `DemoJobs`, `DemoWorld`, `DemoThreats`, `DemoTrade`. Each domain file
  declares its knobs as public fields whose initialisers are the classic values.
- `Sim/Model/World.Pacing.cs`: serialized `public string PacingProfile = "classic";`,
  `internal Pacing Pace => Pacing.For(PacingProfile);` and `public Pacing CurrentPacing()`.
  `Pace` must never appear in save JSON.
- `Sim/Recipes.cs`: `public Recipe RecipeOf(string id)` on `partial class World`. Every
  simulation lookup of a recipe's seconds, inputs or outputs goes through it. Until W2-10 it
  returns `Catalog.Recipe(id)` unchanged; W2-10 returns demo-profile overrides from
  `DemoContent`.
- `Sim/Skills.cs`: `public void GainSkill(Cat c, string skill, double amount)` on
  `partial class World` (today: `Add(c.Skills, skill, amount)`). Every skill gain goes through
  it; W3-02 adds front-loaded growth and rank-up events here.
- `Pres/Contracts/Subject.cs`: `public enum SubjectKind { None, Cat, Building, Pile,
  Expedition, Site, Entrance }` and `public readonly struct Subject { SubjectKind Kind; string
  Id; static Subject None; }`.
- `Pres/Contracts/GameSpeeds.cs`: `Paused = 0`, `Player = { 1, 2, 4 }`, `Dev = { 8, 16 }`
  (only when `Debug.isDebugBuild` or `--forest-dev-speeds`), `float Clamp(float requested)`
  snapping to the nearest allowed value. Hotkeys: Space pauses or resumes, 1/2/3 select 1x,
  2x, 4x (D18).

### 3.2 Pacing layout and owners

Files live in `Sim/Pacing/`. Classic values equal today's literals exactly; knobs that W2
needs are pre-declared in W1 with the values W2 will use (unused in W1, so W1 stays
behaviour-neutral).

| File | Knobs (W1 list; complete list frozen in DEMO.md by W1-INT) | W0/W1 creator | W2 owner | W4 owner |
|---|---|---|---|---|
| `Pacing.cs` | Profile, DayScale, MakeDemo, For | W0-04 | W2-01 | W4-03 |
| `Pacing.Life.cs` | need decay rates and thresholds, sleep recovery, need dwell min/max (4/6 s), metabolism min/max (0.85/1.15), threshold jitter (10), need-trip backoff base (3 s), idle dwell min/max (8/40 s), night sleep bias, `NeedsCanKill` (classic true) | W1-02 | W2-01 | W4-03 |
| `Pacing.Population.cs` | breeding window and chance, gestation, migration first-at and interval, probation, aging | W1-02 | W2-01 | W4-03 |
| `Pacing.Research.cs` | research divisor (600), leader study interval, election interval | W1-02 | W2-01 | W4-03 |
| `Pacing.Threats.cs` | raid triggers, `DemoRaidAtSeconds` (0 = off), `RaidWarningSeconds` (120), `RaidCanKill` (classic true), dungeon threat constants, `FloorRestockSeconds` (600) | W1-02 | W2-01 | W4-03 |
| `Pacing.Trade.cs` | trader first-at, interval and window | W1-02 | W2-01 | W4-03 |
| `Pacing.Jobs.cs` | job durations, hunt/fish/gather yields, regrowth and replant timings, fish regeneration, scout and build progress, carry factors, `JobWatchdogSeconds` (25), `SourceExclusionSeconds` (60), `BuildersPerSite` (2) | W1-01 | W2-01 | W4-03 |
| `Pacing.World.cs` | cat movement speeds and terrain factors | W1-03 | W2-01 | W4-03 |

In Wave 2 only W2-01 edits `Sim/Pacing/**`. Another W2 package that needs a new knob uses a
`private const` in its own file marked `// PACING-HANDOFF <name>` and lists it in its handoff
file; W2-INT-a moves it into the right partial.

### 3.3 Identifier constants

`const string` values byte-identical to today's literals. W1 packages replace literals only in
their own files. The complete lists, plus the codes pre-declared for Wave 2, are frozen in
`docs/DEMO.md` by W1-INT; W2-15 writes player text against that list.

| File | Class | Created by | W2 owner |
|---|---|---|---|
| `Sim/Ids/Ids.Jobs.cs` | `JobKind`, `JobPhase` | W1-01 | W2-04 |
| `Sim/Ids/Ids.Blocked.cs` | `Blocked` (pre-declares `worker_required`, `missing_scaffold_input`, `missing_material`, `route_blocked`, `source_excluded`) | W1-01 | W2-05 |
| `Sim/Ids/Ids.Goals.cs` | `GoalKey` (pre-declares `idle_nap`, `idle_groom`, `idle_social`, `idle_patrol`, `idle_shrine`, `idle_play`) | W1-02 | W2-03 |
| `Sim/Ids/Ids.Codes.cs` | `ResultCode` (pre-declares `ok`, `queued`, `planned`, `assigned`, `released`, `studied`, `boosted`, `cancelled`, `demolished`, `not_a_workplace`, `unknown_target`, `requires_players`, `full_game_only`, `missing_material`, `all_grounds_busy`, `not_implemented`) | W1-03 | W2-06 |
| `Sim/Ids/Ids.Events.cs` | `EventKind` (pre-declares `milestone`, `raid_warning`, `raid_repelled`, `building_completed`, `discovery`, `storage_full`, `storage_empty`, `rank_up`, `birth`, `arrival`, `death`, `expedition_loot`, `error`) | W1-03 | W2-08 |

### 3.4 Stub contracts created in Wave 1

| Contract | File | Created by | Implemented by |
|---|---|---|---|
| `Milestones.Current(World, Village)` returning `MilestoneStatus { Id, Index, Count, Progress, HintCode, Complete }` (immutable: readonly fields and a constructor; `MilestoneStatus.None`); `Milestones.DemoComplete(World, Village)` | `Sim/Milestones.cs` | W1-02 (returns none / false) | W3-01 |
| `Attention.Compute(World, Village)` returning `IReadOnlyList<AttentionItem>`; `AttentionItem { Id, Severity, TitleCode, CauseCode, Section, Anchor, ActionKind, Args }`; `Attention.CountBySection(World, Village)` | `Sim/Attention.cs` | W1-02 (empty) | W2-16, then W3-06 |
| `Die(Village, Cat, string cause)`: the existing private `World` method (parameter renamed to `cause`; it already writes the cause into the death note, so no separate overload exists) | `Sim/Systems/Population.cs` | W1-02 | W3-02 |
| `CatActivity` and the pure `World.ActivityOf(Village, Cat)` (unused by game code in W1) | `Sim/Activity.cs` | W1-02 | W2-03 |
| `DayClock`: static `GameMinutes(World)`, `Day(World)`, `DayFraction(World)`, `IsNight(World)`; Day 1 starts at 06:00, scaled by `DayScale`; never saved | `Sim/DayClock.cs` | W1-02 | W2-01, W2-17 |
| `World.RandomInt(int n)` (high bits, one draw, throws for n <= 0) | `Sim/World/World.cs` | W1-03 | all W2 simulation packages |
| `ActionResult.Code` (default `""`), `ActionResult.Args` (null by default), `ActionResult.Fail(string code, string message, params (string key, string value)[] args)`, `ActionResult.Ok(string code, string entityId, params (string key, string value)[] args)`; the legacy `Ok`/`Fail` keep an empty code | `Sim/Model/ActionResult.Codes.cs` | W1-03 | W2-06 |
| `Event.Seq` and `World.NextEventSeq`, assigned in `Note`; old saves continue above the highest `Seq` found | `Sim/Model/Event.Seq.cs` | W1-03 | W2-08 |
| `DemoScope`: `SharedWorldEnabled`, `IsSectionVisible(string)`, `IsStudyVisible(string)`, `IsBuildingVisible(string)`, `MaxDungeonDepth`, `Check(GameAction)` | `Auth/DemoScope.cs` | W1-05 (all visible, Check ok) | W3-06 |
| `ShellLinks`: `StoreUrl` (null), `WishlistAvailable` (false), `OpenStore()`, `FeedbackUrl` | `Pres/Shell/ShellLinks.cs` | W1-05 | W3-06 |
| `GameSettings`: static `UiScale` (1), `LargerText`, `ReducedMotion`, `Volume(string bus)` (1), `MuteInBackground`, `Quality`, `event Action Changed` | `Pres/Shell/GameSettings.cs` | W1-05 (defaults, no persistence) | W3-05 |
| `UiEvents`: static events `Clicked`, `Confirmed`, `Failed`, `Opened`, `Closed`, `Attention`, `Flourish(string kind)` | `Pres/UI/UiEvents.cs` | W1-05 (declared, raised nowhere) | W2-16 |
| `ForestGame.Faulted` (false), `ForestGame.Changed` event, `ForestGame.PersistenceTick(double steppedSeconds)` called once per frame from `Update` | `Pres/Game/ForestGame.cs`, `Pres/Game/ForestGame.Persistence.cs` | W1-07 | W2-12 (Faulted), W2-11 (PersistenceTick) |
| `LocalAuthority.RestoreLastGood()` returning bool (false) | `Auth/LocalAuthority.cs` | W1-07 | W2-11 |
| `ForestGame.PersistenceTick` is `private void PersistenceTick(double steppedSeconds)`, called once per frame after the telemetry frame and `StepLocal` (0 in remote play) and before `Changed` is raised; `Changed` fires at most once per frame when `Revision` or the remote frame sequence changed | `Pres/Game/ForestGame.Persistence.cs` | W1-07 | W2-11, W2-12 |
| `DemoScope.Check(GameAction)` returns an `ActionResult` (Success true today), so `LocalAuthority.Apply` can return a refusal directly | `Auth/DemoScope.cs` | W1-05 | W3-06 |
| `CatMotion.PawContact` static event `(string catId, Vector3 position)` | `Pres/View/CatMotion.cs` | W1-06 (never raised) | W3-13 raises, W3-17 subscribes |
| `Visual` child: every entity root has one child named `Visual` holding the model; `ForestView.VisualOf(GameObject)`; reconcile writes only the root | `Pres/View/ForestView.Entities.cs` | W1-06 | used by W3-14, W3-15, W4-07 |
| Carried cargo rendering in its own partial (`ReconcileCarriedCargo`, `ItemCargoAsset`) | `Pres/View/ForestView.Cargo.cs` | W1-06 | W3-13 (sockets) |
| Accessors: `TryGetEntity`, `CatTransforms()`, `CameraStandoff`, `Zoom`, `CameraYaw`, `RenderedLevel`, `CursorTile`, `HoveredSubject`, `ForceReconcile()` | `Pres/View/*.cs` | W1-06 | kept by every later owner |
| UI skeleton (ForestUI facade, `Select(Subject)`, mount slots, UiState, UiContext, ISection, SectionRegistry, DrawerHost, widgets, UiText final API, chrome, card and build-mode stubs) | `Pres/UI/**` | W1-05 | see W2 and W3 sections |

Mount slots in `ForestUI`: `slot-chip`, `slot-seal`, `slot-pills`, `slot-dock`, `slot-hint`,
`slot-drawer`, `slot-card`, `slot-toasts`, `slot-mode`, `slot-reins`, `slot-objective`,
`slot-modal`.

**W1-INT rulings on Wave 1 deviations.** Accepted as built: `Die` is the existing method rather
than a new overload; `TickIdle(Village, Cat, double dt, bool planning)` takes the extra `planning`
argument; `DayClock` is a static class so the clock never enters save JSON; the stub types are
immutable; `NightSleepBias` is pre-declared as 15; the coded success helper is
`ActionResult.Ok(code, entityId, args)` (the wire format has no success message, and W2-06 needs the
affected entity id); `PointerAction` gained `PrimaryPress` (today's selection timing). W1-02's
`WorkingAgeHours` duplicated W1-01's `WorkerMinAgeHours` (both 12); W1-INT removed it and
`Leader.cs` and `Dungeons.cs` now read `WorkerMinAgeHours`. `Blocked.RouteBlocked`
(`route_blocked`, pre-declared) and `Blocked.BlockedRoute` (`blocked_route`, what movement sets
today) are distinct constants; W2-05 and W2-15 handle both.

### 3.5 Input router

One router decides every key and mouse-button meaning. Files: `Pres/Input/InputRouter.cs`,
`Pres/Input/InputIntent.cs`, `Pres/Input/KeyMap.cs`. Owner: W1-06 creates it, W2-18 owns it in
Wave 2, W3-12 owns it in Wave 3. Everyone else only calls its API from their own files.

- `InputIntent`: `Escape`, `ToggleControl` (Tab), `TogglePause` (Space), `Speed1`, `Speed2`,
  `Speed3` (1, 2, 3), `FocusSelected` (F), `Home` (H, Home), `LevelUp` (], Page Up),
  `LevelDown` ([, Page Down), `ZoomIn` (+, =, keypad +), `ZoomOut` (-, keypad -),
  `RotateLeft` (Q), `RotateRight` (E outside direct control), `Interact` (E in direct
  control), `RotatePlacement` (R), `PhotoMode` (P), `ToggleFullscreen` (Cmd+Enter).
- Continuous state each frame: `MoveAxis` (WASD and arrows), `PointerPosition`, `ScrollZoom`,
  `PanDelta` (drag or trackpad), `ShiftHeld`, `CommandHeld`, `PointerOverUi`,
  `TextFieldFocused` (set by the UI; while true only `Escape` is dispatched), `ControlMode`
  (set by the view; decides E).
- `InputPriority`: `Modal = 100` (shell dialogs, settings page, end card), `Popover = 90`,
  `Placement = 80`, `Control = 70` (leave direct control), `Drawer = 60`, `Selection = 50`
  (deselect), `World = 40` (camera, picking), `Fallback = 0` (open the pause menu).
- `IDisposable Bind(InputIntent intent, int priority, Func<bool> handler)`: handlers run from
  the highest priority down until one returns true; ties run in registration order.
- `IDisposable BindPointer(PointerAction action, int priority, Func<PointerInfo, bool> handler)`
  with `PointerAction { PrimaryClick, SecondaryClick, PrimaryDragStart, PrimaryDrag,
  PrimaryDragEnd, PrimaryPress }`. `PrimaryPress` fires on the press frame. A gesture that
  starts over UI is ignored, a drag is latched until release and a click released over UI is
  suppressed (W2-18 may tighten this). A primary press that moves more than 6 px
  (`KeyMap.DragThresholdPixels`) becomes a drag and produces no click.
- The Esc order is therefore: modal, popover, placement, leave control, close drawer,
  deselect, open pause.

### 3.6 View hooks

`Pres/View/ForestView.cs` declares these partial methods and calls them from `Awake`, `Update`,
`LateUpdate` and the level switch. Each domain implements them in its own partial file. Unused
hooks compile away.

| Hook domain | Methods | Implementing file | W2 | W3 | W4 |
|---|---|---|---|---|---|
| Lighting | `AwakeLighting()`, `UpdateLighting(float dt)`, `LevelChangedLighting(int level)` | `ForestView.Lighting.cs` | W2-17 | none | W4-01 |
| Terrain | `AwakeTerrain()`, `UpdateTerrain(float dt)`, `LevelChangedTerrain(int level)` | `ForestView.Terrain.cs` | W2-19 | none | W4-01 |
| Props | `AwakeProps()`, `UpdateProps(float dt)`, `LevelChangedProps(int level)` | `ForestView.Props.cs` | W2-20 | none | W4-01 |
| Fx | `AwakeFx()`, `UpdateFx(float dt)`, `LateUpdateFx()` | `ForestView.Fx.cs` | W2-21 | W3-15 | none |
| Markers | `AwakeMarkers()`, `LateUpdateMarkers()` | `ForestView.Markers.cs` | none | W3-16 | none |
| Atmosphere | `AwakeAtmosphere()`, `UpdateAtmosphere(float dt)`, `LateUpdateAtmosphere()` | `ForestView.Atmosphere.cs` | none | none | W4-01 |
| CloseUp | `AwakeCloseUp()`, `LateUpdateCloseUp()` | `ForestView.CloseUp.cs` | none | none | W4-02 |
| BuildingMotion | `UpdateBuildingMotion(float dt)` | `ForestView.BuildingMotion.cs` | none | none | W4-07 |

W1-06 also moves fog distance logic out of `UpdateCamera` into `ForestView.Lighting.cs`
(`ApplyFogForCamera(bool directControl)`), sun creation out of `Awake` into
`ForestView.Lighting.cs` (`CreateLights()`), and the terrain rebuild trigger out of tile
reconcile into `ForestView.Terrain.cs` (`TerrainTick()`, called from `Reconcile`).

### 3.7 Capture driver API dependencies

`Pres/Capture/CaptureDriver.cs` (W0-01) drives the player for `forest.sh capture` through
public members only: `ForestGame.Instance`, `ForestGame.SetSpeed`, `ForestGame.Act`,
`ForestGame.CurrentWorld` (including `World.TimeSeconds`), `ForestGame.Selected` (the selected village),
`ForestGame.UI`, `ForestView.FocusVillage`, `ForestView.ZoomBy`,
`ForestView.InspectCat`, `ForestView.InspectBuilding`, `ForestView.ViewLevel`,
`ForestUI.OpenPanel(string)`, `ForestUI.ClosePanel()`, `ForestUI.OpenSection` (read-only property), and the
model fields `Cat.Id`, `Cat.Name`, `Cat.Alive`, `Cat.JobId`, `Building.Id`, `Building.Kind`. The driver finds the
view with `GetComponent<ForestView>()` on the `ForestGame` object. These signatures are frozen until W5-03 owns
the capture folder. `--forest-save <path>` must keep opening that world directly (no title
screen) for tooling, also after W3-04.

### 3.8 Frozen lists (frozen at W1-INT)

Generated from the merged code on 2026-10-02. Packages may add members; they must not rename or
remove one listed here. Values are byte-identical to the literals they replaced.

#### Identifier constants

**`JobKind`** (`Sim/Ids/Ids.Jobs.cs`, 37): `Hunt` = `hunt`, `Water` = `water`, `Logs` = `logs`, `Quarry` = `quarry`, `Fibre` = `fibre`, `Fish` = `fish`, `Scout` = `scout`, `Replant` = `replant`, `Train` = `train`, `Expand` = `expand`, `Haul` = `haul`, `Build` = `build`, `Production` = `production`, `Offering` = `offering`, `MouseFarm` = `mouse_farm`, `Road` = `road`, `Rail` = `rail`, `Bridge` = `bridge`, `Dock` = `dock`, `Wagon` = `wagon`, `Vessel` = `vessel`, `SupplyFood` = `supply_food`, `LeaderPlanHunt` = `leader_plan_hunt`, `HuntExpedition` = `hunt_expedition`, `SupplyWater` = `supply_water`, `FetchWater` = `fetch_water`, `GatherLogs` = `gather_logs`, `ForageFibre` = `forage_fibre`, `ReplantTree` = `replant_tree`, `Explore` = `explore`, `TrainWarrior` = `train_warrior`, `ExpandVillage` = `expand_village`, `LeaderPlanHouse` = `leader_plan_house`, `BuildHouse` = `build_house`, `Ritual` = `ritual`, `CarryOffering` = `carry_offering`, `PerformOffering` = `perform_offering`
**`JobPhase`** (`Sim/Ids/Ids.Jobs.cs`, 7): `Travel` = `travel`, `Fetch` = `fetch`, `ItemFetch` = `item_fetch`, `InputDelivery` = `input_delivery`, `Working` = `working`, `OutputDelivery` = `output_delivery`, `Return` = `return`
**`Blocked`** (`Sim/Ids/Ids.Blocked.cs`, 19): `ForeignTerritory` = `foreign_territory`, `ExpansionFootprintBlocked` = `expansion_footprint_blocked`, `InfrastructureFootprintBlocked` = `infrastructure_footprint_blocked`, `BuildingEntranceDisconnected` = `building_entrance_disconnected`, `QueuePaused` = `queue_paused`, `MissingScaffoldInput` = `missing_scaffold_input`, `UnknownRecipe` = `unknown_recipe`, `MissingRecipeInput` = `missing_recipe_input`, `WallMaterialsMissing` = `wall_materials_missing`, `PavingInputMissing` = `paving_input_missing`, `OutputStorageFullOrUnreachable` = `output_storage_full_or_unreachable`, `WorkerRequired` = `worker_required`, `SourceChanged` = `source_changed`, `BuildingResearchRequired` = `building_research_required`, `SiteRequired` = `site_required`, `BlockedRoute` = `blocked_route`, `MissingMaterial` = `missing_material`, `RouteBlocked` = `route_blocked`, `SourceExcluded` = `source_excluded`
**`GoalKey`** (`Sim/Ids/Ids.Goals.cs`, 33): `Idle` = `idle`, `RestingAtHome` = `resting at home`, `Leaving` = `leaving`, `Research` = `research`, `Defending` = `defending`, `PlayerControl` = `player_control`, `PlayerRest` = `player_rest`, `ProductionPaused` = `production paused`, `NeedPrefix` = `need_`, `NeedDrink` = `need_drink`, `NeedEat` = `need_eat`, `NeedHeal` = `need_heal`, `NeedSleep` = `need_sleep`, `Separator` = ` · `, `FarmPrefix` = `farm · `, `AccountingPrefix` = `accounting · `, `AccountingReturning` = `accounting · returning to tent`, `ExploringDungeon` = `exploring dungeon`, `ReturningFromDungeon` = `returning from dungeon`, `DescendingDungeonStairs` = `descending dungeon stairs`, `FightingDungeonGuardian` = `fighting dungeon guardian`, `CollectingDungeonSupplies` = `collecting dungeon supplies`, `ReturningToPrefix` = `returning to `, `ReturningRailVehicle` = `returning rail vehicle to its reached tile`, `ReturningVesselToDock` = `returning vessel to dock`, `PlayerControlReturningToDock` = `player_control · returning to dock`, `TransportReturning` = `transport returning`, `IdleNap` = `idle_nap`, `IdleGroom` = `idle_groom`, `IdleSocial` = `idle_social`, `IdlePatrol` = `idle_patrol`, `IdleShrine` = `idle_shrine`, `IdlePlay` = `idle_play`
**`ResultCode`** (`Sim/Ids/Ids.Codes.cs`, 16): `Ok` = `ok`, `Queued` = `queued`, `Planned` = `planned`, `Assigned` = `assigned`, `Released` = `released`, `Studied` = `studied`, `Boosted` = `boosted`, `Cancelled` = `cancelled`, `Demolished` = `demolished`, `NotAWorkplace` = `not_a_workplace`, `UnknownTarget` = `unknown_target`, `RequiresPlayers` = `requires_players`, `FullGameOnly` = `full_game_only`, `MissingMaterial` = `missing_material`, `AllGroundsBusy` = `all_grounds_busy`, `NotImplemented` = `not_implemented`
**`EventKind`** (`Sim/Ids/Ids.Events.cs`, 31): `Research` = `research`, `Tithe` = `tithe`, `Completed` = `completed`, `Defense` = `defense`, `Dungeon` = `dungeon`, `Election` = `election`, `Emergency` = `emergency`, `Expansion` = `expansion`, `Migration` = `migration`, `Pregnancy` = `pregnancy`, `Raid` = `raid`, `RaidBreach` = `raid_breach`, `RaidLoss` = `raid_loss`, `Recovery` = `recovery`, `RecoveryBlocked` = `recovery_blocked`, `Trade` = `trade`, `Trader` = `trader`, `Upgrade` = `upgrade`, `Milestone` = `milestone`, `RaidWarning` = `raid_warning`, `RaidRepelled` = `raid_repelled`, `BuildingCompleted` = `building_completed`, `Discovery` = `discovery`, `StorageFull` = `storage_full`, `StorageEmpty` = `storage_empty`, `RankUp` = `rank_up`, `Birth` = `birth`, `Arrival` = `arrival`, `Death` = `death`, `ExpeditionLoot` = `expedition_loot`, `Error` = `error`

#### Pacing knobs (classic values; every `Demo<Domain>()` hook is empty at W1)
- **`Pacing.Jobs.cs`** (57; hook `DemoJobs()`): `WorkerMinAgeHours` 12, `AssignPreferenceScore` 100, `AssignBoostScore` 20, `HuntWork` 120, `GatherWork` 60, `TrainWork` 120, `ScoutWork` 15, `ExpansionWork` 10, `OfferingWork` 30, `OfferingHerbs` 5, `AutoDenSearchRadius` 5, `ScoutSurveyRingStep` 2, `ScoutSurveyMaxRadius` 10, `ScoutBaseRadius` 12, `ScoutKnownTilesPerRadius` 100, `ScoutRadiusJitter` 8, `ScoutSightRadius` 2, `ScoutMaxSeconds` 180, `HaulBatchMax` 8, `HaulSkillPerDelivery` 0.25, `SpillPileCapacity` 1000000, `ItemSpillPileCapacity` 10000, `JobPlanningIntervalSeconds` 1, `StorageRetrySeconds` 1, `LaborSkillPerJob` 1, `RoleExperiencePerJob` 1, `SpecializationRoleExperience` 25, `ToolWearPerJob` 0.1, `WallSegmentWork` 10, `PavingSegmentWork` 30, `ScaffoldTimberBase` 4, `ScaffoldTimberPerExisting` 2, `ScaffoldBlocksBase` 2, `ScaffoldBlocksPerExisting` 1, `ScaffoldMinWork` 1, `ItemBaseCondition` 100, `PreservationDurabilityMult` 1.25, `OfferingHerbsPerBlessing` 5, `MouseFarmFoodPerCycle` 4, `TrainFightSkill` 3, `ReplantRegrowSeconds` 86400, `HuntMasteryYieldPerLevel` 0.12, `FishCatchMax` 8, `HuntHidePerFood` 0.25, `HuntBonePerFood` 0.125, `HuntRegrowSeconds` 3600, `WorkSkillDivisor` 300, `WorkSkillBonusCap` 0.333, `WorkStatBase` 0.75, `WorkStatDivisor` 200, `WorkCycleTimeFloor` 0.1, `WorkToolBonus` 1.15, `WorkRateMin` 0.1, `WorkRateMax` 20, `JobWatchdogSeconds` 25, `SourceExclusionSeconds` 60, `BuildersPerSite` 2
- **`Pacing.Life.cs`** (74; hook `DemoLife()`): `HungerDecayPerHour` 20.0, `ThirstDecayPerHour` 30.0, `RestDecayPerHour` 12.0, `ResiliencePerLevel` 0.08, `ResilienceFloor` 0.45, `HealthRegenPerSecond` 0.002, `HealthRegenMinNeed` 50, `StarvationDamagePerMinute` 2.0, `DehydrationDamagePerMinute` 3.0, `NeedsCanKill` true, `DrinkThreshold` 35, `EatThreshold` 35, `HealThreshold` 40, `SleepThreshold` 20, `CargoKeepThirst` 15, `CargoKeepHunger` 15, `CargoKeepHealth` 20, `NeedRetrySeconds` 1, `NeedServing` 1, `BrewServing` 0.25, `WaterServingAfterBrew` 0.75, `DrinkRestorePerServing` 50, `EatRestorePerServing` 50, `HealRestorePerServing` 25, `SleepRecoveryPerSecond` 0.15, `SleepWakeRest` 95, `NeedDwellMin` 4, `NeedDwellMax` 6, `MetabolismMin` 0.85, `MetabolismMax` 1.15, `NeedThresholdJitter` 10, `NeedTripBackoffSeconds` 3, `IdleDwellMin` 8, `IdleDwellMax` 40, `NightSleepBias` 15, `IdleWanderIntervalSeconds` 20, `IdleWanderRadius` 4, `LeaderDirectSeconds` 10, `UpkeepTickSeconds` 60, `CompletedJobRetentionSeconds` 600, `RegrowthCheckSeconds` 60, `RegrownLogs` 40, `RegrownForage` 30, `CrewGroupSize` 15, `HuntCrewPerGroup` 6.0, `WaterCrewPerGroup` 2.0, `FoodReservePerCat` 6, `WaterReservePerCat` 8, `EmergencyStockPerCat` 1, `ScoutIntervalSeconds` 600, `ForesterStockTarget` 40, `ReplantIntervalSeconds` 600, `FibreStockTarget` 10, `LoremasterOfferingBlessingCap` 10, `LoremasterOfferingMaterialsMin` 30, `LoremasterOfferingMaterials` 5, `LoremasterOfferingWork` 30, `OfficerReserveMin` 2, `OfficerReservePerGroup` 2.0, `OfficerItemsPerCat` 0.5, `OfficerFoodPerCat` 8, `OfficerStockTarget` 32, `MouseFarmGrain` 1, `MouseFarmWorkSeconds` 600, `ResearchSkillSecondsPerPoint` 3600, `FarmHarvestCarry` 8, `FarmHaulSkillGain` 0.25, `FarmSkillSecondsPerPoint` 3600, `AccountingWaitSeconds` 60, `AccountingCountSeconds` 5, `SpoilageRate` 0.0002, `PreservesSpoilageRate` 0.00002, `EcologyTickSeconds` 60, `FishRegenPerHour` 0.5
- **`Pacing.Population.cs`** (26; hook `DemoPopulation()`): `KittenUntilHours` 24, `ElderAgeHours` 240, `HonouredElderAgeHours` 288, `OldAgeDeathChance` 0.0002, `BreedingCheckSeconds` 3600, `BreedingAfterFoundingSeconds` 36 * 3600, `BreedingFoodPerCat` 2.5, `BreedingWaterPerCat` 2.5, `ParentMinAgeHours` 24, `ParentMaxAgeHours` 240, `BreedChanceBase` 0.001, `BreedBlessingStep` 0.02, `BreedBlessingCap` 0.5, `BreedBlessingWeight` 0.0007, `GestationSeconds` 18 * 3600, `KittenStatSpread` 16, `MigrationFirstAtSeconds` 30 * 3600, `MigrationIntervalSeconds` 12 * 3600, `MigrationFoodPerCat` 4, `MigrationWaterPerCat` 5, `MigrationPopulationCap` 200, `ProbationSeconds` 36 * 3600, `HousingPlanAfterSeconds` 6 * 3600, `HousingSpareBeds` 1, `HousingFoodPerCat` 3, `HousingWaterPerCat` 4
- **`Pacing.Research.cs`** (21; hook `DemoResearch()`): `ResearchDivisor` 600, `LeaderStudyIntervalSeconds` 86400, `ElectionIntervalSeconds` 86400, `ElectionDurationSeconds` 1800, `FamilySourcesInputFactor` 0.9, `FamilySourcesOutputFactor` 1.1, `FamilyBulkDurationFactor` 0.9, `UpgradeSupplySpeedStep` 0.1, `UpgradeSupplySpeedFloor` 0.55, `UpgradeHuntStep` 0.1, `UpgradeHuntFloor` 0.45, `UpgradeBuildStep` 0.1, `UpgradeBuildFloor` 0.45, `UpgradeRitualStep` 0.12, `UpgradeRitualFloor` 0.4, `BoostSeconds` 10, `BoostSecondsPerClickPower` 2, `BoostFreePerMinute` 30, `BoostDecay` 0.95, `BoostDecayClicks` 10.0, `BoostMinSeconds` 1
- **`Pacing.Threats.cs`** (44; hook `DemoThreats()`): `RaidCheckSeconds` 3600, `RaidGraceSeconds` 8 * 3600, `RaidMinCats` 20, `RaidMinWealth` 250, `RaidChance` 0.1, `DemoRaidAtSeconds` 0, `RaidWarningSeconds` 120, `RaidCanKill` true, `RaidSpeed` 0.5, `DefenderStrikePerSecond` 0.1, `RaiderStrikePerSecond` 0.03, `DefenderWeaponWearPerSecond` 0.005, `DefenderSkillSecondsPerPoint` 3600, `RaidLootShare` 0.3, `RaidLootMax` 20, `RaidBreachDamage` 20, `DungeonBaseThreat` 1.15, `DungeonBandThreat` 0.65, `DungeonFloorThreat` 0.85, `GuardianBaseHealth` 15, `GuardianFloorHealth` 9, `GuardianBandHealth` 8, `DungeonLootBase` 3, `DungeonLootSecondary` 2, `FloorRestockSeconds` 600, `ReadinessFightSkillFactor` 0.25, `ReadinessNeedNorm` 45, `ExpeditionMinHealth` 65, `ExpeditionMinNeed` 45, `ExpeditionRetreatHealth` 50, `ExpeditionRetreatNeed` 30, `ExpeditionGiveUpHealth` 40, `ExpeditionGiveUpNeed` 25, `ExpeditionDescendCargoLimit` 8, `DungeonRevealTiles` 4, `DungeonReplanSeconds` 1, `ExplorerStrikePerSecond` 1.8, `ExplorerSkillSecondsPerPoint` 300, `ExplorerWeaponWearPerSecond` 0.025, `CreatureStrikePerSecond` 0.45, `CreatureSpeed` 0.65, `CreatureLeashTiles` 12, `CreatureAggroTiles` 5, `CreatureReplanSeconds` 1
- **`Pacing.Trade.cs`** (15; hook `DemoTrade()`): `TraderFirstAtSeconds` 3600, `TraderIntervalSeconds` 12 * 3600, `TraderWindowSeconds` 900, `TraderReplanSeconds` 10, `TraderSpeed` 0.8, `TraderPurse` 1000, `TraderStockBase` 20, `TraderStockSpread` 40, `TraderBuyPrice` 2, `TraderSellPricePerQuality` 4, `CaravanSpeed` 0.8, `VehicleSpeed` 1, `TransportSkillSecondsPerPoint` 3600, `LinearWorkPerTile` 30, `InfrastructureWork` 120
- **`Pacing.World.cs`** (8; hook `DemoWorld()`): `CatBaseSpeed` 1.6, `RoadSpeedFactor` 1.75, `DirtSpeedFactor` 1.05, `MountainSpeedFactor` 0.4, `ForestSpeedFactor` 0.85, `OpenGroundSpeedFactor` 1, `WadingSpeedFactor` .55, `PathWearToDirt` 50
- **`Pacing.cs`** (2): `Profile` ClassicProfile, `DayScale` 1

Total at W1-INT: 247 public fields across the Pacing partials (293 after W2-INT-a; see 3.9).

#### Input router

- `InputIntent` (dispatch order within a frame): `Escape`, `ToggleControl`, `TogglePause`,
  `Speed1`, `Speed2`, `Speed3`, `FocusSelected`, `ZoomIn`, `ZoomOut`, `Home`, `LevelDown`,
  `LevelUp`, `RotateLeft`, `RotateRight`, `Interact`, `RotatePlacement`, `PhotoMode`,
  `ToggleFullscreen`.
- `InputPriority`: `Modal` 100, `Popover` 90, `Placement` 80, `Control` 70, `Drawer` 60,
  `Selection` 50, `World` 40, `Fallback` 0.
- `PointerAction`: `PrimaryClick`, `SecondaryClick`, `PrimaryDragStart`, `PrimaryDrag`,
  `PrimaryDragEnd`, `PrimaryPress`. `PointerInfo`: `Position`, `Delta`, `PressPosition`,
  `ShiftHeld`, `CommandHeld`.
- Router API: `InputRouter.Current`; `Bind(InputIntent, int, Func<bool>)` and
  `BindPointer(PointerAction, int, Func<PointerInfo, bool>)` return `IDisposable`; a throwing
  handler is logged and counts as handled; `Dispatch(InputIntent)`,
  `DispatchPointer(PointerAction, PointerInfo)`, `Tick()`.
- Continuous state (refreshed by `Tick`): `DevicesPresent`, `MoveAxis`, `MoveStepAxis`,
  `MovePressed`, `PointerPosition`, `PointerDelta`, `ScrollZoom`, `PanDelta`, `PrimaryHeld`,
  `SecondaryHeld`, `MiddleHeld`, `ShiftHeld`, `CommandHeld` (Cmd on macOS, Ctrl elsewhere). Set
  before `Tick`: `PointerOverUi`, `TextFieldFocused`, `ControlMode`. In W1 `ForestView.TickInput`
  bridges them from `ForestUI.PointerOverPanel`, `ForestUI.TextInputFocused` and direct control.

Key table (`KeyMap.Intents`; Live = dispatched today):

| Key | Intent | Context | Live |
|---|---|---|---|
| Escape | Escape | any | yes |
| Tab | ToggleControl | any | yes |
| Space | TogglePause | any | no |
| 1 / 2 / 3 | Speed1 / Speed2 / Speed3 | any | no |
| F | FocusSelected | any | no |
| = and keypad + | ZoomIn | any | yes |
| - and keypad - | ZoomOut | any | yes |
| Home | Home | any | yes |
| H | Home | any | no |
| Page Down | LevelDown | any | yes |
| [ | LevelDown | any | no |
| Page Up | LevelUp | any | yes |
| ] | LevelUp | any | no |
| Q | RotateLeft | any | no |
| E | RotateRight | management | no |
| E | Interact | direct control | yes |
| R | RotatePlacement | any | no |
| P | PhotoMode | any | no |
| Cmd+Enter | ToggleFullscreen | any | no |

Movement keys: `KeyMap.MoveRight` D / Right, `MoveLeft` A / Left, `MoveUp` W / Up, `MoveDown`
S / Down.

Bindings in W1: Escape at `Control` leaves direct control and at `Drawer` closes the panel (both
decline while a text field has focus); Tab at `Control` toggles direct control; Interact at
`Control` attacks a creature or interacts with a pile; zoom, Home and level keys at `World` (management
only); `PrimaryPress` at `Placement` places and at `World` picks a cat or building and opens the
inspector.

#### View hooks and call order

Declared as `partial void` in `Pres/View/ForestView.cs`: `AwakeLighting()`,
`UpdateLighting(float dt)`, `LevelChangedLighting(int level)`, `AwakeTerrain()`,
`UpdateTerrain(float dt)`, `LevelChangedTerrain(int level)`, `AwakeProps()`,
`UpdateProps(float dt)`, `LevelChangedProps(int level)`, `AwakeFx()`, `UpdateFx(float dt)`,
`LateUpdateFx()`, `AwakeMarkers()`, `LateUpdateMarkers()`, `AwakeAtmosphere()`,
`UpdateAtmosphere(float dt)`, `LateUpdateAtmosphere()`, `AwakeCloseUp()`, `LateUpdateCloseUp()`,
`UpdateBuildingMotion(float dt)`. None is implemented at W1.

- Awake: CreateCamera, CreateLights, CreateSelectionMarker, CreateInput, then the Awake hooks.
- Update: TickInput (before the world guard), world guard, village switch and follow,
  NotifyLevelChange, Reconcile every 0.2 s, AnimateCats, heartbeat, ApplyContinuousInput,
  UpdateCamera, UpdateSelection, then the Update hooks (Lighting, Terrain, Props, Fx,
  Atmosphere, BuildingMotion).
- LateUpdate: world guard, then Fx, Markers, Atmosphere, CloseUp.
- `dt` is `Time.unscaledDeltaTime`. Level hooks fire whenever `RenderedLevel` changes, including
  the first world frame.
- Reconcile order: TerrainTick, Buildings, Piles, Farms, Tiles, Boundary, Cats, CarriedCargo,
  Movers (vehicles, caravans, trader, raiders, creatures), RenderDungeonPlaces, cleanup.
- Other members: `ForestView.VisualName` (`"Visual"`), `VisualOf(GameObject)`,
  `ApplyFogForCamera(bool)`, `TerrainRaycast(Ray, out Vector3)` (layer 8), internal
  `TryGetEntity`, `CatTransforms()`, `WorldRoot`, `Zoom`, `CameraYaw`, `CameraStandoff`,
  `HoveredSubject` (always `Subject.None` at W1), `ForceReconcile()`, `TickInput()`,
  `ApplyContinuousInput()`, `UpdateCamera()`, `MakeGround()`, `groundCenter`, `groundRoot`,
  `ReconcileCarriedCargo(Village, int)`, `ItemCargoAsset(string)`; `CatMotion.PawContact`
  (`Action<string, Vector3>`, never raised at W1).

#### UI mount slots

| Slot | Mounted |
|---|---|
| `slot-chip` | VillageChip |
| `slot-seal` | AttentionSeal and AttentionPopover |
| `slot-pills` | ResourcePills |
| `slot-dock` | Dock |
| `slot-hint` | HintLine |
| `slot-drawer` | Glass drawer (DrawerHost) |
| `slot-card` | TetheredCard |
| `slot-toasts` | Toasts |
| `slot-mode` | ModeChip and BuildTray |
| `slot-reins` | ReinsBar |
| `slot-objective` | empty |
| `slot-modal` | empty |

The constants live in `UiSlots`. CatCard, BuildingCard, PileCard and LeaderLine are not mounted; they are W2-14's parts of TetheredCard.

#### UI element names

Format: `UiNames.Act(verb, ids...)` produces `act-<verb>-<id1>-<id2>...`.

Chrome and layout:

| Element | Name |
|---|---|
| Root | `forest-ui` |
| Village chip (W2-13) | `village-chip`, `chip-name`, `hud-clock`, `chip-census`, `chip-paused`, `chip-speed`, `act-show-idle` |
| Speed buttons | `act-speed-0`, `act-speed-1`, `act-speed-2`, `act-speed-4` (8 and 16 only with dev speeds) |
| Resource pills | `hud-resources`; `resource-food`, `resource-water`, `resource-logs`, `resource-stone`, `resource-research`, `resource-blessings` |
| Attention seal | `seal` (W2-16; popover and toasts beside it) |
| Dock | `dock-row`, `hud-dock`; entries `nav-<id lowercase>` |
| Hint line | `hud-hint` |
| Drawer | `drawer`, `drawer-dim`, `drawer-title`, `drawer-summary`, `act-close-drawer`, `drawer-scroll` |
| Section bodies | `section-<id lowercase>` (overview included; Inspect is gone) |
| Tethered card (W2-14) | `card-tethered`, `card-leader`, `card-portrait`, `card-name`, `card-facts`, `card-now`, `card-why`, `card-scroll`, `act-pin-card`, `act-close-card`, `act-more-card`, `card-more` |
| Card bodies | `card-cat`, `card-building`, `card-pile` |

Removed at W2-INT-b (the Glass chrome replaced them): `hud-top`, `hud-brand`, `hud-nav`, `hud-bottom`,
`act-zoom-out`, `act-zoom-in`, `act-focus-village`, `act-back-drawer`, `act-inspect-card`, `section-inspect`.

Village section:

- `act-join-<villageId>`, `field-village-name`, `act-found-village`
- `field-server-address`, `act-connect-server`, `act-disconnect-server`, `act-save-world`
- `act-vote-<catId>`, `act-sign-petition`

Cats section: `act-inspect-<catId>`.

Tethered card, cat (W2-14; the Inspect section is deleted):

- Needs and work: `ring-hunger`, `ring-thirst`, `ring-rest`, `ring-health`, `live-work-progress`, `carried-item-<itemId>`
- Actions: `act-help-<catId>`, `act-assign-to-<catId>`, `act-assign-<catId>-<buildingId>`,
  `act-release-workplace-<catId>` (sends `ReleaseWorker`), `act-open-work-card`, `act-control-<catId>`
- More: `act-labor-<labor>` (Toggle), `act-unequip-<itemId>`, `act-prefer-<catId>` (sends `PreferForWork`)

Tethered card, building and pile:

- Construction: `live-build-progress`, `card-material-<resource>`, `card-eta`, `act-card-cancel-<b>`,
  `act-card-cancel-yes-<b>`, `act-card-cancel-no-<b>` (sends `CancelConstruction`)
- Housing and storage: `card-beds`, `card-contents`; pile removal `act-card-remove-<pile>`, `-yes-`, `-no-`
- Workplace: `card-crew-<n>`, `act-assign-idle-<b>`, `act-card-boost-<b>`, `act-edit-queue-<b>`, `act-open-shrine-card`
- Queue editor (opened by `act-edit-queue-<b>`): `choice-recipe`, `act-queue-add-<b>`, `act-slot-add-<b>-<cat>`,
  `act-queue-pause|resume-<b>-<cat>`, `act-queue-up|down|repeat|remove-<b>-<cat>-<index>`,
  `queue-entry-<b>-<cat>-<index>`. `<cat>` is empty for the station queue, which gives a double dash.

Build section:

- `placement-road`, `placement-bridge`, `act-expand-village`
- `placement-<buildingKind>`
- `placement-zone-gather`, `placement-zone-avoid`, `act-remove-zone-<id>`

Work section:

- Pickers and manual jobs: `choice-cat`, `act-job-<labor>`
- Navigation: `act-open-stores`, `act-open-research-work`
- Resource jobs: `act-quarry-resource`, `act-scout-resource`
- Placements: `placement-farm-<crop>`, `placement-gather-spot`, `placement-fishing-spot`
- Farms: `act-assign-farm-<id>`, `act-clear-farm-<id>`
- Workplaces and jobs: `act-inspect-<buildingId>`, `act-boost-<jobId>`

Stores section:

- `placement-stockpile`, `placement-stockpile-filtered`
- `act-haul-pile-<id>`, `act-remove-pile-<id>`
- `act-equip-<itemId>`, `act-repair-<itemId>`

Research section:

- `act-research-view-cards`, `act-research-view-map`
- `choice-research-category`, `field-search-studies`
- `study-<id>`, `act-requires-<prereqId>`, `act-research-<id>`

Officers section: `act-appoint-<role>`, `act-vacate-<role>`.

Shrine section:

- `choice-offering`
- `act-offer-resource`, `act-offer-materials`, `act-offer-tithe`, `act-assign-ritual`
- `act-upgrade-<key>`, `act-unlock-<studyId>`

World section:

- `act-view-depth-<n>` (0 is the surface)
- `act-show-entrance-<dungeonId>`, `act-explore-<dungeonId>`
- `act-follow-<catId>`, `act-recall-<catId>`

Defense section: `act-train-warrior`, `act-defend-raid`.

Trade section:

- `act-buy-resource`, `choice-sale-item`, `act-sell-goods`
- `field-recipient-village`, `choice-request-resource`, `field-request-amount`, `act-offer-exchange`
- `act-accept-trade-<id>`, `act-cancel-trade-<id>`

Routes section:

- `choice-route-mode`, `placement-rail`, `placement-dock`, `placement-vehicle`
- `choice-source-pile`, `choice-destination-pile`
- `act-create-route`, `act-cancel-route-<id>`

#### Scenario prefix-to-file map

The same mapping is the comment at the top of `AcceptanceScenarios.cs`:

```text
Which file owns which case-name prefix (every name below is under regression. unless shown otherwise):
  DungeonScenarios.cs          dungeons_*, dungeon.*
  SpatialScenarios.cs          world_levels_*, spatial.*
  WorldGenerationScenarios.cs  world_generation_*
  SeamScenarios.cs             seam.*
  CoreScenarios.cs             continuous_* except continuous_storage_*, core.*
  NeedsScenarios.cs            cargo_critical_need_*, preserved_food_*, emergency_staffed_water, medicine_and_brew_*,
                               needs.*
  LivelinessScenarios.cs       liveliness.*
  PopulationScenarios.cs       review_housing_skips_disconnected_*, pregnancy_beds_*, death_preserves_*, extinction_*,
                               unhoused_migrant_*, population_*
  DemoPacingScenarios.cs       demo_pacing.*, demo_timing.* (demo_timing.* explicit in Unity, runner needs --include-demo-timing)
  HaulingScenarios.cs          continuous_storage_*, busy_cat_*, item_only_stockpile_*, exact_haul_*, review_scalar_haul_*,
                               review_armor_*
  JobRobustnessScenarios.cs    job_robustness.*
  ConstructionScenarios.cs     layout_* except layout_pending_*, review_expansion_respects_linear_*, buildable_founding_*,
                               communal_and_personal_enclosures, expansion_interruption_*, construction.*
  PlacementScenarios.cs        placement.*, review_expansion_reserves_*, review_fishing_placement_*, layout_pending_*
  ProductionScenarios.cs       recipe.*, chain.*, production_expertise_*, accountant_*, mountain_research_*,
                               station_capacity_*, repeat_queue_*, pause_preserves_*, officer_vacancy_*, selected_quarry_*,
                               farm_worker_replacement
  ColonyAutomationScenarios.cs colony_automation.*, demo_timing.colony_idle_ratio_10min
  EffectScenarios.cs           catalog.*, building_effect.*, service_effect.*, resource_effect.*, legacy_upgrade.*,
                               legacy_effect.*, resource_capacity_is_resource_specific
  DemoContentScenarios.cs      demo_content.*
  ControlScenarios.cs          direct_control_*, review_scaffold_preserves_*, review_new_building_preserves_*,
                               review_new_job_preserves_*, review_resume_*, gather_and_avoid_zones, resource_targeted_scout
  ActionContractScenarios.cs   action_contract.*
  TransportScenarios.cs        capability.*, rail_transfers_*, review_rail_*, review_transport_*, shipping_*
  GovernanceScenarios.cs       territory_*
  DefenseScenarios.cs          undefended_raid_*, functional_weapon_*, review_land_mover_*, defense.*
  RaidBeatScenarios.cs         raid_beat.*
  AttentionScenarios.cs        attention.*
  FoundingScenarios.cs         founding.*
  CampaignScenarios.cs         campaign.* (explicit in Unity, excluded by the runner unless --include-campaigns)
  ScenarioKit.cs               no cases: Scenario plus the shared fixtures and assertions
A new file names its cases <file prefix in snake_case>.<case> (job_robustness.* in JobRobustnessScenarios.cs) and
needs no registration: any static *Scenarios class with a public static Cases() is discovered below.
```

### 3.9 Wave 2 simulation additions (W2-INT-a)

Merged and wired on 2026-10-03 (W2-01 to W2-10). Members were only added; nothing in 3.1 to 3.8 was
renamed or removed. The scenario map above includes the new files.

**Demo founding.** `World.CreateColony(int seed, string colonyName, string pacingProfile, long nowUnixMs)`
founds the player's colony on generation version 2 (`World.DemoSeed` 16, `DefaultColonyName` "Mosslight",
`MaxColonyNameLength` 48, `ColonyRevealRadius` 34). `World.Validate` accepts generation versions 0 to 2. A
demo-profile founding owns `DemoContent.FoundingStudies` (the Research Hut) and the Leader waits one
`LeaderStudyIntervalSeconds` before its first study, so the first study is the player's. Every new
single-player world must use `CreateColony(World.DemoSeed, name, Pacing.DemoProfile, now)`; the switch in
`AuthorityRuntime` and `ForestGame` is W2-INT-b's or W3-04's (both files belong to W2-11 and W2-12).
`World.Create` stays on version 1 for the server and tooling.

**Identifier constants added.**
- `Blocked`: `ConstructionReserve` = `construction_reserve`, `ExpeditionWounded` = `expedition_wounded`,
  `ExpeditionStranded` = `expedition_stranded`.
- `EventKind`: `Repaired` = `repaired` (W2-08), `Appointment` = `appointment`; `EventSeverity` `Info`,
  `Notable`, `Alert` with `ForKind(kind)`. Info events (completed, discovery, repaired) live in
  `Village.RoutineEvents`; read both logs in order with `World.EventsSince(v, seq)`.
- `ResultCode`: the W2-06 list (successes, router, cat, queue, research, equipment, governance and
  direct-control failures; see `Ids.Codes.cs`), plus `BusyWithNeed`, `ReleaseControl`, `AlreadyExploring`,
  `NeedsRest`, `DungeonCleared`, `DungeonDeadly`, `ChestGuarded`, `NoChestInReach`, `Rallied`,
  `NoActiveRaid`, `RallyCooldown`, `NoDefenders`. `PlacementCode` (Placement.cs) holds the placement codes.
- `GoalKey`: the six `idle_*` goals are live; `resting at home` is only read from old saves.

**Pacing knobs added** (classic value, then demo value where it differs). Total: 293 public fields
(Jobs 64, Life 87, Population 28, Research 24, Threats 65, Trade 15, World 8, core 2).
- `Pacing.Life.cs`: `NeedsHealthFloor` 0 / 10, `NeedStagger` false / true, `NeedSeedMin` 55, `NeedSeedMax` 100,
  `DeliveryLegFinishTiles` 15, `NeedBackoffMaxDoublings` 4, `NeedSourceSearchLimit` 4, `IdleWalkLimitSeconds` 45,
  `IdleTileShare` 2, `LeaderStockTarget` 60, `LeaderGathersPerResource` 2, `EmergencyCooldownSeconds` 60,
  `EmergencyProtectedProgress` 0.5.
- `Pacing.Population.cs`: `MigrationRequiresFreeBed` false / true, `HousingSitingRetrySeconds` 600.
- `Pacing.Jobs.cs`: `ToolQualityWorkScale` 0 / 0.375, `BaseCarryLoad` 8, `BuildWorkBase` 120,
  `BuildWorkPerExisting` 30, `BridgeLumber` 4, `DockLumber` 8, `DemolishRefundShare` 0.5.
- `Pacing.Research.cs`: `BoostJobFraction` 0 / 0.05, `BlessingStudyCostFactor` 1 / 3, `HurryStudySeconds` 300.
- `Pacing.Threats.cs`: `RootHollowThreat` 0.9, `GuardianAttackPerThreat` 4.6, `GuardianDefensePerFloor` 0.25,
  `FightFloorHealth` 15, `CombatVariance` 0.25, `SafeLossShare` 0.25, `GuardianRegenPerSecond` 0.5,
  `FloorOneLore` 5, `FloorTwoLore` 8, `RaidReferenceStrength` 30, `RallySize` 4, `RallyCooldownSeconds` 60,
  `DefenderFloorHealth` 15, `RelicCombatBonus` 1.1, `ScriptedBandMembers` 3, `ScriptedRaiderHealth` 6,
  `ScriptedRaiderStrength` 15, `NaturalRaiderHealth` 10, `NaturalRaiderStrength` 15, `CatsPerRaider` 8,
  `MaxRaiders` 6. `RaidMinWealth` is no longer read.
- The `Demo<Domain>()` hooks are filled (W2-01); `demo_pacing.*` pins the classic table and every demo
  difference field by field, and `docs/unity/GAMEPLAY_ACCEPTANCE.md` explains each demo value.

**Public simulation API added.**
- Population: `EligibleForOffice(Cat)`. Needs and idle life: static `BusyWithNeed(Cat)`, `RestRate(Village)`,
  `NeedThreshold(Cat, string)`, static `IsIdleActivity(string)`; `DayClock.SleepThresholdBias(World)`;
  `CatActivity.Shrine`.
- Jobs: `CanTakeOrders(Cat)`, `ReserveNearest(Village, owner, resource, amount, Int2 from)`,
  `ToolWorkBonus(Village, Cat)`; serialized `Job.LastProgressAt`, `Village.SourceExclusions`.
- Construction and placement: `CheckPlacement`, `CheckLine`, `CheckArea`, `LineFootprint`, `PlanTransportPath`,
  `BuildCost`, `LineCost`, `InfrastructureCost`, `ExpansionCost`, `Shortfalls`, `PlayerBuilt`, `MaxLineTiles`;
  types `PlacementCheck`, `PlacementCode`, `MaterialShortfall`, `BuildBill`. `World.DemoConstruction` gates
  queued plans, extra builders, full loads, player-built pricing and natural expansion barriers to demo worlds.
  Router kinds `CancelConstruction`, `Demolish` and `DemolishBuilding` are live.
- Actions: `World.ActionKinds`, `NormaliseKind`, `ActionAvailable(PlayerContext, kind)`, `IsWorkplace`,
  `InReach`, `AtShrine`, `OfferingBlessings`, `OfferableResources`, `BlessingStudyCost`; kinds `ReleaseWorker`,
  `PreferForWork`, `HurryStudy`; non-saved `World.ControlTimeScale`; `ActionResult.Coded`, `ActionResult.Number`.
- Automation and estimates: `EstimatedTotal(v, resource, out bool isEstimate)`, `EstimateAgeSeconds(v)`,
  static `CoarseCount(x)`.
- Events and faults: `Event.Position`, `Severity`, `Args`, `EffectiveSeverity()`; `NoteMilestone`,
  `NoteBuildingCompleted`, `NoteResourceDiscovered`, `NoteStorageFull`, `NoteStorageEmpty`, `NoteRankUp`,
  `NoteRaidRepelled`, `NoteExpeditionLoot`; `IsolatedFaults()`, `ValidateFatal()`, `ValidateRepairable()`,
  `RepairInvariants()`. `Step` isolates faults per system; scenarios assert none happened (`ScenarioKit.Valid`).
- Threats: `RiskOf(cat, floor)`, `DungeonChestLocked`, `TakeDungeonChest`, `MaterialiseStairEndpoints`,
  `ContinueExpeditionAfterControl`; `GameAction.AcceptDeadly`; `Village.RaidsRepelled`, `DeepestLootFloor`.
- Content: `DemoContent` (`IsDemo`, `Recipe`, `Recipes`, `AllRecipes`, `Study`, `Studies`, `PriorityOf`,
  `IsDemoVisible*`, `IsFunctional`, `OwnedAtFounding`, `SlotsFor`, `ServiceFactor`, `FoundingServiceFactor`,
  `Slice`, `FoundingStudies`) and the `Catalog` overloads that take a `World`. Simulation code asks the
  World overloads, so demo worlds see demo costs, prerequisites, slots and founding service effects.

### 3.10 Wave 2 authority, game loop and UI additions (W2-INT-b)

Merged and wired on 2026-10-03 (W2-11 to W2-16). Members were only added, except the retired
Inspect drawer (`InspectSection` is deleted; `DrawerHost.InspectId` stays as a no-op so
`OpenPanel("Inspect")` from old capture scripts only syncs the selection).

**Persistence and identity (W2-11).** Local play has one stable player (`World.Players[0]`, or the one
named by an existing `<save>.identity/session.json`), no bearer and no expiry; the server keeps
`SessionAuthority`. New: `LocalAuthority.RestoreLastGood()` (implemented), `Snapshot()`,
`Write(SaveSnapshot)`, `PlayerId`, `SavePath`, `Created`, `LoadWarnings`; `AuthorityRuntime.Snapshot()`,
`Write(SaveSnapshot)`, `ValidateStructure(World)`, `LoadWarnings`, `Created`, `SavePath`; `Save()` returns
`SaveResult`. Every open or create failure is a `SaveException` (an `IOException`) with `Code` from
`SaveFailure`: `damaged`, `made_by_newer_version`, `unsupported_version`, `too_large`, `already_running`,
`already_exists`, `missing_save_with_backup`, `invalid_ownership`. `SaveStore.Load(path, out
SaveLoadReport)`, `DecodePayload`, `Encode`, `Write`; `SaveMigrations.Migrate` and `OldestSupported`;
`Backups.SessionBackupPath`, `PreviousPath`, `InvalidCopyPath`, `Recoverable`, `TakeSessionSnapshot`,
`RemoveStaleTemporaries`. Simulation invariants never block an open or a save: repairable breaches are
fixed by `World.RepairInvariants()` on open and before every snapshot, the rest become warnings.
Migration rule: adding a field needs nothing; a rename or a change of meaning bumps `SaveStore.Version`
and adds a step in `Authority/SaveMigrations.cs` with a test that loads
`server/Forest.Tests/Fixtures/world-v1.main-5ec7278.json.gz`.

**Game loop (W2-11, W2-12).** On `ForestGame`: `SaveState` (Unavailable, Idle, Saving, Saved,
SavedWithWarnings, Failed), `LastSaveUtc`, `LastSaveError`, `SaveWarnings`, `ConsecutiveSaveFailures`,
`SaveMilliseconds`, `SaveMainThreadMilliseconds`, `AutosaveIntervalSeconds` (30, wall clock),
`ActionSaveDelaySeconds` (2), `OpenFailureCode`, `LastFeedback`, `FeedbackPosted`, `ActAsync`,
`FaultCount`, `SimulationBehind`, `FocusedFrameRate` (60), `BackgroundFrameRate` (10),
`DefaultStepBudgetMilliseconds` (6), `ConnectTimeoutMilliseconds`; the telemetry list of persistence-tick
timings is `PersistenceTickMilliseconds`. `Faulted` and `Changed` work as specified: a throwing step
pauses, restores the last good world and never autosaves until a step succeeds. `ForestGame` sets
`World.ControlTimeScale = Speed` before stepping and logs `World.IsolatedFaults()` once each (the server
tick does the same). New types `Feedback`, `FeedbackCode` (`io_error`, `timeout`, `network`,
`invalid_address`, `unexpected_error`, `already_sent`, `not_ready`, `simulation_fault`,
`server_controls_time`) and `ForestLog`. Speeds are pause, 1x, 2x, 4x (`GameSpeeds.Clamp`).

**Glass chrome (W2-13).** ConstantPixelSize panel with `ForestUI.PanelScaleFor(dpi, uiScale)`; root
classes `has-minimap` (width 1600 and up), `compact` (below 1600), `narrow` (below 1100); tokens in
`Resources/UI/Tokens.uss`; Manrope and Instrument Serif (OFL) and the generated icon set under
`Resources/UI/Fonts` and `Resources/UI/Icons` (deterministic metas from `tools/ui-icons`). Slot order back to
front: chip, pills, hint, drawer, dock, card, seal, toasts, mode, reins, objective, modal. `UiState.Bind`
binds Escape as text field (Modal + 10), popover, placement, drawer, selection. `ForestUI.ChipBounds`,
`IsNarrow`, `PanelScale`; `SectionRegistry.Dock` with groups; `DrawerHost.SummaryOf`. The Space and 1/2/3
bindings exist but only fire once `KeyMap` marks them live (W2-18, merged in W2-INT-c).

**Tethered card (W2-14).** The only inspector. `ForestUI.Select(Subject)` handles cats, buildings and
piles; `ForestView.InspectPile`, `SelectedPileId`, `SelectedSubject`; `TetheredCard.PlaceNear`, `Showing`,
`IsSheet`; `BuildingCard.RoleOf`, `BestIdleCat`; `CatCard.Candidates`; `NeedRing`; `CardPortrait.Layer` 31.

**Copy (W2-15, W2-INT-b).** `UiText` hands off to `Presentation/UI/Text/**` (`Names`, `Numbers`,
`GameClock`, `NowLines`, `Reasons`, `Confirmations`, `Effects`, `EventLines`, and `HudCopy` moved there
from Widgets at W2-INT-b). `tools/presentation-tests` (in the solution and CI) runs under de-DE; its
Coverage category reflects over `Blocked`, `ResultCode`, `EventKind`, `GoalKey`, `JobKind`, `JobPhase`,
`PlacementCode`, `World.ActionKinds` and `AttentionRule`, and checks the `FeedbackCode` values, so a code
added later fails until its text is written. Every Wave 2 code above has authored text.

**Widgets and attention (W2-16).** `Attention.Compute(World, Village)` is pure; rules `raid`,
`raid_warning`, `explorer_danger`, `need_thirst`, `need_hunger`, `need_health`, `no_bed`,
`workplace_blocked`, `research_unstaffed`, `study_affordable`, `workplace_vacant`, `idle_cats`,
`stale_counts` (`AttentionRule.All`); `AttentionItem.Arg(key)`, `AttentionArg`, `AttentionSection`,
`AttentionAction.OpenSection`; `Attention.CountBySection` feeds the dock badges. Widgets: `ActionButton`
(pending, in-place result, inline confirm for destructive verbs), keyed `Table`, `BoundDropdown`,
`StateChip`, `Spark`, `Ring`, `Tooltip`, `Glyph`, `WidgetStyles`, `HudLayout`; `AttentionSeal`,
`AttentionPopover`, `Toasts` (bodies use `UiText.EventLine`). Scenario prefix: `AttentionScenarios.cs`
owns `attention.*`.

