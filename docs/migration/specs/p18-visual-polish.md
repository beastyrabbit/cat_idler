# P18 — Visual polish: DF-Steam UI overhaul + workshop craft-station sprites

User (2026-07-10): (1) workshops should look like DF-Steam craft-stations — find a good asset or
compose one; (2) the whole UI needs an overhaul, big inspiration from how DF Steam looks now.

## Workshop sprites — DF-Steam craft-stations
DF-Steam workshops are distinctive top-down **craft stations** where the function reads at a
glance: smithy = anvil + forge fire, carpenter = saw + planks + workbench, mason = stone-cutting
table + blocks, weaver = loom, mill = millstone/sails, still = barrels. Ours are generic.
- **Library is thin on 2D craft props** (found only a 3D `Survival Kit/workbench-anvil` + windmills;
  Tiny Town has tools tiles 115–128 (pickaxe/axe/hammer/rake/shovel), barrels/crates/haystack;
  Roguelike Base has barrels/anvil-ish/tools). So **compose** ("gen") craft-showing workshops:
  a building base (Tiny Town / Roguelike) + a distinct craft indicator per type:
  - **wood-cutting** (logs→planks): saw / axe + a log pile / sawhorse.
  - **stone-prep** (stone→blocks): stone blocks + chisel/pick + a cutting table.
  - **woodworking** (→tools/weapons): workbench + tools (hammer/saw) + planks.
  - **smithy**: anvil + forge fire (keep/improve the current forge). **mill**: windmill/millstone.
  - **clothier**: loom / striped cloth. Keep them a cohesive 3×3 footprint (P16).
- Verify by montage; each must read as its craft. If a composite looks bad, note it.

## UI overhaul — DF-Steam look
Current UI = plain dark-green text boxes. DF-Steam UI = polished, themed: wood/parchment **framed
panels**, ornate borders, **icon-driven** readouts, clear tabbed menus, banners/headers, resource
**pills with icons**, styled buttons. Restyle the client HUD/panels to that.
- **Asset foundation exists** (catalogued in `docs/assets/items_ui.md`): **UI Pack – Adventure** —
  wood-frame + cream-parchment **9-patch panels** (border-image), wood/red/grey **buttons**,
  colored **progress pills** (food/water/threat), hanging **banner** headers, round medallions for
  resource icons, minimap ring; plus **Board Game Icons** (recolorable white glyphs) for
  resources, and cursors. Copy the chosen ones into `public/images/game/ui/` + `.../icons/`.
- Apply across: the top-left **HUD** (resource pills+icons, status, threat band), the **toolbar**
  (styled buttons), the **officers/ledger** panels, the **inspectors** (hover tooltip + big menu,
  P15), the **event log**, and the upgrade/menus. Bevy UI: 9-patch via `ImageNode` slicing;
  keep it theme-cohesive with the pixel world (crisp, not blurry).
- Big card — do as a focused client pass (likely several commits: panel/9-patch framework →
  HUD → toolbar/menus → inspectors), after the P14.5 render + movement land. Keep it readable +
  performant.

## Also here (from P15)
- **Better wall asset** (palisade could read nicer) and a cleaner top-down cat search may be
  folded into future asset polish; the current P&W sheet remains the selected runtime art.
