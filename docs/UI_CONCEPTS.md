# UI Concept Variants

This document catalogs all 13 visual concept variants created for the Cat Colony Idle Game during the UI redesign exploration. **V9 — The Catford Examiner** was selected as the production direction.

Each variant is a self-contained `page.tsx` file under `app/game/v{N}-{slug}/`. All variants share the same data layer via `useGameDashboard` from `@/hooks/useGameDashboard` and `summarizeCatIdentity` from `@/lib/game/catTraits`. Styles are inline or injected via `<style>` tags — no globals are modified.

A `ConceptSwitcher` component (fixed pill at the bottom of the viewport) allows navigation between all variants at `/game/v1-terminal` through `/game/v13-mail`.

---

## V9 — The Catford Examiner (SELECTED)

**Route:** `/game/v9-newspaper`
**Concept:** 1960s broadsheet newspaper
**File:** `app/game/v9-newspaper/page.tsx`

### Design Philosophy

The entire game UI is presented as a vintage broadsheet newspaper — "The Catford Examiner". Every game mechanic maps to a newspaper element: resources are market tickers, jobs are classified ads, cats are society page entries, events are dispatches from the wire, morale is the weather forecast, and upgrades are display advertisements.

The humor comes from treating a cat colony as utterly serious local news. Headlines change dynamically based on colony status ("PROSPERITY! FOOD STORES SWELL TO RECORD HIGHS" vs "FAMINE GRIPS COLONY AS FOOD STORES DWINDLE"). The tagline: "All the Mews That's Fit to Print."

### Palette

| Token       | Hex       | Usage                                 |
| ----------- | --------- | ------------------------------------- |
| `paper`     | `#F5F0E8` | Page background — warm aged newsprint |
| `paperDark` | `#EDE6D6` | Scrollbar tracks, secondary surfaces  |
| `ink`       | `#1A1A18` | Primary text, heavy rules, borders    |
| `inkLight`  | `#3D3D38` | Secondary text, body copy             |
| `inkFaded`  | `#8A8578` | Tertiary text, datelines, captions    |
| `rule`      | `#B8B0A0` | Light rules, dotted borders           |
| `ruleHeavy` | `#7A7468` | Ornamental rules                      |
| `red`       | `#8B1A1A` | Danger, critical status, red buttons  |
| `redBright` | `#C0392B` | Active nav highlight                  |
| `green`     | `#2E5E1A` | Positive values, healthy resources    |
| `blue`      | `#1A3A5E` | Water-related, blue buttons           |
| `gold`      | `#8B6914` | Ritual points, treasury callouts      |
| `accent`    | `#D4A017` | Subtle highlight backgrounds          |
| `white`     | `#FEFCF5` | Pure white for contrast               |

### Typography

| Role                | Font             | Fallback                        | Usage                                                                         |
| ------------------- | ---------------- | ------------------------------- | ----------------------------------------------------------------------------- |
| Display / Headlines | Playfair Display | Georgia, Times New Roman, serif | Masthead title, section headers, headlines, ad headers, cat names             |
| Body / Captions     | Special Elite    | Courier New, monospace          | Datelines, time stamps, resource values, subscriber name, press room controls |
| Buttons             | Playfair Display | Georgia, serif                  | All `.ce-btn` buttons — uppercase, letter-spaced                              |

### Layout Structure

The page is divided into **5 full-width sections**, each navigable via a sticky nav bar. Sections are separated by `<HeavyRule />` elements (3px double border).

#### Navigation Bar

Positioned below the masthead as `position: sticky; top: 0`. Styled as newspaper section tabs:

- Items separated by `◆` diamond characters
- Active item: `color: red` with 2px bottom border
- When stuck (scrolled past sentinel): subtle box-shadow appears
- Newspaper typography: Playfair, 11px, uppercase, 0.2em letter-spacing
- Background matches page paper color

#### Page Turn Animation

Clicking a nav item triggers a GPU-composited sweep animation instead of smooth scroll:

1. **Phase "out"** (380ms): A paper-colored `::before` pseudo-element sweeps from right to left across the content, covering it. The overlay uses `transform: translateX()` with `skewX(-5deg)` for a slight diagonal edge. A fold-shadow gradient on the leading edge simulates paper depth.
2. Content swaps via `scrollIntoView({ behavior: "instant" })` while covered.
3. **Phase "in"** (340ms): The overlay continues sweeping left, revealing the new section.
4. `pointer-events: none` during animation prevents interaction.

CSS classes: `.ce-content-wrap`, `.ce-turning-out`, `.ce-turning-in`
Keyframes: `ce-sweepCover`, `ce-sweepReveal`

#### Section 1: HEADLINES

Full-width section at the top.

- **Masthead**: Top info line with date, reader count, run number, and "Price: 2 Kibble"
- **Title**: "The Catford Examiner" in large Playfair caps (clamp 36px–72px)
- **Subtitle**: Colony name, edition number (deterministic from run number), tagline
- **Lead Headline**: Dynamic headline based on colony status (see `statusHeadline()` function)
- **Subhead**: Secondary line based on status and population
- **Status badge**: Colony status displayed with appropriate color

Dynamic headlines by status:

- `thriving` + food > 80%: "PROSPERITY! FOOD STORES SWELL TO RECORD HIGHS"
- `thriving` + population >= 8: "COLONY THRIVES AS POPULATION REACHES NEW PEAK"
- `thriving` default: "GOLDEN AGE CONTINUES FOR HAPPY COLONY"
- `struggling` + water < 20: "CRISIS: WATER RESERVES DANGEROUSLY LOW"
- `struggling` + food < 20: "FAMINE GRIPS COLONY AS FOOD STORES DWINDLE"
- `struggling` default: "COLONY STRUGGLES THROUGH DIFFICULT TIMES"
- `dead`: "COLONY FALLS: ALL HOPE LOST"
- `starting`: "NEW COLONY ESTABLISHED: EARLY DAYS OF PROMISE"

#### Section 2: MARKET

Two-column grid (1.8fr | divider | 1fr).

**Left column:**

- **Leader Lede** — Prose paragraph about the colony leader, written in formal newspaper voice. "From the Office of the Colony Leader" header. Uses drop-cap style. Dynamic text based on leader name, colony status, active jobs, queued jobs, ritual points.
- **Market Report** — Resource bars (Food, Water, Herbs, Materials) styled as market tickers. Each shows value/max, trend label (STRONG/STEADY/WEAK/ADEQUATE/LOW/ABUNDANT/SUFFICIENT/SCARCE), and a vintage bar chart with hatching lines (45-degree repeating linear gradient).
- **Colony Treasury** — Gold-bordered callout box showing ritual points in large display font.

**Right column:**

- **Situations Wanted** (Jobs) — Each job displayed as a classified listing with:
  - Job title in uppercase Playfair
  - Assignment status and cat name in monospace
  - Progress bar with percentage
  - ETA countdown and boost count
  - "Boost" button for active jobs
  - Empty state: "No positions currently filled. All cats at leisure."

#### Section 3: COMMUNITY

Two-column grid (1fr | divider | 1.4fr).

**Left column:**

- **Place Your Order** — Action buttons for submitting jobs:
  - Supply Food (20s), Supply Water (15s)
  - Request Hunt, Request Build
  - Request Sacred Ritual (full-width, red)
  - Subhead: "Citizens may submit the following requisitions"
- **Subscriber Name** — Nickname input styled as a form field with "SUBSCRIBER NAME:" label

**Right column:**

- **Society Pages** — Cat roster. Header: "Who's Who in {colony name} ◆ {population} Residents"
  - Each cat: clickable card with name (bold, Playfair), specialization badge, leader indicator
  - Expanded view shows: identity description, stats (attack/defense/hunting/medicine/cleaning/building/leadership/vision), role XP bars
  - Empty state: "The colony stands empty. No residents to report."
- **Obituaries** — Dead cats listed with "In Memoriam" header, each showing "Remembered as a fine {specialization}"

#### Section 4: NEWS

Two-column grid (1fr | divider | 1fr).

**Left column:**

- **Colony Weather** — Morale displayed as weather forecast:
  - > 80%: "SUNNY" — "Clear skies and high spirits across the colony. Expect purring."
  - > 60%: "PARTLY CLOUDY" — "Generally pleasant conditions with occasional grumbling."
  - > 40%: "OVERCAST" — "A pall of discontent hangs over the colony. Naps are restless."
  - > 20%: "STORMY" — "Tempers flare and hissing is commonplace. Take cover."
  - <= 20%: "CATASTROPHIC" — "Colony morale at rock bottom. Even the mice feel sorry for us."
  - Shows morale percentage and outlook
- **Latest Dispatches** — Event log (up to 20 entries) with timestamp, color-coded messages (red for critical, green for positive)
  - Empty state: "No dispatches received. The wire is quiet."

**Right column:**

- **Advertisements** — Upgrade cards styled as vintage display ads:
  - Each upgrade in a bordered box with large uppercase header
  - Level indicator ("Tier {level}/{maxLevel}")
  - Cost in ritual points
  - "Purchase" button (disabled when maxed or can't afford)
  - Empty state: "No advertisements placed this edition."

#### Section 5: EXTRAS

Three-column even grid.

- **Amusements** — Decorative crossword grid spelling CATS/IDLE/MEOW/PURR/NAP. Puzzle clue: "5 letters, 'Feline pastime'"
- **Editorial Cartoon** — ASCII cat art that changes with colony status:
  - thriving: `( o.o )` — "Purrfect!"
  - struggling: `( T_T )` — "Mrow..."
  - dead: `( x_x )` — "...zzz"
  - starting: `( ?.? )` — "Hmm?"
- **Pull Quote** — "A colony is only as strong as the kibble in its stores and the purrs in its hearts." — The Editor

#### Footer / Colophon

"Published continuously since the founding of {colony name}" | "The Catford Examiner" | reader count + time

### Decorative Elements

- **HeavyRule**: `borderTop: 3px double ink` — separates major sections
- **LightRule**: `borderTop: 1px solid rule` — separates subsections
- **OrnamentalRule**: Unicode ornamental characters `──═─✦═──` centered
- **SectionHeader**: Double-bordered label with uppercase Playfair text, centered
- **Column Dividers**: 1px-wide `<div>` elements with `background: rule` color
- **Drop caps**: First letter of leader lede styled larger via `.ce-dropcap` class

### Scroll & Reveal Animations

- **Section Reveal**: IntersectionObserver at `threshold: 0.1` fires once per section. Sections start with `opacity: 0, translateY(20px)` and animate to visible over 600ms. Right columns have 150ms stagger delay.
- **Nav Sentinel**: A zero-height div above the nav bar triggers `isNavStuck` state via IntersectionObserver.
- **Section Tracker**: Another IntersectionObserver watches all sections to update `activeSection` in the nav bar.

### Special States

- **Loading**: Full-page "Preparing this morning's edition..." with pulsing animation, styled as newspaper front page
- **Error**: Red banner: "CORRECTION NOTICE: {error}"
- **Test Controls**: Gold-bordered "Press Room Controls" bar with Off/Fast/Turbo buttons

---

## V1 — CATNET Terminal

**Route:** `/game/v1-terminal`
**Concept:** NASA Mission Control surveillance terminal
**File:** `app/game/v1-terminal/page.tsx`

### Design

A fixed-viewport (100vh, no scrolling) terminal interface styled as a NASA mission control system monitoring cats through surveillance sensors. Deep navy background (`#0a0e1a`), phosphor green text (`#39ff85`), amber alerts (`#f59e0b`). All text in JetBrains Mono monospace.

### Key Elements

- **ASCII cat art** in the header: `/\_/\` with ` ( o.o )` body
- **Resource bars** as ASCII block-character meters: `[████████░░░░]`
- **Three-column layout**: Command Panel (left) | Active Operations (center) | Sensor Feed (right)
- **Terminal panels** with hard borders (no rounded corners), titled with shortcut labels
- **Keyboard-driven overlays**: `[C]` Cat Dossiers, `[U]` Upgrade Tree, `[E]` Event Log — each opens a Radix Dialog styled as a sub-terminal with scanline effect
- **Status displayed as**: NOMINAL (thriving), ELEVATED (struggling), CRITICAL (dead), INITIALIZING (starting)
- Clinical surveillance language: "PURR LEVEL: NOMINAL / HUNGER ALERT: ELEVATED"

### Palette

- Background: `#0a0e1a` (deep navy)
- Surface: `#111827` (dark panel)
- Text: `#39ff85` (phosphor green)
- Accent: `#f59e0b` (amber alerts)
- Danger: `#ef4444` (red)

### Recreation Notes

- CSS animations: scanline sweep, phosphor glow, blinking cursor, boot sequence
- Click-boost triggers green border pulse + inline `> -10s applied` log entry
- Loading shows "INITIALIZING CATNET v2.1..." with blinking cursor

---

## V2 — Pawprint Gazette (Retro RPG)

**Route:** `/game/v2-retro`
**Concept:** 16-bit JRPG / classic RPG interface
**File:** `app/game/v2-retro/page.tsx`

### Design

An RPG game UI with windowed panels, pixel-art-inspired resource bars using block characters, and quest/guild terminology. Dark background with gold and emerald accents.

### Key Elements

- **RPGWindow** panels with double-bordered frames and pixel shadow
- **RPG class system**: Hunter → Ranger, Architect → Artificer, Ritualist → Mystic, Unassigned → Adventurer
- **Quest names**: supply_food → "Forage Quest", supply_water → "Spring Fetch", ritual → "Ancient Ritual"
- **Pixel bars** using `█` and `░` characters for HP/resource display
- **RPG status**: thriving → PROSPEROUS ♡, struggling → ENDANGERED ⚠, dead → FALLEN ☠
- **RPGDialog** overlays (Radix Dialog) styled as RPG menu screens
- **Blinking cursor** component for RPG text feel
- **RPGButton** with gold/blue/red variants
- **Event log** styled as RPG battle text ("A wild encounter appeared!")
- **Tabs** (Radix Tabs) for switching between Guild, Quests, and Shop views

### Palette

- Hunter: `#33cc33` (green), Architect: `#ffd700` (gold), Ritualist: `#bb88ff` (purple)

### Recreation Notes

- Uses Radix Dialog + Radix Tabs primitives
- Tab-based navigation between main game panels
- Gold-bordered "Guild banner" header section

---

## V3 — Colony Dashboard (Corporate)

**Route:** `/game/v3-dashboard`
**Concept:** Corporate KPI dashboard / SaaS analytics
**File:** `app/game/v3-dashboard/page.tsx`

### Design

A clean, white corporate dashboard that reimagines cat colony management as a business productivity tool. Indigo accent color, card-based layout, "Sprint" and "KPI" language throughout.

### Key Elements

- **Corporate labels**: supply_food → "Catering Procurement", supply_water → "Hydration Logistics", hunt_expedition → "Field Operations — Hunt", ritual → "Team-Building Ceremony"
- **Status language**: thriving → "Excellent" (All KPIs within target. Colony health score: A+), dead → "Critical" (Colony in triage)
- **Upgrade labels**: hunt_efficiency → "Hunt Ops Efficiency — Reduces field operation cycle time"
- **Specialization titles**: hunter → "Field Ops Director", architect → "Infrastructure Lead", ritualist → "Culture & Morale Officer"
- **Active Sprints** section for job tracking
- **KPI Metrics Row** at the bottom
- Radix Tabs for view switching

### Palette

- Background: `#ffffff`, Accent: `#4f46e5` (indigo), Green: `#059669`, Amber: `#d97706`, Red: `#dc2626`

### Recreation Notes

- Uses Radix Tabs for panel navigation
- Cards with subtle borders and shadows
- Corporate wellness language throughout

---

## V4 — Whisker Woods (Field Journal)

**Route:** `/game/v4-journal`
**Concept:** Victorian naturalist's field journal
**File:** `app/game/v4-journal/page.tsx`

### Design

A hand-written field journal documenting cat colony observations. Warm cream/parchment tones, handwriting font (Caveat), Latin classification system for cats, tally mark resource meters.

### Key Elements

- **Latin subspecies**: hunter → "Felis venator silvestris", architect → "Felis architectus structura", ritualist → "Felis mysticus caeremonialis"
- **Tally mark meters** for resources (rendered as groups of 5 with strike-throughs)
- **Resource units**: food → "portions", water → "flagons", herbs → "bundles", materials → "loads"
- **Status language**: thriving → "Flourishing" ("The colony prospers. Subjects well-fed and content."), struggling → "Under Duress"
- **Radix Dialog** overlays styled as journal sub-pages
- **Handwriting font** (Caveat) for notes and annotations
- Serif body text (Playfair or similar)

### Palette

- Healthy: `#2d6a4f` (forest green), Danger: `#c44536` (dried blood), Caution: `#8b6914` (aged ink)

### Recreation Notes

- Font: `var(--font-caveat)` for handwriting feel
- Ink spots, aged paper texture feel
- Each cat entry reads like a naturalist's observation

---

## V5 — MEOW! (Brutalist)

**Route:** `/game/v5-brutalist`
**Concept:** Brutalist design — stark, raw, no rounded corners
**File:** `app/game/v5-brutalist/page.tsx`

### Design

Aggressive brutalist design with off-white background, pure black text, electric blue/coral/acid green accents. All caps, thick borders, no border-radius anywhere. Animated ticker tape for events.

### Key Elements

- **Color coding by specialization**: Hunter → coral (`#ff3366`), Architect → blue (`#0055ff`), Ritualist → acid green (`#ccff00`)
- **Ticker tape animation** for events: `meow-ticker` CSS keyframe scrolling text horizontally
- **Radix Dialog** overlays for detail panels
- **Rotating element** animation (`meow-spin`) for loading states
- **Flash animations** on boost/action events
- **Large, blocky typography** — everything uppercase
- **Status**: THRIVING (blue), STRUGGLING (coral), DEAD (coral), STARTING (muted)

### Palette

- Background: `#f5f5f0`, Black: `#000000`, Blue: `#0055ff`, Coral: `#ff3366`, Acid: `#ccff00`

### Recreation Notes

- 6+ CSS keyframe animations (spin, pulse, ticker, flash-blue, flash-coral, flash-acid, boost-bar, slide-in)
- No rounded corners on anything
- Maximum visual contrast and density

---

## V6 — Pawpost Corkboard

**Route:** `/game/v6-corkboard`
**Concept:** Physical corkboard with pinned cards and sticky notes
**File:** `app/game/v6-corkboard/page.tsx`

### Design

A realistic corkboard with tacked index cards, colored sticky notes, pushpins, tape, coffee stains, and red string connections. Each game element is a "physical" object pinned to the board.

### Key Elements

- **Seeded pseudo-random** function for deterministic card rotations and positions
- **Sticky note colors**: Yellow (`#fff9a8`), Pink (`#ffb8c6`), Blue (`#b8d4ff`), Green (`#c8f0b8`), Orange (`#ffd4a0`), Lavender (`#d8c4f0`)
- **Pushpin colors**: Red, Blue, Green, Yellow — rendered as CSS circles
- **Tape strips** with semi-transparent beige background
- **Coffee ring stains** as decorative elements
- **Red string** connecting related items
- **Card shadow** and slight rotation for organic feel
- **CSS animations**: gentle sway, pin bounce, fade-in

### Palette

- Board: `#c4956a` (cork), Cards: `#fffef7` (white), Ink: `#2c1810` (dark brown)

### Recreation Notes

- Cards have randomized slight rotation (±2deg) for organic placement
- CSS custom properties `--sway-start` and `--sway-end` for per-card animation
- Shadow: `rgba(0, 0, 0, 0.25)` for card lift
- No Radix components — all custom

---

## V7 — Neko Shinkansen (Train Station)

**Route:** `/game/v7-station`
**Concept:** Japanese train station departure board
**File:** `app/game/v7-station/page.tsx`

### Design

A Marantz-era Japanese train station departure board. Dark navy background, amber flip-flap display segments, platform numbers, and train line color coding.

### Key Elements

- **Train line colors by specialization**: Hunter → Chuo Rapid (orange), Architect → Sobu Line (blue), Ritualist → Saikyo Line (teal)
- **Job destinations**: supply_food → "Sakana Market" (Shokuryou line), supply_water → "Mizu Springs" (Suido line), hunt_expedition → "The Wild Frontier" (Shinkansen), ritual → "Mystic Shrine" (Meishin Express)
- **Track numbers** for each job (1-8)
- **Amber LED text** for display readouts
- **Flip-flap segment** styling (dark panels with amber text)
- **Station name**: "Neko Shinkansen"
- **Platform/track visual cues**
- **Yamanote green, Chuo orange, Keihin red, Saikyo green** — actual JR line colors

### Palette

- Background: `#0a0e1a`, Board: `#111827`, Amber: `#ff9f0a`, Green: `#34d399`, Red: `#ef4444`, Cyan: `#22d3ee`

### Recreation Notes

- Multiple Japanese rail line colors as constants
- Flap-display aesthetic for text readouts
- Station-departure-board grid layout

---

## V8 — Colony OS 7 (Classic Mac)

**Route:** `/game/v8-colony-os`
**Concept:** Macintosh System 7 desktop (1991)
**File:** `app/game/v8-colony-os/page.tsx`

### Design

A pixel-perfect recreation of the 1991 Macintosh System 7 Finder. Classic purple-blue desktop, draggable windows with title bars, menu bar, desktop icons, Chicago font approximation.

### Key Elements

- **System 7 UI chrome**: Title bars with horizontal stripes, close/zoom boxes, scrollbars
- **Font approximation**: Chicago → bold small sans-serif (Inter bold 12px), Geneva → Inter regular 12px, Monaco → JetBrains Mono 11px
- **Desktop pattern**: Purple-blue (`#6666CC`) with pattern alternate (`#5555AA`)
- **Raised/sunken borders**: Classic Mac 3D beveled edges (white highlight top-left, dark shadow bottom-right)
- **Standard Mac gray** (`#C0C0C0`), selection highlight (`#000080`)
- **Job labels**: Plain names (Supply Food, Plan Hunt, etc.)
- **Status**: Thriving (+), Struggling (!), Dead (X), Starting (~)
- **Blinking cursor** animation at 1Hz

### Palette

- Desktop: `#6666CC`, White: `#FFFFFF`, Gray: `#C0C0C0`, Selection: `#000080`

### Recreation Notes

- Extensive border helpers: `raised`, `sunken`, `deepRaised`, `deepSunken` objects
- Windows with drag handles and title bar stripes
- Desktop icon grid for quick actions
- Authentic System 7 window management feel

---

## V9 — The Catford Examiner (Newspaper)

See **detailed section above** — this is the selected production concept.

---

## V10 — Whisker Radio (Hi-Fi Stereo)

**Route:** `/game/v10-radio`
**Concept:** 1970s hi-fi stereo receiver (Marantz/Pioneer/Sansui)
**File:** `app/game/v10-radio/page.tsx`

### Design

A warm, amber-glowing stereo receiver faceplate. Walnut wood surround, brushed metal panels, chrome knobs, VU meters, and AM/FM-style tuning displays.

### Key Elements

- **VU meters** for resource levels (needle-style gauges)
- **LCD displays** with amber text on dark backgrounds
- **Chrome highlights** and brushed metal textures
- **Walnut wood surround** (`#1a1008` deep brown border)
- **Job names as radio programs**: supply_food → "Forage Broadcast", supply_water → "Spring Relay", ritual → "Midnight Ritual FM"
- **Status as LED indicators**: thriving → green LED, struggling → amber, dead → red
- **Knob-style controls**
- **Specialization styling**: Hunter → green, Architect → gold, Ritualist → purple

### Palette

- Walnut: `#1a1008`, Faceplate: `#1c1c1c`, Chrome: `#c0c0c0`, Amber: `#F5A623`, Green: `#33cc33`

### Recreation Notes

- VU meter needle animation
- Gradients for chrome/brushed metal surface
- Warm amber glow effects (`text-shadow`)

---

## V11 — Cat Tarot

**Route:** `/game/v11-tarot`
**Concept:** Mystical tarot card reader / fortune teller
**File:** `app/game/v11-tarot/page.tsx`

### Design

A deep purple/indigo mystical interface with gold accents, tarot card motifs, and fortune-telling language. Smoky atmospheric backgrounds.

### Key Elements

- **Mystical prefixes** for events: "The spirits reveal...", "The cards whisper...", "The stars foretell...", "The crystal shows..."
- **Tarot-themed job names**: supply_food → "Offering of Sustenance", supply_water → "Rite of Waters", ritual → "Cosmic Ritual"
- **Roman numeral arcana**: supply_food → III, supply_water → IV, hunt_expedition → VIII, ritual → XXI
- **Specialization as archetypes**: Hunter → "The Hunter" (⚔), Architect → "The Architect" (∧), Ritualist → "The Mystic" (☄)
- **Atmospheric smoke** effects: `rgba(180, 160, 220, 0.06)` overlay
- **Gold accents**: `#D4AF37` (antique gold), `#FFD700` (bright gold)

### Palette

- Background: `#1a0a2e` (deep purple), Card: `#1f1035`, Gold: `#D4AF37`, Purple: `#6b3fa0`, Red: `#e04060`

### Recreation Notes

- Heavy use of gradients for mystical depth
- Card-flip or reveal animations
- Atmospheric particle effects possible
- Smoke/fog overlay layers

---

## V12 — Catquarium (Deep Sea)

**Route:** `/game/v12-aquarium`
**Concept:** Submarine / deep-sea research station monitoring system
**File:** `app/game/v12-aquarium/page.tsx`

### Design

A deep ocean research station interface with bioluminescent accents, steel/rivet industrial panels, and submarine operations language.

### Key Elements

- **Submarine ops labels**: supply_food → "ORGANIC RESUPPLY", supply_water → "H2O EXTRACTION", hunt_expedition → "DEEP-SEA HUNT", build_house → "HULL CONSTRUCTION", ritual → "BIOLUMINESCENCE RITUAL"
- **Status display**: thriving → "NOMINAL" (phosphor green), struggling → "CRITICAL" (red), starting → "INITIALIZING" (cyan)
- **Color accents**: Phosphor green (`#00ff41`), Cyan (`#00e5ff`), Bioluminescent purple (`#7b68ee`), Coral (`#ff6b6b`)
- **Steel panel textures**: Dark blue-gray with rivet details
- **Glass panel effect**: Semi-transparent overlays with subtle borders
- **Specialization colors**: Hunter → coral, Architect → cyan, Ritualist → bioluminescent purple

### Palette

- Abyss: `#0a1628`, Deep: `#0d2847`, Phosphor: `#00ff41`, Cyan: `#00e5ff`, Bio: `#7b68ee`, Coral: `#ff6b6b`

### Recreation Notes

- `rgba` glass panels with backdrop effect
- Glow effects on status indicators
- Steel/rivet industrial border details
- Depth gradient backgrounds (darker at bottom)

---

## V13 — Meow Mail (Win98 Email Client)

**Route:** `/game/v13-mail`
**Concept:** Windows 98 Outlook Express email client
**File:** `app/game/v13-mail/page.tsx`

### Design

A faithful recreation of the Windows 98 Outlook Express interface. Classic silver/gray chrome, 3D beveled buttons, folder tree, message list with read/unread states, status bar.

### Key Elements

- **Win98 chrome**: Raised/sunken borders using highlight/shadow pairs
- **Folder tree**: Inbox, Outbox, Sent, Drafts, Contacts — game sections mapped to email folders
- **3D button helpers**: `raised`, `sunken`, `deepRaised`, `deepSunken` border objects
- **Win98 palette**: Background `#C0C0C0`, Navy `#000080`, Teal `#008080`, Window `#FFFFFF`
- **Tooltip background**: `#FFFFE1` (classic yellow)
- **Status bar** at the bottom with connection status
- **Folder yellow**: `#FFD700` for folder icons
- **Font**: Tahoma/MS Sans Serif approximation via Inter

### Palette

- Background: `#C0C0C0`, Navy: `#000080`, Teal: `#008080`, White: `#FFFFFF`, Button face: `#C0C0C0`

### Recreation Notes

- `deepRaised`/`deepSunken` border helpers for authentic Win98 3D effect
- Folder tree + message list + preview pane three-panel layout
- Read/unread state tracking for messages
- Info bar: `#ECE9D8`

---

## Shared Data Interface

All variants consume the same hook:

```typescript
import { useGameDashboard, formatDuration } from "@/hooks/useGameDashboard";
import { summarizeCatIdentity } from "@/lib/game/catTraits";
```

### `useGameDashboard()` returns:

| Field                | Type    | Description                                                                                                     |
| -------------------- | ------- | --------------------------------------------------------------------------------------------------------------- |
| `dashboard`          | object  | Raw dashboard query result                                                                                      |
| `colony`             | object  | `{ name, status, runNumber, resources: { food, water, herbs, materials }, globalUpgradePoints, testTimeScale }` |
| `jobs`               | array   | `{ _id, kind, status, startedAt, endsAt, assignedCatName, totalClicks }`                                        |
| `upgrades`           | array   | `{ _id, key, baseCost, level, maxLevel }`                                                                       |
| `events`             | array   | `{ _id, timestamp, message }`                                                                                   |
| `cats`               | array   | `{ _id, name, specialization, spriteParams, roleXp, stats, deathTime }`                                         |
| `ritualPoints`       | number  | Available ritual points for upgrades                                                                            |
| `accelerationPreset` | string  | Current test acceleration mode                                                                                  |
| `statusTone`         | string  | Colony status tone                                                                                              |
| `now`                | number  | Current timestamp (updates every second)                                                                        |
| `sessionId`          | string  | Session identifier                                                                                              |
| `nickname`           | string  | Player nickname                                                                                                 |
| `showTestControls`   | boolean | Whether test controls are visible                                                                               |
| `busyAction`         | string  | Currently processing action                                                                                     |
| `error`              | string  | Error message if any                                                                                            |
| `leader`             | object  | Leader cat (or null)                                                                                            |
| `onlineCount`        | number  | Number of online players                                                                                        |

### Actions:

| Function                    | Description                                                                               |
| --------------------------- | ----------------------------------------------------------------------------------------- |
| `submitJob(kind)`           | Submit a new job (supply_food, supply_water, leader_plan_hunt, leader_plan_house, ritual) |
| `onBoostJob(jobId)`         | Click-boost an active job                                                                 |
| `onBuyUpgrade(upgradeId)`   | Purchase an upgrade with ritual points                                                    |
| `onSetAcceleration(preset)` | Set test acceleration (off, fast, turbo)                                                  |
| `updateNickname(name)`      | Update player nickname                                                                    |
| `ensureGlobalState({})`     | Initialize/ensure global state exists                                                     |

### `summarizeCatIdentity(spriteParams)`:

Returns a human-readable string describing the cat's appearance based on its sprite parameters (fur color, pattern, eye color, etc.).

---

## Screenshots

Screenshots of all variants were captured during development and are stored in the project root:

- `v1-terminal.png` through `v13-mail.png` — Initial captures of each variant
- `v9-newspaper-redesign.png` — V9 after section restructuring
- `v9-page-turn-*.png` — Page turn animation iterations
- `v9-sweep-*.png` — Final GPU-composited sweep animation
