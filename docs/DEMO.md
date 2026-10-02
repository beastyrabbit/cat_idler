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
| W0 | Guardrails: compile gate, worktrees, Unity slots, captures, CI, demo brief, seams | 5 (1 integration) | In progress | `demo-w0` | `artifacts/demo/wave-0/report.md` |
| W1 | Behaviour-neutral splits and every frozen contract | 9 (1 integration) | Not started | `demo-w1` | `artifacts/demo/wave-1/report.md` |
| W2 | Core rebuild in parallel; staged integration A, B, C | 29 (3 integration) | Not started | `demo-w2a`, `demo-w2b`, `demo-w2` | `artifacts/demo/wave-2/report.md` |
| W3 | Features: milestones, shell, settings, demo boundary, build mode, drawers, direct control, animation, art hookup, FX, markers, audio, art | 22 (1 integration) | Not started | `demo-w3` | `artifacts/demo/wave-3/report.md` |
| W4 | Polish and ship-readiness, Windows (conditional), playtest punch lists | 12 (1 integration) | Not started | `demo-w4` | `artifacts/demo/wave-4/report.md` |
| W5 | Docs, store page, trailer, release candidate | 4 (1 integration) | Not started | `demo-w5` | `artifacts/demo/wave-5/report.md` |

## Appendix: Frozen contracts

Copied from section 3 of the wave plan; integrators keep it current.

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
| `Milestones.Current(World, Village)` returning `MilestoneStatus { Id, Index, Count, Progress, HintCode, Complete }`; `Milestones.DemoComplete(World, Village)` | `Sim/Milestones.cs` | W1-02 (returns none / false) | W3-01 |
| `Attention.Compute(World, Village)` returning `IReadOnlyList<AttentionItem>`; `AttentionItem { Id, Severity, TitleCode, CauseCode, Section, Anchor, ActionKind, Args }`; `Attention.CountBySection(World, Village)` | `Sim/Attention.cs` | W1-02 (empty) | W2-16, then W3-06 |
| `Die(Village, Cat, string cause)` overload | `Sim/Systems/Population.cs` | W1-02 (ignores cause) | W3-02 |
| `DemoScope`: `SharedWorldEnabled`, `IsSectionVisible(string)`, `IsStudyVisible(string)`, `IsBuildingVisible(string)`, `MaxDungeonDepth`, `Check(GameAction)` | `Auth/DemoScope.cs` | W1-05 (all visible, Check ok) | W3-06 |
| `ShellLinks`: `StoreUrl` (null), `WishlistAvailable` (false), `OpenStore()`, `FeedbackUrl` | `Pres/Shell/ShellLinks.cs` | W1-05 | W3-06 |
| `GameSettings`: static `UiScale` (1), `LargerText`, `ReducedMotion`, `Volume(string bus)` (1), `MuteInBackground`, `Quality`, `event Action Changed` | `Pres/Shell/GameSettings.cs` | W1-05 (defaults, no persistence) | W3-05 |
| `UiEvents`: static events `Clicked`, `Confirmed`, `Failed`, `Opened`, `Closed`, `Attention`, `Flourish(string kind)` | `Pres/UI/UiEvents.cs` | W1-05 (declared, raised nowhere) | W2-16 |
| `ForestGame.Faulted` (false), `ForestGame.Changed` event, `ForestGame.PersistenceTick(double steppedSeconds)` called once per frame from `Update` | `Pres/Game/ForestGame.cs`, `Pres/Game/ForestGame.Persistence.cs` | W1-07 | W2-12 (Faulted), W2-11 (PersistenceTick) |
| `LocalAuthority.RestoreLastGood()` returning bool (false) | `Auth/LocalAuthority.cs` | W1-07 | W2-11 |
| `CatMotion.PawContact` static event `(string catId, Vector3 position)` | `Pres/View/CatMotion.cs` | W1-06 (never raised) | W3-13 raises, W3-17 subscribes |
| `Visual` child: every entity root has one child named `Visual` holding the model; `ForestView.VisualOf(GameObject)`; reconcile writes only the root | `Pres/View/ForestView.Entities.cs` | W1-06 | used by W3-14, W3-15, W4-07 |
| Carried cargo rendering in its own partial (`ReconcileCarriedCargo`, `ItemCargoAsset`) | `Pres/View/ForestView.Cargo.cs` | W1-06 | W3-13 (sockets) |
| Accessors: `TryGetEntity`, `CatTransforms()`, `CameraStandoff`, `Zoom`, `CameraYaw`, `RenderedLevel`, `CursorTile`, `HoveredSubject`, `ForceReconcile()` | `Pres/View/*.cs` | W1-06 | kept by every later owner |
| UI skeleton (ForestUI facade, `Select(Subject)`, mount slots, UiState, UiContext, ISection, SectionRegistry, DrawerHost, widgets, UiText final API, chrome, card and build-mode stubs) | `Pres/UI/**` | W1-05 | see W2 and W3 sections |

Mount slots in `ForestUI`: `slot-chip`, `slot-seal`, `slot-pills`, `slot-dock`, `slot-hint`,
`slot-drawer`, `slot-card`, `slot-toasts`, `slot-mode`, `slot-reins`, `slot-objective`,
`slot-modal`.

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
  PrimaryDragEnd }`. Pointer actions are not dispatched while `PointerOverUi` is true. A
  primary press that moves more than 6 px becomes a drag and produces no click.
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
`ForestGame.CurrentWorld`, `ForestGame.UI`, `ForestView.FocusVillage`, `ForestView.ZoomBy`,
`ForestView.InspectCat`, `ForestView.InspectBuilding`, `ForestView.ViewLevel`,
`ForestUI.OpenPanel(string)`, `ForestUI.ClosePanel()`, `ForestUI.OpenSection` (read-only property). These signatures are frozen until W5-03 owns
the capture folder. `--forest-save <path>` must keep opening that world directly (no title
screen) for tooling, also after W3-04.
