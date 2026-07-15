# Items, Resources & UI — Curated Kenney Catalog

Curated asset picks for the **item / resource / goods icons** (stockpiles + dashboard) and
the **UI kit** (panels, buttons, bars, cursors) for *Idle Cat Forest*.

All sources live under the gitignored pack `public/Kenney Game Assets All-in-1 3.5.0/`.
Chosen files get **copied** into tracked folders:

- Icons  → `public/images/game/icons/`
- UI     → `public/images/game/ui/`
- Cursors → `public/images/game/ui/cursor/`

The Bevy client uses the tracked semantic sprites under these directories; the old lettered
resource images are not the maintained HUD source. The exact resource-to-image mapping lives in
`resource_icon_path` in `crates/cat-client/src/lib.rs`.

---

## Maintained runtime selection

- **ONE icon set:** **Board Game Icons** (`Icons/Board Game Icons`). Clean monochrome-white
  glyphs, 64px + 128px, and it literally ships named `resource_*` and `structure_*` glyphs.
  The selected glyphs were copied into semantic tracked PNGs and are tinted by Bevy where needed.
  This is the cohesive DF-Steam-readable core.
- **ONE colored accent set:** **Fish Pack** (`2D assets/Fish Pack`) for the cat-food fish and
  seaweed = catnip/herbs — flat vector, same visual language, and adorable for a cat game.
- **ONE UI kit:** **UI Pack - Adventure** (`UI assets/UI Pack - Adventure`). Warm wood-frame +
  cream-parchment 9-patch panels, wood/red buttons, colored progress pills, banners, round
  medallions, compass. Cozy-forest tone, cohesive with the icons.
- **Cursors:** **Cursor Pack** (`UI assets/Cursor Pack`) — white+outline pointer, grab hand,
  target reticle, and a **paw-print** cursor.

Why Board Game Icons over the colored "Generic Items": the monochrome glyph system reads far
better at 16–32px in a HUD, recolors to any resource, and stays visually unified. Generic
Items (`2D assets/Generic Items`, 329 colored PNGs) is a good **fallback** if a richer colored
tool/goods icon is ever wanted, but its files are numbered (`genericItem_color_104.png`) with
no semantic names, so mapping is manual.

---

## Resource / goods art — live semantic copies

The tracked semantic-art mapping contains one unique PNG for every maintained HUD row. Most live
under `game/icons/`:
`armor`, `blessings`, `blocks`, `bone`, `catnip`, `cloth`, `fish`, `flour`, `food`,
`grain`, `herbs`, `hide`, `leather`, `logs`, `lumber`, `materials`, `metal`, `ore`, `planks`,
`refined`, `stone`, `tools`, `water`, and `weapons`. `goods` remains a separate fallback for
finite crafted-item kinds that do not yet have their own glyph. The Goods panel also exposes
each finite item's physical weight, condition range, damaged/broken counts, and a repair action
when a valid staffed workshop and matching visible material are available.
Raw Fibre intentionally maps to the tracked public-pack `game/props/haystack.png`, whose bundled
plant silhouette stays distinct from both the Flour pouch and finished Cloth.

Source dir: `public/Kenney Game Assets All-in-1 3.5.0/Icons/Board Game Icons/PNG/`
Two sizes: `Default (64px)/` and `Double (128px)/`. Ship **128px** (crisp, downscales clean).
Vector recolor source: `Icons/Board Game Icons/Vector/Icons/`.

| Game resource / good | Source file (128px) | Verdict |
|---|---|---|
| **Food** | `resource_apple.png` | Apple; universal food glyph. Tint warm red/green. |
| **Water** | `flask_full.png` | Filled flask; tint blue. (No literal droplet in this pack — flask reads as "liquid store"; alt: recolor a droplet from Game Icons.) |
| **Herbs / catnip** | Fish Pack `background_seaweed_a.png` (see below) | Leafy sprig; tint green. Board Game has no herb glyph. |
| **Grain / wheat** | `resource_wheat.png` | Two wheat stalks. Exact match. |
| **Flour** | `pouch.png` | Cinched sack = flour/meal. Tint pale cream. |
| **Wood / logs** | `resource_wood.png` | Rolled log. |
| **Lumber (worked wood)** | `resource_lumber.png` | Bundled timber; use if you split raw vs worked wood. |
| **Stone** | `resource_iron.png` | Ingot-in-hexagon; tint grey for stone / steel-blue for metal. |
| **Bone** | Fish Pack `fish_blue_skeleton.png` | Clear 128 px skeleton silhouette; tint warm ivory in the HUD. |
| **Materials (generic)** | `token.png` | Neutral disc token; the catch-all goods glyph. |
| **Refined goods** | `resource_planks.png` | Stacked planks/bricks = processed output. |
| **Cloth** | `pouch.png` (alt tint) or Generic Items bolt | Finished textile; the tracked Cloth glyph remains distinct from raw Fibre. |
| **Fibre** | tracked `game/props/haystack.png` | Public-pack plant bundle with a unique silhouette; used directly so Fibre never aliases the Flour pouch. |
| **Weapons** | `sword.png` (or `bow.png`) | Sword primary, bow for ranged. |
| **Armour** | `shield.png` | Shield. |
| **Tools** | Generic Items `genericItem_color_*` (hammer/axe/saw) | Board Game has no tool glyph; pull one colored tool from Generic Items, or reuse `sword` as "equipment". |
| **Blessings / ritual** | `fire.png` or `flask_full.png`; **Rune Pack** for a mystical variant | Flame = ritual/offering. For a carved-rune blessing token see Rune Pack note below. |
| **Research points** | `book_open.png` | Open book = knowledge/research. |
| **Time / duration (HUD)** | `hourglass.png` | For job timers / cooldowns. |
| **Population / hearts** | `suit_hearts.png` | Health/affection. |
| **Threat / death** | `skull.png` | Raid/mortality band. |
| **Leadership / crown** | `crown_a.png` | Leader/election glyph. |

**Structure glyphs** (same pack, for building buttons / map legend / newspaper):
`structure_house.png`, `structure_farm.png`, `structure_church.png` (shrine),
`structure_wall.png`, `structure_gate.png`, `structure_tower.png`, `structure_watchtower.png`.

**Ritual / blessing flavor option — Rune Pack** (`2D assets/Rune Pack`, 667 PNGs, Blue/Grey/Black):
carved-stone rune slabs. One rune (e.g. a Grey slab) makes a distinctive "blessing token" if
the flat `fire`/`flask` glyph feels too plain. Heavier than a glyph — use sparingly, e.g. the
blessings currency chip only.

---

## Cat carry-glyphs (what a cat visibly hauls) — verified

Every physical cargo overlay reuses the exact tracked semantic resource PNG named above rather
than a second carry-only vocabulary. This covers Food, Fish, Water, Materials, Stone, Refined,
Blessings, Logs, Lumber, Planks, Blocks, Tools, Weapons, Armor, Catnip, Grain, Flour, Herbs, Fibre,
Hide, Bone, Cloth, Leather, Ore, and Metal. Fibre deliberately uses the tracked public-pack haystack prop as
its raw-plant bundle while Cloth keeps its finished-textile icon; both therefore remain distinct
from the Flour pouch. Their existing resource tints remain legible at the on-map overlay size, while icon shape
is authoritative: Lumber and Planks, for example, keep separate symbols. The runtime has no
colored-square, terrain, farm, or furniture fallback for cargo.

The exhaustive mapping/file/identity test and the inspected client-owned
`/tmp/semantic-cargo-icons-1024.png` frame verify ten simultaneous representative loads at the
supported 1024×768 lower bound.

---

## UI kit — UI Pack - Adventure

Source dir: `public/Kenney Game Assets All-in-1 3.5.0/UI assets/UI Pack - Adventure/PNG/Double/`
Wood-frame + cream-parchment aesthetic. The selected panels are integrated through Bevy sliced
images. Semantic tracked copies, not the ignored source pack, are the runtime assets.

| UI need | Source file | Verdict |
|---|---|---|
| **Primary panel / frame** | `panel_brown.png` | Wood frame, parchment fill. The main dashboard/HUD panel. Use as `border-image` (9-slice). |
| **Panel — ornate corners** | `panel_border_brown.png` / `panel_brown_corners_a.png` | Metal-corner variant for headers / featured cards. |
| **Panel — dark/inset** | `panel_brown_dark.png` | Recessed sub-panel (e.g. inventory well, stockpile grid background). |
| **Grey/stone panel** | `panel_grey.png` | Neutral variant if brown is too warm for a given screen. |
| **Button (default)** | `button_brown.png` | Wood button, idle state. |
| **Button (primary/CTA)** | `button_red.png` | Red for confirm/boost/attack actions. |
| **Button (secondary)** | `button_grey.png` | Neutral/disabled-ish. |
| **Button hover** | reuse `button_grey`↔`button_brown` swap, or Bevy tint | Pack has no explicit hover frame; use the tracked interactive-state sprites/tints. |
| **Close button** | `button_brown_close.png` / `button_red_close.png` | Pre-baked ✕ button for modals. |
| **Progress / resource bar** | `progress_green_border.png`, `progress_red_border.png`, `progress_blue_border.png`, `progress_white_border.png` | Colored pill bars — green=food/health, blue=water, red=threat, white=neutral. `_small` variants for compact rows. Bordered versions read best. |
| **Progress track (empty)** | `progress_transparent.png` | Empty groove behind a fill. |
| **Banner / header ribbon** | `banner_hanging.png` (also `banner_modern.png`, `banner_classic_curtain.png`) | Red hanging ribbon for section titles ("Colony", "Stockpile"). |
| **Round medallion / icon slot** | `round_brown.png` | Circular frame to seat a resource glyph (bar-icon chip). `round_grey.png` neutral. |
| **Hexagon slot** | `hexagon_brown.png` | Alt icon frame (tech-tree node?). |
| **Checkbox / toggle** | `checkbox_brown_empty.png` / `checkbox_brown_checked.png` / `checkbox_grey_cross.png` | Vote/option toggles. |
| **Scrollbar** | `scrollbar_brown.png` / `scrollbar_brown_small.png` | Event-log / list scrollbars. |
| **Minimap ring + compass** | `minimap_ring_brown.png`, `minimap_compass_toon_n/e/s/w.png` | Frame the map minimap; compass rose. |
| **Minimap markers** | `minimap_icon_star_red/yellow.png`, `minimap_icon_exclamation_red.png` | Alerts/points of interest. |

**Resource-bar pattern:** `round_brown.png` medallion + a tinted Board Game glyph inside +
a `progress_*_border.png` pill next to it = one cohesive resource row.

**Ornate accent (optional):** **Fantasy UI Borders** (`UI assets/Fantasy UI Borders`) —
white/blue decorative corner frames (9-patch). Nice for a single "hero" panel (e.g. the
leader portrait or a ritual modal), but UI Pack - Adventure already covers the workhorse needs;
don't mix both broadly.

---

## Cursors — Cursor Pack

Source dir: `public/Kenney Game Assets All-in-1 3.5.0/UI assets/Cursor Pack/PNG/Outline/Default/`
White fill + dark outline (readable over any map tile).

| Cursor | Source file | Use |
|---|---|---|
| **Default pointer** | `pointer_b_shaded.png` (or flat `pointer_b.png`) | Base cursor. |
| **Hand — point / interact** | `hand_point.png` | Hover on clickable (cat, building). |
| **Hand — grab / drag** | `hand_closed.png` (open: `hand_small_open.png`) | Zone-painting drag. |
| **Selection / target** | `target_a.png` / `target_round_a.png` | Boost-click / defense-click reticle. |
| **Paw print** | search `paw` in pack (`Vector/`), or footprints `hand_point_*` | On-brand cat cursor / "assign cat here". |
| **Zoom** | `zoom_out.png` (+ zoom_in) | Map zoom affordance. |

---

## Tracked runtime copies

```
public/images/game/icons/     # Board Game Icons 128px + a few Fish Pack fish
public/images/game/ui/         # Semantic UI Pack - Adventure Double PNGs
public/images/game/ui/cursor/  # Semantic Cursor Pack Outline/Default 32px PNGs
```

The Bevy client uses `panel.png`, `panel-dark.png`, and `panel-ornate.png` as sliced panels;
`button.png`, `button-active.png`, and `button-disabled.png` for interaction states;
`progress-track.png`, `progress-good.png`, `progress-mid.png`, and `progress-low.png` for need
bars; plus `banner.png`, `icon-frame.png`, and `minimap-ring.png`. The cursor directory tracks
`pointer.png`, `interact.png`, `pressed.png`, `target.png`, and `disabled.png`.

Implemented semantic copies are listed above and committed under `public/images/game/icons/`.
The source-name table remains useful provenance for future additions, but new runtime mappings
must be added explicitly to the Bevy asset table and verified in the client's own framebuffer.
