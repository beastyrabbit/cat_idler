# P18 — Visual polish: DF-Steam UI overhaul + workshop craft-station sprites

> **Living target spec; compact-presentation gate verified.** Label-free roofed homes,
> distinct open stations, and the Adventure 9-patch/button/progress/cursor foundation have
> accepted native and optimized-WASM evidence at 1024×768, 1280×800, and 1920×1080. The maintained
> presentation now uses a four-store survival HUD, a complete 32-resource Stores menu, and one
> expanded category in the compact command dock rather than an exhaustive resource/button wall.
> Research is presented as one single-root, left-to-right dependency tree on a dark
> cartographer's worktable, with high-contrast paper studies, category accents, actionable state
> colors, a complete highlighted ancestor path, and a plain paper inspector. The compact layout is
> derived from prerequisites instead of category coordinate strips, and its canvas supports direct
> accelerated left- or middle-drag navigation.
> The maintained surface, scrolling, scaling, and navigation rules live in
> [`../../UI_ARCHITECTURE.md`](../../UI_ARCHITECTURE.md).
> That redesign passes focused layout tests, generalized narrow/wide native frame inspection, and
> the shared WASM compile gate. The accepted
> Accounting Tent and staged wall/agricultural sequence remain valid prior world-composition
> evidence in
> [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

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
  - **woodworking** (→wooden tools/wood goods): workbench + tools (hammer/saw) + planks.
  - **smithy**: anvil + forge fire (keep/improve the current forge). **mill**: windmill/millstone.
  - **clothier**: loom / striped cloth. Keep them a cohesive 3×3 footprint (P16).
- Verify by montage; each must read as its craft. If a composite looks bad, note it.

## UI overhaul — DF-Steam look
The pre-P18 UI used plain dark-green text boxes. DF-Steam UI = polished, themed: wood/parchment **framed
panels**, restrained borders, clear screen buttons, banners/headers, and styled controls. The
Adventure skin remains the visual foundation; decoration does not duplicate text.
- **Asset foundation exists** (catalogued in `docs/assets/items_ui.md`): **UI Pack – Adventure** —
  wood-frame + cream-parchment **9-patch panels** (border-image), wood/red/grey **buttons**,
  colored **progress pills** (food/water/threat), hanging **banner** headers, round medallions for
  resource icons; plus **Board Game Icons** (recolorable white glyphs) for
  resources, and cursors. Copy the chosen ones into `public/images/game/ui/` + `.../icons/`.
- **Maintained compact presentation (supersedes the exhaustive P18 HUD/toolbar layout):** the
  top-left world card uses text-only Food, Water, Materials, and Medicine values plus colony status.
  The Stores screen owns the semantic icon grid for all 32 protocol resources. The bottom dock
  shows primary categories and expands
  only Gather, Build, Territory, Scout, Village, or contextual controls for the active tool. The
  officers/ledger panels, inspectors (hover tooltip + big menu, P15), Log, and research screen
  retain the same Adventure treatment. Bevy UI uses 9-patch `ImageNode` slicing and crisp pixel
  filtering. Generalized captures wait several stable render frames after opening a menu or changing
  tools so transient state-transition frames are not mistaken for persistent rendering defects.
- The implementation sequence remains useful history (panel/9-patch framework → HUD →
  toolbar/menus → inspectors). The integrated generalized frames and interaction campaign close
  the maintained presentation; focused component captures alone did not close it.

## Also here (from P15)
- The square top-down sharpened-timber palisade and current top-down cat sheet are accepted runtime
  art. Either may be revisited as optional visual polish, not as a release blocker.
